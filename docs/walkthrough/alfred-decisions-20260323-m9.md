# Alfred Editor -- Design Decisions (M9)

Analysis date: 2026-03-23

This document catalogs all architectural decisions recovered from the Alfred codebase.
Decisions are classified as **Documented** (found in ADRs) or **Inferred** (reconstructed
from code structure, commit history, and comments).

---

## Documented Decisions (from docs/adrs/)

### ADR-001: Adopt Existing Lisp Interpreter Rather Than Build From Scratch

- **Status**: Accepted
- **Context**: Alfred needs a Lisp interpreter as extension language. Building a custom interpreter (MAL approach, 11 steps) is a project-sized effort that distracts from the architecture proof.
- **Decision**: Adopt an existing embeddable Lisp interpreter.
- **Alternatives rejected**: (1) Build custom Lisp via MAL -- scope creep risk, interpreter bugs mask architecture issues. (2) Use Lua -- does not provide homoiconicity or macros central to Lisp-based editor identity.
- **Consequences**: Eliminates 3-4 weeks of interpreter development. Less control over syntax. Dependency on external project's maintenance.

### ADR-002: Plugin-First Architecture

- **Status**: Accepted
- **Context**: How much functionality in Rust kernel vs. Lisp extension layer? Evidence from Emacs (~70% Lisp), Helix (no plugins = most-cited limitation), Neovim (LSP as Lua plugin).
- **Decision**: Thin kernel with core primitives only. All user-visible features (keybindings, line numbers, status bar, modal editing) are Lisp plugins.
- **Alternatives rejected**: (1) Full-featured kernel with optional plugins -- does not prove the architecture. (2) Balanced split -- creates blurry boundary.
- **Consequences**: Plugin API is battle-tested by the walking skeleton. Every feature is independently removable. Forces API to be sufficient for real use cases.
- **Validation**: Vim modal editing works as 52 lines of Lisp (M7 completion).

### ADR-003: Single-Process Synchronous Execution

- **Status**: Accepted
- **Context**: Xi editor post-mortem: "process separation between front-end and core was not a good idea." Emacs single-threaded for 40+ years. Kakoune explicitly states no multithreading.
- **Decision**: Single-process, synchronous execution. No async runtime, no multi-process communication, no background threads.
- **Alternatives rejected**: (1) Multi-process (Xi-style) -- author warns against it. (2) Async-everywhere (tokio) -- colored function problem, premature complexity. (3) Thread pool -- not needed for walking skeleton scope.
- **Consequences**: Simplest execution model. No synchronization bugs. Deterministic behavior. Long Lisp expressions would freeze UI (acceptable for current scope).

### ADR-004: rust_lisp Over Janet

- **Status**: Accepted
- **Context**: Janet is the better language (bytecode VM, green threads, larger community). rust_lisp provides better integration (native Rust, no FFI, direct closure registration).
- **Decision**: Adopt rust_lisp. Zero FFI friction, pure Rust build chain, single-language debugging.
- **Alternatives rejected**: (1) Janet -- C FFI introduces build complexity and cross-language debugging. (2) Build custom Lisp -- see ADR-001. (3) steel (Scheme) -- more complex semantics than needed.
- **Consequences**: Pure Rust build chain. Direct closure registration. Smaller community (mitigated: simple enough to fork). Tree-walking interpreter slower than Janet's bytecode VM (validated: all primitives under 1ms).
- **Migration path**: Isolated to alfred-lisp crate. alfred-core, alfred-tui, alfred-bin unaffected.

### ADR-005: Hybrid Development Paradigm (Functional Core, Imperative Shell)

- **Status**: Accepted
- **Context**: Rust is multi-paradigm. Editor domain has both pure-functional aspects (buffer transformations, key resolution) and inherently stateful aspects (event loop, terminal I/O).
- **Decision**: Functional core (alfred-core) with imperative shell (alfred-tui, alfred-bin). Traits as ports at boundaries.
- **Alternatives rejected**: (1) Pure functional -- Rust's ownership model fights pure FP patterns. (2) OOP -- deep trait hierarchies fight Rust's ownership model; dynamic dispatch has runtime cost.
- **Consequences**: alfred-core is highly testable (pure functions, no mocking). Effect boundaries visible in crate structure. Some operations that feel like methods are free functions for purity.

### ADR-006: Cargo Workspace Crate Structure

- **Status**: Accepted
- **Context**: Rust's Cargo workspaces provide crate-level visibility enforcement. Crate boundaries are architectural enforcement mechanisms.
- **Decision**: 5 crates -- alfred-core (pure), alfred-lisp (interpreter), alfred-plugin (plugins), alfred-tui (UI), alfred-bin (binary). Key invariant: alfred-core has zero dependencies on other Alfred crates.
- **Alternatives rejected**: (1) Single crate with modules -- module-level visibility is weaker than crate boundaries. (2) More granular crates (6+) -- over-decomposition creates circular dependency pressure.
- **Consequences**: alfred-core purity enforced by Cargo. Independent compilation. Refactoring across crate boundaries requires updating multiple Cargo.toml files.

---

## Inferred Decisions (from code analysis)

### Inferred: Rope-Based Undo via Whole-Buffer Snapshots

- **Context**: Undo/redo requires capturing editor state before destructive edits.
- **Decision**: Store `UndoSnapshot { buffer: Buffer, cursor: Cursor }` on a Vec stack. Rope cloning is O(1) via structural sharing, making whole-buffer snapshots cheap.
- **Evidence**: `undo_stack` and `redo_stack` fields on EditorState. `push_undo()` called before J, dd, cc, C, yy, p commands. ropey documentation confirms O(1) clone.
- **Alternatives not explored**: (1) Operation-based undo (store individual edits and reverse them). (2) Persistent data structure with immutable history. The current approach is simpler and leverages ropey's structural sharing.
- **Confidence**: High. The code is explicit and the ropey O(1) clone property is well-documented.

### Inferred: DeferredAction Pattern for RefCell Safety

- **Context**: Lisp commands capture `Rc<RefCell<EditorState>>` and call `borrow_mut()` internally. The event loop also borrows the RefCell. If both borrow simultaneously, the program panics.
- **Decision**: `handle_key_event` returns a `DeferredAction` enum instead of executing the command immediately. The event loop drops its borrow, then executes the deferred action.
- **Evidence**: Two explicit fix commits (06847bf, 765f95f) reference "RefCell double-borrow." The DeferredAction enum has 6 variants (None, Eval, ExecCommand, SaveBuffer, OpenFile, SaveAndQuit).
- **Confidence**: High. The problem and solution are documented in commit messages.

### Inferred: Hook-as-Signal Pattern for UI Features

- **Context**: Line numbers and status bar are plugin-controlled, but the formatting logic is performance-sensitive (runs every frame).
- **Decision**: The Lisp hook callback's *presence* signals the feature should activate. The actual formatting (line number width calculation, status bar content) is done in Rust (compute_gutter_content, compute_status_content).
- **Evidence**: line-numbers plugin callback just returns `start` argument unchanged. status-bar plugin callback returns a static string `"status-bar-active"`. The Rust code checks `if results.is_empty()` to decide whether to render.
- **Alternatives not explored**: (1) Full Lisp formatting (slower, more flexible). (2) Feature flags (no plugin involvement).
- **Confidence**: High. The pattern is consistent across both hooks and the callbacks are clearly sentinel-only.

### Inferred: ClonedHandler Pattern for Command Execution

- **Context**: The CommandRegistry is part of EditorState. Looking up a handler borrows the registry (and thus EditorState). Calling the handler also needs EditorState access.
- **Decision**: `extract_handler()` returns a `ClonedHandler` (Native fn pointer is copied, Dynamic Rc is cloned) so the registry borrow can be dropped before calling the handler.
- **Evidence**: `ClonedHandler` enum and `extract_handler` method exist alongside the original `lookup`. Comment: "Clone/copy the handler to release the borrow on state before calling it."
- **Confidence**: High. The code and comments are explicit.

### Inferred: Plugin Metadata in Header Comments (Not Separate Config File)

- **Context**: Plugin metadata (name, version, description, dependencies) needs to be declared somewhere.
- **Decision**: Embed metadata as `;;; key: value` comments in the init.lisp header, rather than a separate package.json or plugin.toml.
- **Evidence**: `parse_metadata` in discovery.rs parses `;;; name:`, `;;; version:`, `;;; description:`, `;;; depends:` prefixes. All 5 plugins follow this format.
- **Alternatives not explored**: (1) TOML/JSON config file per plugin. (2) Lisp-level metadata forms.
- **Confidence**: High. The format is consistent and discovery.rs is built around it.

### Inferred: Key Representation as Formatted String

- **Context**: Lisp plugins need to bind keys to commands. Lisp has no native representation of KeyEvent.
- **Decision**: Keys are represented as formatted strings: `"Char:h"`, `"Ctrl:r"`, `"Escape"`, `"Backspace"`. The bridge parses these strings into KeyEvent values.
- **Evidence**: All define-key calls in plugins use string format. bridge.rs contains parse logic for `Char:`, `Ctrl:`, and bare key names.
- **Alternatives not explored**: (1) Structured Lisp data `(key 'h :ctrl)`. (2) Integer keycodes.
- **Confidence**: High. The pattern is used consistently across all plugins.

### Inferred: Dummy EditorState for Dynamic Command Execution

- **Context**: Dynamic (Lisp) command handlers capture their own `Rc<RefCell<EditorState>>` and ignore the `&mut EditorState` argument passed to them. But the ClonedHandler::call() signature requires an `&mut EditorState`.
- **Decision**: Create a temporary dummy EditorState (`editor_state::new(1,1)`) to pass to dynamic handlers. The handler ignores it and uses its captured Rc instead.
- **Evidence**: In app.rs event loop: `let mut dummy = editor_state::new(1, 1); f(&mut dummy)`. Comment: "Dynamic (Lisp) handlers capture their own Rc<RefCell<EditorState>>."
- **Alternatives not explored**: (1) Separate handler trait with different signatures for native vs dynamic. (2) Pass the Rc directly.
- **Confidence**: Medium. This is a pragmatic workaround. The comment acknowledges the asymmetry.

---

## Decision Summary

| Category | Count |
|----------|-------|
| Documented (ADRs) | 6 |
| Inferred (code analysis) | 7 |
| **Total** | **13** |

All documented decisions follow the Context > Decision > Alternatives > Consequences format.
Inferred decisions are labeled with confidence levels and supporting evidence.
