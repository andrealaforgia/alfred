# Alfred Editor -- Recovered and Inferred Design Decisions

**Generated**: 2026-03-21
**Scope**: Full codebase analysis at M2 completion

---

## Documented Decisions (from docs/adrs/)

### ADR-001: Adopt Existing Lisp Interpreter Rather Than Build From Scratch

- **Status**: Accepted
- **Context**: Alfred needs a Lisp interpreter. Building one from scratch (MAL, 11 steps) is project-sized effort. The editor's goal is proving architecture, not building a language.
- **Decision**: Adopt an existing embeddable Lisp interpreter.
- **Alternatives rejected**: Custom Lisp (MAL approach) -- too much effort, interpreter bugs would mask architecture issues. Lua -- no homoiconicity, wrong identity for an Emacs-inspired editor.
- **Consequences**: 3-4 weeks saved. Less control over syntax. Dependency on external project.

### ADR-002: Plugin-First Architecture

- **Status**: Accepted
- **Context**: Spectrum from full-featured kernel to thin kernel with everything as plugins. Evidence: Emacs (~70% Lisp), Helix (no plugins = most-cited limitation).
- **Decision**: Thin kernel provides only primitives. All user-visible features (keybindings, line numbers, status bar, modal editing) are Lisp plugins.
- **Alternatives rejected**: Full-featured kernel -- does not prove architecture. Balanced split -- blurry boundary, missed validation opportunities.
- **Consequences**: Plugin API battle-tested by walking skeleton itself. More Lisp code required. Performance risk for per-keystroke Lisp evaluation.

### ADR-003: Single-Process Synchronous Execution

- **Status**: Accepted
- **Context**: Xi editor retrospective: "process separation was not a good idea." Emacs: single-threaded for 40+ years.
- **Decision**: Single process, synchronous event loop, no async runtime, no threads.
- **Alternatives rejected**: Multi-process (Xi-style) -- author explicitly warns against it. Async-everywhere (tokio) -- colored function problem, no operations justify it. Thread pool -- not needed for walking skeleton.
- **Consequences**: Simplest model. No synchronization bugs. Long Lisp expressions freeze UI (acceptable).

### ADR-004: Lisp Interpreter Selection -- rust_lisp Over Janet

- **Status**: Accepted
- **Context**: Two validated candidates. Janet is better language (features, performance, community). rust_lisp has better integration (no FFI, native Rust closures, simpler build).
- **Decision**: Adopt rust_lisp for the walking skeleton.
- **Alternatives rejected**: Janet -- C FFI introduces build complexity and cross-language debugging. Custom (MAL) -- already rejected in ADR-001. Steel (Scheme) -- more complex semantics than needed.
- **Consequences**: Pure Rust build chain. Direct closure registration. Smaller community risk. Kill signal: if >1ms eval latency, evaluate Janet.

### ADR-005: Hybrid Development Paradigm (Functional Core, Imperative Shell)

- **Status**: Accepted
- **Context**: Rust is multi-paradigm. Editor has both pure (buffer transforms, key resolution) and stateful (event loop, terminal I/O) aspects.
- **Decision**: Functional core (alfred-core, pure functions) + imperative shell (alfred-tui, alfred-bin, mutable state and I/O).
- **Alternatives rejected**: Pure FP -- Rust ownership model fights persistent data structures. Pure OOP -- deep trait hierarchies inappropriate for Rust; editor data flow is a pipeline.
- **Consequences**: alfred-core testable without mocking. Effect boundaries explicit in crate structure. Requires discipline to keep core pure.

### ADR-006: Cargo Workspace Crate Structure

- **Status**: Accepted
- **Context**: Rust's Cargo workspaces enforce crate-level visibility. Crate boundaries become architectural enforcement, not just organization.
- **Decision**: 5 crates: alfred-core, alfred-lisp, alfred-plugin, alfred-tui, alfred-bin. alfred-core has zero dependencies on other Alfred crates.
- **Alternatives rejected**: Single crate with modules -- weaker boundaries, not compiler-enforced. 6+ crates -- over-decomposition, circular dependency pressure.
- **Consequences**: Cargo enforces dependency rule. Each crate compiles independently. Refactoring across boundaries requires updating multiple Cargo.toml.

---

## Inferred Decisions (from code analysis)

### Inferred-001: Use Rc<RefCell<EditorState>> for Lisp Bridge Sharing

- **Status**: Implemented (not documented as ADR)
- **Context**: The Lisp bridge registers closures that need to read/write EditorState. The event loop also needs mutable access. Rust's borrow rules prevent multiple &mut references.
- **Decision**: Wrap EditorState in `Rc<RefCell<>>` at the alfred-bin composition root. Bridge closures capture cloned Rc. Event loop borrows via the same Rc.
- **Tension**: component-boundaries.md explicitly states "no Rc<RefCell<T>>". Implementation diverged from documented design.
- **Why this works**: Single-threaded execution means RefCell's runtime checks are sufficient. No risk of actual data races.
- **Risk**: RefCell panics at runtime if borrow rules violated. Mitigated by careful borrow scoping in app.rs (drop mutable borrow before Lisp eval).

### Inferred-002: Free Functions Over Methods for Core Domain

- **Status**: Implemented consistently
- **Context**: Buffer, Cursor, and Viewport operations could be implemented as methods (`buffer.insert_at()`) or free functions (`insert_at(&buffer, ...)`).
- **Decision**: Use free functions. Buffer, Cursor, Viewport are data types. Operations are in module-level functions.
- **Rationale** (inferred): Aligns with functional paradigm. Free functions make purity visible -- they cannot access hidden state. Also avoids Rust's `&self` vs `&mut self` method receiver complexity.
- **Consequence**: API reads as `buffer::insert_at(&buffer, line, col, text)` rather than `buffer.insert_at(line, col, text)`. Unfamiliar to OOP developers.

### Inferred-003: Command-Line Mode (`:`) Instead of Ctrl-Q for Quit

- **Status**: Implemented (commit 0fe2bb7)
- **Context**: M1 originally used Ctrl-Q for quit. This was replaced with `:q` command-line mode (Vim/Emacs-style).
- **Decision**: Implement a command-line input mode triggered by `:`. Support `:q`, `:quit`, `:eval <expr>`.
- **Rationale** (inferred): Prepares for M6/M7 where all keybindings will be plugin-defined. The `:` prefix pattern is extensible (add commands without changing key dispatch).
- **Consequence**: More complex input state machine (InputState::Normal vs InputState::Command). But cleaner separation of key handling from command execution.

### Inferred-004: Viewport Gutter Width Field (Forward Design)

- **Status**: Implemented as field, not yet used
- **Context**: Viewport struct has a `gutter_width: u16` field initialized to 0. No code currently reads this field for rendering.
- **Decision**: Include gutter_width in the Viewport data model from M1.
- **Rationale** (inferred): Forward design for M4 (line numbers plugin). The line-numbers plugin will set gutter_width, and the renderer will offset text accordingly.
- **Consequence**: Slight premature design, but low cost (single field). Avoids breaking Viewport's API when M4 arrives.

### Inferred-005: AtomicU64 for Buffer ID Generation

- **Status**: Implemented
- **Context**: Each Buffer needs a unique ID. IDs are generated by a global atomic counter.
- **Decision**: Use `static NEXT_BUFFER_ID: AtomicU64` with `Ordering::Relaxed`.
- **Rationale** (inferred): Despite single-threaded execution model (ADR-003), AtomicU64 is safe across all Rust compilation contexts. Using a simpler counter (Cell<u64>) would require unsafe or thread_local, which is more complex for no practical benefit.
- **Consequence**: Minimal overhead. IDs are monotonically increasing but not guaranteed sequential if buffers are created across tests.

### Inferred-006: Eval-Then-Display Pattern for Lisp Integration

- **Status**: Implemented (app.rs, eval_and_display function)
- **Context**: When the user runs `:eval (message "hi")`, the Lisp `message` primitive sets `state.message` during evaluation. But if the expression returns a value (like `(+ 1 2)`), the result should also display.
- **Decision**: Clear message before eval. If a bridge primitive set the message during eval, keep it. Otherwise, display the eval result.
- **Rationale** (inferred): Matches Emacs minibuffer behavior -- primitives with side effects show their effects, pure expressions show their return values.
- **Consequence**: Slightly complex logic in eval_and_display. Requires clearing message before eval and checking after.
