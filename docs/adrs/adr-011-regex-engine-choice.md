# ADR-011: Regex Engine for Buffer Pattern Matching

## Status

Proposed

## Context

The regex wizard feature requires compiling user-supplied regex patterns and finding all matches across the entire buffer. The existing `find_forward` and `find_backward` functions in `buffer.rs` use literal substring matching (`str::find`), which does not support regex syntax.

**Requirements**:
- Compile arbitrary user-supplied regex patterns
- Find all non-overlapping matches across all buffer lines
- Return match positions (line, start_col, end_col) for highlight rendering
- Handle invalid patterns gracefully (compile error, not panic/crash)
- Must not freeze the UI on pathological patterns (single-threaded editor, ADR-003)

**Constraints**:
- Open source with permissive license (project convention)
- Rust ecosystem (no FFI complexity unless justified)
- Pattern matching exposed as a generic bridge primitive (not wizard-specific)
- Functional-core: the matching function is pure (pattern + buffer -> results)

## Decision

Use the Rust **`regex`** crate (version 1.x, MIT + Apache-2.0 dual license) as the regex engine. Add it as a dependency to `alfred-lisp` (where bridge primitives compile and execute regex against buffer content).

The `regex` crate compiles patterns into a finite automaton that guarantees **linear-time matching** with respect to input size. There is no backtracking, so pathological patterns like `(a+)+` cannot cause exponential blowup. This safety guarantee is critical for a single-threaded editor where any blocking operation freezes the UI.

The bridge primitive `regex-find-all` will:
1. Accept a pattern string from Lisp
2. Attempt compilation via `Regex::new` -- return 0 with error message on failure
3. Iterate buffer lines, calling `find_iter` per line to collect all match positions
4. Store results in `EditorState.match_highlights` for renderer consumption
5. Return the total match count

A companion `regex-valid?` primitive enables pattern validation without side effects (no highlight mutation), useful for the wizard to check pattern validity before triggering a full buffer scan.

## Alternatives Considered

### Alternative 1: Pure Lisp Regex Implementation

**Description**: Implement a regex engine entirely in Alfred Lisp using the existing `rust_lisp` interpreter. Pattern matching would be built from string primitives (`str-contains`, `str-substring`, character iteration).

**Evaluation**:
- (+) Zero Rust dependency additions
- (+) Full plugin-first purity -- no new bridge primitives
- (-) `rust_lisp` has no character-level iteration primitive -- would need new bridge functions anyway
- (-) Performance: interpreted Lisp is 100-1000x slower than compiled Rust regex for pattern matching. On a 10,000-line buffer, matching would take seconds, freezing the UI
- (-) Correctness: implementing a correct regex engine is a massive undertaking (Unicode support, character classes, quantifiers, grouping, alternation). The `regex` crate represents years of engineering
- (-) No linear-time guarantee -- a naive NFA/backtracking implementation in Lisp could hang on pathological inputs
- (-) Maintenance burden: regex engine bugs would be Alfred's responsibility

**Rejection rationale**: Performance and correctness are disqualifying. Building a regex engine in interpreted Lisp is not viable for real-time interactive highlighting.

### Alternative 2: PCRE2 Bindings via `pcre2` Crate

**Description**: Use the `pcre2` crate which provides Rust bindings to the PCRE2 C library. PCRE2 supports Perl-compatible regex syntax including lookahead, lookbehind, backreferences, and recursive patterns.

**Evaluation**:
- (+) Richer syntax than Rust `regex` (backreferences, recursive patterns, `\K`, etc.)
- (+) Well-maintained C library (PCRE2 11.x, BSD license)
- (+) The `pcre2` Rust crate (MIT license) provides safe wrappers
- (-) Requires system C library (PCRE2) as build dependency -- complicates cross-compilation and CI
- (-) PCRE2 uses backtracking, meaning pathological patterns CAN cause exponential blowup. In a single-threaded TUI editor, this would freeze the UI with no way to cancel
- (-) JIT compilation adds complexity and platform-specific behavior
- (-) The extra features (backreferences, recursion) are rarely needed for an interactive regex builder wizard -- the wizard categories cover standard regex components that `regex` handles fully
- (-) Additional FFI layer increases binary size and potential for memory safety issues at the boundary

**Rejection rationale**: The backtracking risk is the primary disqualifier. A user entering `(a+)+b` against a buffer with many `a` characters could freeze the editor indefinitely. The richer syntax does not justify this risk for an interactive highlighting feature. If PCRE2 features are ever needed, they can be added as an opt-in alternative engine behind a flag.

### Alternative 3: `fancy-regex` Crate

**Description**: The `fancy-regex` crate extends Rust `regex` with backtracking features (backreferences, lookaround) while delegating non-backtracking patterns to the `regex` crate's automaton engine.

**Evaluation**:
- (+) Superset of `regex` syntax (backreferences, lookaround)
- (+) Pure Rust, MIT license, no C dependencies
- (+) Delegates to `regex` crate for patterns that do not need backtracking -- gets linear-time guarantee for most patterns
- (-) Backreference patterns still use backtracking with potential exponential behavior
- (-) Smaller community (~500 GitHub stars vs 3000+ for `regex`)
- (-) Additional dependency layer (depends on `regex` internally)
- (-) The wizard's category-based design does not include backreference components, so the extra capability is unused

**Rejection rationale**: Adds backtracking risk for features the wizard does not expose. If backreferences are needed later, `fancy-regex` can replace `regex` as a drop-in upgrade.

## Consequences

### Positive
- Linear-time matching guarantee eliminates UI freeze risk from pathological patterns
- Zero-cost: `regex` is the most widely used Rust crate, extremely well-maintained (5000+ GitHub stars, weekly releases)
- Pure Rust: no C dependencies, no FFI, simple cross-compilation
- MIT + Apache-2.0 dual license: maximally permissive
- Fast compilation: typical patterns compile in <1ms
- Unicode-aware by default
- Well-documented API with clear error types for invalid patterns

### Negative
- No backreferences (`\1`), no lookaround (`(?<=...)`, `(?<!...)`), no recursive patterns
- Adds one new dependency to `alfred-lisp/Cargo.toml` (but `regex` is a leaf crate with minimal transitive deps)
- Pattern syntax differs slightly from PCRE (users expecting Perl-style regex may notice missing features)

### Neutral
- The `regex` crate's syntax is documented at https://docs.rs/regex/latest/regex/#syntax -- the wizard's category labels should match this syntax, not PCRE
- Future upgrade path: swap `regex` for `fancy-regex` if backreference support is requested (API is compatible)
