# ADR-004: Lisp Interpreter Selection -- rust_lisp Over Janet

## Status

Accepted

## Context

Alfred needs an adopted embeddable Lisp interpreter (ADR-001). Two candidates were validated during discovery: Janet (a C-based Lisp) and rust_lisp (a native Rust Lisp). The selection criteria established during discovery are: (1) quality of Rust interop, (2) expression evaluation performance, (3) ease of exposing core primitives, (4) community/maintenance health.

### Evaluation Matrix

| Criterion | Janet | rust_lisp | Weight |
|-----------|-------|-----------|--------|
| **Rust interop quality** | Requires C FFI bridge (cc/bindgen). Every call crosses FFI boundary. Type marshalling between C and Rust | Native Rust. Direct function registration. `Value::Foreign()` wraps Rust types. `lisp!` macro for embedded Lisp | HIGH |
| **Ease of exposing primitives** | Must write C-compatible wrapper functions, then register them via Janet's C API | Register Rust closures directly. Native argument types | HIGH |
| **Language features** | Full-featured: green threads, event loop, PEG parser, fibers, struct types, module system | Minimal: basic Lisp (atoms, lists, functions, closures, macros). No built-in module system | MEDIUM |
| **Performance** | Compiles to bytecode. Generally faster for complex scripts | Tree-walking interpreter. Adequate for per-keystroke command dispatch (<1ms) | MEDIUM |
| **Community/maintenance** | Larger community (~3.5k stars), maintained by Calvin Rose, regular releases | Smaller community (~300 stars), fewer contributors, less frequent updates | MEDIUM |
| **Build complexity** | Requires C compiler in build chain. cc crate or bindgen for FFI. Platform-specific build issues possible | Pure Rust. `cargo build` just works. No external toolchain dependencies | HIGH |
| **Clojure-inspired syntax** | Has Clojure-inspired persistent data structures, but its own syntax | Standard Lisp syntax. Would need preprocessing for Clojure flavor (deferred) | LOW |
| **Binary size impact** | Janet is <1MB C source, but serde/FFI overhead (Xi learned binary bloat lesson) | Adds only Rust code to binary. No FFI overhead | LOW |

### Key Trade-off

Janet is the **better language** (more features, better performance, larger community). rust_lisp provides **better integration** (no FFI boundary, simpler build, native Rust types).

For a walking skeleton where the goal is proving the plugin architecture works, the integration quality matters more than language features. The FFI boundary with Janet introduces a category of bugs (memory management across FFI, type marshalling errors, build system complexity) that would obscure architectural issues.

## Decision

**Adopt rust_lisp** as the Lisp interpreter for the walking skeleton.

Rationale:
1. **Zero FFI friction**: Registering core primitives is a Rust closure, not a C-compatible extern function. This directly reduces M2 complexity and risk
2. **Build simplicity**: No C compiler dependency. `cargo build` handles everything. Reduces CI/build matrix complexity
3. **Debugging**: Errors stay in Rust-land. No cross-language debugging across FFI boundary
4. **Foreign types**: `Value::Foreign()` can wrap editor state types directly, enabling future pattern 2 (foreign type wrapping) without additional FFI layers
5. **Adequate for scope**: The walking skeleton's Lisp needs are modest -- register primitives, evaluate expressions, define functions, create keymaps. rust_lisp handles all of these

### Migration Path

If rust_lisp proves insufficient post-skeleton (performance, features, or maintenance), migration to Janet or another interpreter is isolated to the `alfred-lisp` crate. The `alfred-core` types and the plugin Lisp source files would need adaptation, but the `alfred-tui`, `alfred-plugin`, and `alfred-bin` crates remain unaffected due to the dependency inversion boundary.

## Alternatives Considered

### Alternative 1: Janet
- **What**: Adopt Janet, a full-featured embeddable Lisp written in C
- **Expected impact**: Richer language features, better performance, larger community
- **Why rejected**: C FFI introduces build complexity, cross-language debugging, and type marshalling overhead. Every core primitive registration requires C-compatible wrappers. The walking skeleton does not need Janet's advanced features (green threads, PEG parser, fibers). The integration friction would slow M2 development without proportional benefit

### Alternative 2: Build Custom Lisp (MAL approach)
- **What**: Build a purpose-built Lisp in Rust following the MAL incremental process
- **Expected impact**: Perfect integration, exact syntax control, full ownership
- **Why rejected**: See ADR-001. Building a Lisp is a project-sized effort that distracts from the architecture proof. Already decided against in discovery phase

### Alternative 3: steel (Scheme in Rust)
- **What**: Adopt steel, an embeddable Scheme interpreter written in Rust
- **Expected impact**: More mature than rust_lisp, Scheme semantics, bytecode compilation
- **Why not selected**: Scheme's semantics (hygienic macros, continuations, tail call requirement) are more complex than needed. The walking skeleton needs a simple Lisp, not a standards-compliant Scheme. rust_lisp's simplicity is an advantage for this scope

## Consequences

### Positive
- Pure Rust build chain -- no external toolchain dependencies
- Direct Rust closure registration for core primitives
- `Value::Foreign()` enables wrapping editor types for Lisp manipulation
- Simpler debugging (single language, single runtime)
- `lisp!` macro allows embedding Lisp expressions in Rust tests

### Negative
- Smaller community and less battle-tested than Janet
- Tree-walking interpreter is slower than Janet's bytecode VM (acceptable if <1ms per command dispatch -- validate at M2)
- Fewer built-in features (no green threads, no PEG parser, no module system)
- Maintenance risk if the project becomes inactive (mitigated: rust_lisp is simple enough to fork and maintain)
- May need to contribute upstream or fork for missing features
