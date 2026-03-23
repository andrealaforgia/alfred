# Alfred Editor -- Recovered Design Decisions

Generated: 2026-03-23
Analysis: Walking Skeleton M1-M7 Complete

---

## Documented Decisions (from ADRs)

### ADR-001: Adopt Existing Lisp Interpreter

**Status**: Accepted

**Context**: Alfred requires a Lisp interpreter as its extension language. Building a custom Lisp (MAL approach, 11 steps) is a project-sized effort. The walking skeleton's goal is proving plugin architecture, not building a language.

**Decision**: Adopt an existing embeddable Lisp interpreter rather than building from scratch.

**Consequences**:
- Positive: Eliminated 3-4 weeks of interpreter development
- Positive: Inherited reliability from proven interpreter
- Negative: Less control over language syntax and semantics
- Negative: Dependency on external project's maintenance

---

### ADR-002: Plugin-First Architecture

**Status**: Accepted

**Context**: Three positions on the kernel-vs-plugin spectrum: full-featured kernel with optional plugins, balanced split, or thin kernel where everything is a plugin. Evidence from Emacs (~70% Lisp), Neovim (LSP as Lua plugin), and Helix (no plugins, most-cited limitation).

**Decision**: Plugin-first. The kernel provides only core primitives (buffer, cursor, keymap, hook, rendering). All user-visible features are Lisp plugins.

**Consequences**:
- Positive: Plugin API battle-tested by the walking skeleton itself
- Positive: Every feature independently removable
- Positive: Proves the architecture claim end-to-end
- Negative: More Lisp code for basic features
- Negative: Per-keystroke performance goes through the Lisp interpreter

---

### ADR-003: Single-Process Synchronous Execution

**Status**: Accepted

**Context**: The walking skeleton does not require async I/O, green threads, or multi-threading. Adding async complexity would obscure the architecture proof.

**Decision**: Single-process, synchronous, single-threaded execution.

**Consequences**:
- Positive: Simple mental model, no race conditions
- Positive: No async runtime dependency
- Negative: Cannot do background work (LSP, file watching) without architecture change

---

### ADR-004: rust_lisp Over Janet

**Status**: Accepted

**Context**: Two candidates validated: Janet (C-based, full-featured, larger community) and rust_lisp (native Rust, minimal, smaller community). Key trade-off: Janet is the better language, rust_lisp provides better integration.

**Decision**: Adopt rust_lisp. Zero FFI friction, pure Rust build chain, direct closure registration.

**Migration path**: If rust_lisp proves insufficient, migration to Janet is isolated to the `alfred-lisp` crate.

**Performance gate**: 1ms per primitive eval. Validated at M2 with kill-signal benchmark tests.

**Consequences**:
- Positive: No C compiler dependency, `cargo build` just works
- Positive: Native Rust closure registration
- Negative: Smaller community, tree-walking interpreter
- Negative: Fewer built-in features than Janet

---

### ADR-005: Hybrid Development Paradigm (Functional Core, Imperative Shell)

**Status**: Accepted

**Context**: Rust supports both functional and imperative styles. A pure functional core enables easier testing and reasoning, while I/O naturally lives in an imperative shell.

**Decision**: Types-first design with algebraic data types. Pure functions for all domain logic. I/O at boundaries only (event loop, renderer). Immutable domain operations return new values.

**Consequences**:
- Positive: Buffer, Cursor, Viewport operations are all testable without I/O
- Positive: State changes are explicit (new values, not mutation)
- Negative: EditorState aggregation requires `Rc<RefCell>` for shared Lisp access

---

### ADR-006: Cargo Workspace Crate Structure

**Status**: Accepted

**Context**: Crate-level visibility enforcement makes crate boundaries an architectural enforcement mechanism. Key constraint: `alfred-core` must have zero outward dependencies.

**Decision**: 5-crate workspace: core, lisp, plugin, tui, bin. Dependencies point inward.

**Consequences**:
- Positive: `alfred-core` purity enforced by Cargo at compile time
- Positive: Parallel compilation, clear dependency graph
- Negative: Inter-crate type sharing requires careful API design

---

## Inferred Decisions (from code analysis)

### Inferred-001: DeferredAction Pattern for Borrow Safety

**Status**: Implemented (evolved during M3)

**Context**: `handle_key_event` borrows `EditorState` mutably. Lisp-defined commands also borrow `EditorState` via `Rc<RefCell>`. Nested borrows cause runtime panics. Two bugs were discovered and fixed during M3 (commits `765f95f`, `06847bf`).

**Decision**: `handle_key_event` returns a `DeferredAction` enum (`None | Eval(String) | ExecCommand(String)`) instead of executing commands inline. The event loop drops the borrow, then executes the deferred action.

**Evidence**: `DeferredAction` enum in `app.rs`, `ClonedHandler` pattern in `command.rs`, two bug-fix commits.

**Consequences**:
- Positive: Eliminates RefCell double-borrow panics
- Positive: Clear separation between key interpretation and command execution
- Negative: Adds indirection to the event loop
- Negative: Dynamic (Lisp) handlers require a "dummy" EditorState pass-through

---

### Inferred-002: Hook Presence as Feature Flag

**Status**: Implemented (introduced at M4)

**Context**: Optional features (line numbers, status bar) need a way to signal their presence to the TUI. Traditional approaches: boolean config flags, feature flags, or capability queries.

**Decision**: The hook's presence IS the feature flag. `compute_gutter_content` dispatches `"render-gutter"` hook. If no callbacks registered, returns `(0, [])` and no gutter renders. No boolean config needed.

**Evidence**: `compute_gutter_content` and `compute_status_content` in `app.rs` both check `results.is_empty()`. Line-numbers plugin registers a single callback. Status-bar plugin registers a single callback.

**Consequences**:
- Positive: Zero-config feature activation (load plugin = enable feature)
- Positive: Rendering code has zero knowledge of specific features
- Negative: Hook dispatch occurs every render frame even when no hooks registered
- Negative: Feature presence is implicit (must know to look for hook registrations)

---

### Inferred-003: Self-Insert in Event Loop, Not Plugin

**Status**: Implemented (introduced at M6-M7)

**Context**: In insert mode, unbound printable characters should be inserted into the buffer. This could be handled by: (a) binding every printable character in the keymap, (b) a "self-insert" command, or (c) fallback behavior in the event loop.

**Decision**: Self-insert lives in the event loop. When `resolve_key` returns `None` for a printable character while in insert mode with active keymaps, `handle_key_event` inserts the character directly.

**Evidence**: `handle_key_event` in `app.rs`, the `None =>` match arm checks `MODE_INSERT` and `KeyCode::Char(c)`.

**Consequences**:
- Positive: No Lisp eval overhead for every typed character in insert mode
- Positive: No need to bind 95+ printable characters in the keymap
- Negative: This is the ONE behavior not defined by plugins
- Negative: Cannot override self-insert from a Lisp plugin

---

### Inferred-004: Metadata in Lisp Comment Headers

**Status**: Implemented (introduced at M3)

**Context**: Plugin metadata (name, version, dependencies) must be accessible before evaluating the Lisp code. Options: separate metadata file (TOML/JSON), Lisp-level metadata form, or comment-based header.

**Decision**: Metadata is parsed from `;;; key: value` comment headers in `init.lisp`. This avoids evaluating Lisp code to discover metadata and keeps everything in one file.

**Evidence**: `parse_metadata` function in `discovery.rs` parses `;;; name:`, `;;; version:`, `;;; description:`, `;;; depends:` prefixes.

**Consequences**:
- Positive: Metadata extraction is pure string parsing, no Lisp eval needed
- Positive: Single file per plugin (no separate metadata file)
- Negative: Non-standard format (not TOML, JSON, or Lisp forms)
- Negative: Limited expressiveness (flat key-value pairs only)

---

### Inferred-005: ClonedHandler for Registry Borrow Release

**Status**: Implemented (introduced at M3)

**Context**: `CommandRegistry` stores handlers. Executing a handler requires `&mut EditorState`, which contains the `CommandRegistry`. Borrowing the handler from the registry while mutating the state creates a borrow conflict.

**Decision**: `extract_handler` clones/copies the handler out of the registry into a `ClonedHandler` enum. For `Native` handlers, the `fn` pointer is copied. For `Dynamic` handlers, the `Rc` is cloned. The registry borrow is released before execution.

**Evidence**: `ClonedHandler` enum and `extract_handler` method in `command.rs`.

**Consequences**:
- Positive: Clean borrow separation
- Positive: Works for both native fn pointers and Rc-wrapped closures
- Negative: Adds a wrapper type and an extra method
- Negative: Dynamic handler execution uses a dummy EditorState parameter

---

### Inferred-006: Mode-Keymap Coupling via set-mode

**Status**: Implemented (introduced at M7)

**Context**: Vim-style editing requires mode switches to also change the active keymap. Mode and keymap could be independent (manually synced by plugins) or coupled (changing mode automatically changes keymap).

**Decision**: `set-mode` bridges both. It sets `state.mode` AND swaps `state.active_keymaps` to the corresponding keymap. The convention is: mode "normal" uses keymap "normal-mode", mode "insert" uses keymap "insert-mode".

**Evidence**: `register_set_mode` in `bridge.rs` sets both `state.mode` and rebuilds `state.active_keymaps` based on the mode name + "-mode" suffix convention.

**Consequences**:
- Positive: Single Lisp call switches both mode and keymap
- Positive: Plugins define the mode-to-keymap mapping by naming convention
- Negative: Coupling is implicit (naming convention, not explicit mapping)
- Negative: Cannot have a mode without a corresponding keymap (or vice versa) without workaround
