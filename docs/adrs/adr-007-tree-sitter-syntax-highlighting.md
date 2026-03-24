# ADR-007: Tree-sitter for Syntax Highlighting

## Status

Accepted

## Context

Alfred needs syntax highlighting to be a usable code editor. The feature must integrate with the existing `line_styles` system on `EditorState`, respect `alfred-core` purity (no I/O, no tree-sitter dependency in core), and support incremental re-parsing as the user edits, since full re-parse on every keystroke would be prohibitive for large files.

Two broad approaches exist: regex-based pattern matching (used by early editors) and concrete syntax tree parsing (tree-sitter, used by Neovim, Helix, Zed).

## Decision

Use **tree-sitter** via the `tree-sitter` Rust crate with compiled-in grammar crates as Cargo dependencies. Initial languages: Python, Rust, JavaScript.

Tree-sitter is a parser generator that produces incremental, error-tolerant parsers. It builds a concrete syntax tree (CST) on initial parse, then re-parses only the changed region on edits. Highlight queries (S-expression patterns) map CST nodes to named highlight groups (e.g., `@keyword`, `@function`, `@string`).

## Alternatives Considered

### Alternative 1: Regex-based highlighting (TextMate-style grammars)

- **What**: Define regex patterns per language that match token categories. Each pattern maps to a color group. Patterns are applied line-by-line or with limited multi-line lookahead.
- **Expected impact**: Simpler implementation, smaller binary, no grammar compilation. Covers ~70% of highlighting cases accurately.
- **Why rejected**: Regex grammars cannot distinguish context-dependent tokens (e.g., `for` as keyword vs. `for` as identifier in certain contexts). Multi-line constructs (doc strings, block comments, raw strings) are fragile. Maintenance burden is high -- each language requires hand-tuned patterns. Helix, Neovim, and Zed have all moved to tree-sitter, validating that the accuracy and incremental parsing benefits outweigh the complexity.

### Alternative 2: Syntect (Sublime Text syntax definitions)

- **What**: Use the `syntect` crate which implements Sublime Text's syntax highlighting engine (regex-based with stacks and contexts).
- **Expected impact**: Good accuracy for common languages, proven library, moderate binary size. Covers ~85% of cases.
- **Why rejected**: Still fundamentally regex-based -- no true incremental parsing. On each edit, syntect must re-highlight from the start of the changed region (or from an earlier state checkpoint). For large files with edits near the end, this can be expensive. License is MIT (acceptable), but the approach is a dead end as the industry moves toward tree-sitter. Syntect's maintenance has slowed compared to tree-sitter's ecosystem growth.

### Alternative 3: No syntax highlighting (defer indefinitely)

- **What**: Keep plain text rendering, focus on other features.
- **Expected impact**: Zero implementation cost.
- **Why rejected**: Syntax highlighting is a baseline expectation for any code editor. Without it, Alfred cannot be evaluated against alternatives for real editing tasks.

## Consequences

### Positive

- Accurate, context-aware highlighting for all supported languages
- Incremental parsing: only the changed region is re-parsed on edit, enabling <5ms re-parse for 10K-line files
- Error-tolerant: partial/broken code still gets best-effort highlighting
- Extensible: adding a language requires adding a grammar crate dependency and a highlight query file
- Industry standard: same approach as Helix, Neovim (0.5+), Zed
- Highlight queries are declarative S-expressions, easy to maintain

### Negative

- Binary size increases ~200-500KB per compiled grammar
- Tree-sitter dependency adds C compilation to the build (grammar crates include C sources)
- `alfred-syntax` crate adds a 6th crate to the workspace
- Complexity: tree-sitter's edit notification API must be correctly synchronized with ropey's edit operations
- Highlight query files must be sourced and maintained per language
