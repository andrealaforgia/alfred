# Alfred: Recovered and Documented Design Decisions

This document lists all design decisions found in the Alfred codebase, both documented (from ADRs) and inferred (from code patterns and commit history).

---

## Documented Decisions (from docs/adrs/)

### ADR-001: Adopt Existing Lisp Interpreter Rather Than Build From Scratch

**Status:** Accepted

**Context:** Alfred needs a Lisp extension language. Building a custom Lisp interpreter (following the MAL 11-step process) would take 3-4 weeks and introduce interpreter bugs that could obscure plugin architecture issues.

**Decision:** Adopt an existing embeddable Lisp interpreter. Do not build a custom one for the walking skeleton.

**Consequences:**
- Positive: Eliminated 3-4 weeks of development. Inherited reliability. Smaller codebase.
- Negative: Less control over syntax/semantics. Dependency on external project's maintenance.

---

### ADR-002: Plugin-First Architecture

**Status:** Accepted

**Context:** How much functionality should live in Rust vs. Lisp? Evidence from Emacs (~70% Lisp) and Helix (no plugins, most criticized limitation) informed the decision.

**Decision:** Thin kernel. Everything beyond core primitives (buffer, cursor, commands, hooks, keymaps, rendering) is a Lisp plugin. Keybindings, line numbers, status bar, and theme are all plugins.

**Consequences:**
- Positive: Plugin API battle-tested by the walking skeleton. Clean kernel boundary. Every feature independently removable.
- Negative: More Lisp code for basic features. Plugin API must support complex features (modal editing).

---

### ADR-003: Single-Process Synchronous Execution

**Status:** Accepted

**Context:** Xi editor's post-mortem: "I now firmly believe that the process separation between front-end and core was not a good idea." Emacs has been single-threaded for 40+ years.

**Decision:** Single-process, synchronous execution. No async runtime, no threads, no IPC.

**Consequences:**
- Positive: Simplest execution model. No synchronization bugs. Deterministic behavior.
- Negative: Long-running Lisp freezes UI. No parallelism for future CPU-intensive features.

---

### ADR-004: rust_lisp Over Janet

**Status:** Accepted

**Context:** Two candidates: Janet (C-based, more features, larger community) vs. rust_lisp (Rust-native, simpler, direct closure registration).

**Decision:** rust_lisp. Integration quality over language features. No C FFI, no cross-language debugging, pure Rust build chain.

**Consequences:**
- Positive: Zero FFI friction. Build simplicity. Native Rust debugging.
- Negative: Smaller community. Tree-walking interpreter (slower than Janet's bytecode). Fewer built-in features.
- Migration path: Isolated to alfred-lisp crate if rust_lisp proves insufficient.

---

### ADR-005: Hybrid Development Paradigm (Functional Core, Imperative Shell)

**Status:** Accepted

**Context:** Rust is multi-paradigm. Buffer transformations are pure; terminal I/O is inherently effectful.

**Decision:** Functional core in alfred-core (pure functions, no side effects). Imperative shell in alfred-tui and alfred-bin (mutable state, terminal I/O).

**Consequences:**
- Positive: Core is trivially testable. Effect boundaries explicit in crate structure.
- Negative: Discipline required to keep core pure. Some methods become free functions for purity.

---

### ADR-006: Cargo Workspace Crate Structure

**Status:** Accepted

**Context:** Need modular organization with compile-time enforcement of architectural boundaries.

**Decision:** 5-crate Cargo workspace: alfred-core, alfred-lisp, alfred-plugin, alfred-tui, alfred-bin.

**Consequences:**
- Positive: Core purity enforced by Cargo. Parallel compilation. Visible dependency graph.
- Negative: Inter-crate type sharing requires careful API design. Refactoring across crate boundaries is more work.

---

## Inferred Decisions (from code patterns)

### Inferred: Rc<RefCell<EditorState>> for Shared Mutable State

**Status:** Inferred from code pattern in alfred-bin/src/main.rs and alfred-lisp/src/bridge.rs

**Context:** Both the Rust event loop and Lisp bridge closures need mutable access to EditorState. Rust's ownership model does not allow multiple &mut references.

**Decision:** Wrap EditorState in `Rc<RefCell<EditorState>>`. The event loop borrows it, Lisp closures capture clones of the Rc, and runtime borrow checking (RefCell) prevents simultaneous mutation.

**Consequences:**
- Positive: Works within Rust's ownership model. No unsafe code needed.
- Negative: Runtime borrow panics possible if bridge closures are called during an active borrow. The "DeferredAction" pattern in app.rs was introduced specifically to avoid this.

---

### Inferred: DeferredAction Pattern to Avoid RefCell Panics

**Status:** Inferred from app.rs DeferredAction enum and event loop structure

**Context:** handle_key_event borrows EditorState mutably. Some actions (Lisp eval, command execution) also need to borrow EditorState through the bridge closures. Executing them inside handle_key_event would cause a RefCell double-borrow panic.

**Decision:** handle_key_event returns a DeferredAction enum (Eval, ExecCommand, SaveBuffer, OpenFile, SaveAndQuit). The event loop drops the borrow, then executes the deferred action separately.

**Consequences:**
- Positive: Eliminates all RefCell double-borrow panics. Clear separation of key handling and action execution.
- Negative: More complex event loop. Two-phase dispatch pattern may confuse new contributors.

---

### Inferred: Ropey for Text Storage

**Status:** Inferred from Cargo.toml dependency and buffer.rs implementation

**Context:** Text editors need efficient insertion and deletion in the middle of large files. A Vec<String> (line array) requires O(n) for mid-file operations.

**Decision:** Use ropey, a rope data structure library. Insertions and deletions are O(log n). Cloning is O(1) due to structural sharing (critical for cheap undo snapshots).

**Consequences:**
- Positive: Efficient for large files. O(1) clone enables whole-buffer undo snapshots. Battle-tested library (used by Helix editor).
- Negative: Line access requires going through ropey's API (RopeSlice). Some operations (get_line) may return None for the contiguous string representation.

---

### Inferred: Command Names as Strings (Not Enums)

**Status:** Inferred from command.rs and keymap system

**Context:** Commands are identified by name (e.g., "cursor-up", "delete-line"). Could use Rust enums for compile-time safety.

**Decision:** Use String names for commands. This allows Lisp plugins to register new commands at runtime without modifying Rust code.

**Consequences:**
- Positive: Fully extensible. Plugins add new commands without Rust changes. Keymaps can reference commands that do not exist yet.
- Negative: No compile-time verification of command names. Typos in command names become runtime errors.

---

### Inferred: Hook-Based Feature Activation

**Status:** Inferred from line-numbers and status-bar plugin implementations

**Context:** Features like line numbers and the status bar could be always-on or plugin-activated.

**Decision:** Features are activated by hook presence. The line-numbers plugin registers a callback for "render-gutter". The TUI checks if any callbacks exist for that hook. If none, no gutter is rendered. If present, gutter content is computed and displayed.

**Consequences:**
- Positive: Zero cost when plugin is not loaded. Features are truly optional. Clean deactivation (just remove the plugin folder).
- Negative: Slightly indirect -- understanding why line numbers appear requires knowing about the hook chain, not just the plugin code.

---

### Inferred: Per-Feature Mutation Testing Strategy

**Status:** Documented in CLAUDE.md as "per-feature"

**Context:** Mutation testing validates test quality by introducing small code changes and checking if tests catch them.

**Decision:** Run mutation testing per-feature (not across the entire codebase). Evidence: mutants.out.old/ directory with mutation results for alfred-core modules.

**Consequences:**
- Positive: Focused mutation testing on the most critical code. Faster feedback loop.
- Negative: Cross-crate mutations may be missed.
