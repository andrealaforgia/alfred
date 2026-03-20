# Alfred Editor -- Walking Skeleton Roadmap (M1-M7)

**Feature**: alfred-core
**Date**: 2026-03-19

---

## Roadmap Metadata

- **Total milestones**: 7
- **Estimated total duration**: 12-14 weeks
- **Execution model**: Sequential. Each milestone gate must pass before next begins
- **Paradigm**: Hybrid (functional core, imperative shell)
- **Development**: AI agents execute, human reviews

---

## M1: Rust Kernel -- Event Loop, Buffer, Rendering, Navigation

**What it proves**: Can display a file and navigate it with arrow keys.

**Duration**: ~2 weeks

**Deliverables**:
- Cargo workspace with 5 crates (alfred-core, alfred-lisp as stub, alfred-plugin as stub, alfred-tui, alfred-bin)
- Rope-based buffer wrapping ropey (load file from CLI arg)
- Terminal raw mode via crossterm, rendering via ratatui
- Viewport management with scrolling
- Cursor movement (arrow keys, hardcoded in Rust)
- Quit with Ctrl-Q

**Acceptance criteria**:
- Open a file passed as CLI argument, content renders in terminal
- Arrow keys move cursor; scrolling occurs when cursor exits viewport
- Ctrl-Q exits cleanly (terminal restored to normal mode)
- Buffer is read-only (no text insertion/deletion)
- `alfred-core` has zero terminal/IO dependencies

**Architectural constraints**:
- Buffer and cursor logic in `alfred-core` (pure, no I/O)
- Event loop and rendering in `alfred-tui` (imperative shell)
- Keybindings are temporarily hardcoded in `alfred-tui` (replaced in M6)

**Kill signal**: If basic rendering + navigation takes >3 weeks, reassess approach.

---

## M2: Adopt and Integrate Lisp, Expose Core Primitives

**What it proves**: Can evaluate Lisp expressions that call Rust primitives.

**Duration**: ~2 weeks

**Deliverables**:
- rust_lisp integrated into `alfred-lisp` crate
- FFI bridge: registered native functions pattern
- Core primitives exposed: buffer-insert, buffer-delete, cursor-move, cursor-position, message
- Lisp evaluation callable from the event loop (command-line or file loading)
- Performance baseline: measure per-expression evaluation latency

**Acceptance criteria**:
- Lisp expression `(buffer-insert "hello")` modifies the buffer content
- Lisp expression `(cursor-move :down 5)` moves the cursor
- Lisp expression `(message "test")` displays text in the message area
- Evaluation latency <1ms for single primitive calls
- Lisp errors display as messages, do not crash the editor

**Architectural constraints**:
- All Lisp-to-Rust communication via registered native functions (no direct struct access)
- `alfred-lisp` depends on `alfred-core` for types only
- Bridge module registers primitives at editor startup

**Kill signal**: If neither rust_lisp meets <1ms latency target, evaluate Janet before proceeding.

---

## M3: Plugin System -- Discovery, Loading, Lifecycle

**What it proves**: Can discover, load, initialize, and unload Lisp plugins.

**Duration**: ~2 weeks

**Deliverables**:
- Plugin discovery: scan `plugins/` directory for subdirectories containing `init.lisp`
- Plugin metadata parsing from Lisp source (name, version, description)
- Plugin lifecycle: init function called on load, cleanup on unload
- Plugin registry: track loaded plugins, enforce load order via topological sort
- Test plugin: `plugins/test-plugin/init.lisp` that registers a command

**Acceptance criteria**:
- Plugin in `plugins/test-plugin/init.lisp` discovered at startup
- Plugin's init function called, command registered, command callable
- Unloading a plugin removes its registered commands
- Missing plugin directory handled gracefully (warning, not crash)
- Plugin load errors display as messages, do not prevent other plugins from loading

**Architectural constraints**:
- Plugin system in `alfred-plugin` crate
- Plugins communicate with kernel only through registered Lisp primitives
- Plugin cleanup unregisters all commands, hooks, and keymaps the plugin registered

---

## M4: Line Numbers Plugin

**What it proves**: First real Lisp plugin works end-to-end.

**Duration**: ~1 week

**Deliverables**:
- `plugins/line-numbers/init.lisp`
- `render-gutter-hook` added to hook system
- Gutter rendering area in the TUI renderer
- Plugin registers hook callback that returns line number text per line

**Acceptance criteria**:
- Line numbers appear in gutter column when plugin is loaded
- Removing the plugin directory and restarting: no line numbers, no errors
- Line numbers update correctly when scrolling
- Gutter width adjusts to accommodate digit count (e.g., 4 digits for files >999 lines)

**Architectural constraints**:
- Line numbers are rendered by hook callback, not hardcoded in renderer
- Renderer provides hook with line number, hook returns gutter content
- New primitives added only if line-numbers plugin reveals gaps in the API

---

## M5: Status Bar Plugin

**What it proves**: Plugin can render dynamic UI with editor state.

**Duration**: ~1 week

**Deliverables**:
- `plugins/status-bar/init.lisp`
- `render-status-hook` added to hook system
- Status area in the TUI renderer (bottom row)
- Plugin reads filename, cursor position, modified flag, current mode

**Acceptance criteria**:
- Status bar shows filename, cursor position (line:col), and modified indicator
- Moving cursor updates position in status bar in real time
- Modifying buffer (once editing exists in M6) shows modified indicator
- Removing the plugin: status bar disappears, editor still functional

**Architectural constraints**:
- Status bar content provided entirely by plugin via hook
- Renderer reserves bottom row(s) and calls render-status-hook
- Plugin uses cursor-position, buffer-filename, buffer-modified?, current-mode primitives

---

## M6: Basic Keybinding Plugin

**What it proves**: Plugins can intercept input, bind keys, and mutate the buffer.

**Duration**: ~2 weeks

**Deliverables**:
- `plugins/basic-keybindings/init.lisp`
- Keymap primitives: make-keymap, define-key, set-active-keymap
- Command primitives: define-command, execute-command
- Plugin defines: arrow key navigation, character insertion, backspace/delete, Ctrl-Q quit
- Hardcoded keybindings from M1 removed from Rust code

**Acceptance criteria**:
- All navigation works through Lisp-defined keybindings (no hardcoded keys in Rust)
- Text insertion works: typing characters inserts them at cursor
- Backspace deletes character before cursor
- Ctrl-Q quits the editor
- Removing the plugin: editor starts but no keybindings work (proves they are not hardcoded)

**Architectural constraints**:
- This milestone removes all hardcoded keybinding logic from `alfred-tui`
- Event loop resolves keys through keymap system, not a match statement
- Plugin uses make-keymap, define-key, define-command, set-active-keymap

**Key transition**: After M6, all key handling flows through the plugin-defined keymap system. This is the critical architectural inflection point.

---

## M7: Vim Keybindings Plugin

**What it proves**: Full modal editing as a plugin. Architecture proven end-to-end.

**Duration**: ~2-3 weeks

**Deliverables**:
- `plugins/vim-keybindings/init.lisp`
- Modal editing: Normal mode and Insert mode
- Normal mode: h/j/k/l navigation, dd (delete line), x (delete char), 0/$ (line start/end)
- Insert mode: i/a/o to enter, Escape to exit, free text entry
- Mode display: status bar shows current mode (-- NORMAL -- / -- INSERT --)
- Multi-key sequence support: dd with timeout handling

**Acceptance criteria**:
- Start in Normal mode; h/j/k/l navigate; i enters Insert mode
- In Insert mode, typing inserts text; Escape returns to Normal mode
- dd deletes current line in Normal mode
- x deletes character under cursor in Normal mode
- Status bar displays current mode
- All behavior driven by the Lisp plugin, not Rust code

**Architectural constraints**:
- Vim plugin creates two keymaps (normal-map, insert-map) and switches between them
- Mode state managed by the plugin via set-mode/current-mode primitives
- Key sequence timeout handling (for dd) works through the keymap resolver
- The plugin builds on basic-keybindings (extends or replaces it)

**Completion of this milestone = walking skeleton complete.** The architecture is proven: a complex, stateful feature (modal editing) works entirely as a Lisp plugin on top of the thin Rust kernel.

---

## Post-Skeleton Milestones (Deferred, Not Designed)

These are explicitly out of scope. Listed for context only.

| Feature | Why Deferred |
|---------|-------------|
| Undo/redo | Requires Transaction system. Proves editing completeness, not architecture |
| Syntax highlighting | Requires tree-sitter integration. Feature, not architecture proof |
| LSP integration | Large surface area. Proves language tooling, not plugin architecture |
| Dynamic Rust plugins (.so/.dylib) | Platform-specific complexity. Lisp-only sufficient for walking skeleton |
| Split windows | UI complexity. Proves layout, not plugin system |
| Search/replace | Feature, not architecture |
| Mouse support | Input variant, not architecture |
| Config file loading | Convenience, not architecture |
| File save/open commands | Useful but not needed to prove plugin architecture |

---

## Milestone Dependency Graph

```
M1 (Kernel)
  |
  v
M2 (Lisp Integration)
  |
  v
M3 (Plugin System)
  |
  +---> M4 (Line Numbers Plugin)
  |       |
  |       v
  +---> M5 (Status Bar Plugin)
          |
          v
        M6 (Basic Keybinding Plugin)
          |
          v
        M7 (Vim Keybinding Plugin)
```

M4 and M5 could theoretically run in parallel, but sequential execution prevents scope creep and ensures each milestone's lessons inform the next.

---

## Quality Gates Per Milestone

Each milestone must pass before the next begins:

- [ ] All acceptance criteria met
- [ ] No panics under normal operation
- [ ] `alfred-core` dependency rule maintained (zero outward dependencies)
- [ ] New primitives (if any) documented
- [ ] Existing acceptance criteria from previous milestones still pass (regression)
