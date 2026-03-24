# Component Boundaries: Folder Browser

## Overview

The folder browser is implemented across three existing crates with one new Lisp plugin. No new crate is introduced.

---

## alfred-core: Pure Domain

### New Module: `browser`

**Responsibility**: Pure data types and pure transformation functions for the folder browser.

**Contains**:
- `BrowserState` struct (data)
- `DirEntry` struct + `EntryKind` enum (algebraic data types)
- `NavigationEntry` type alias for history stack entries
- Browser command functions (pure transformations on EditorState)
- `MODE_BROWSE` constant

**Does NOT contain**:
- Filesystem IO (no `std::fs`)
- Rendering logic
- Key event handling

### Modified: `editor_state`

**Changes**:
- Add `pub browser: Option<BrowserState>` field to `EditorState`
- Add `MODE_BROWSE` constant (`"browse"`)
- Register browser commands in `register_builtin_commands()` or via Lisp bridge
- Initialize `browser: None` in `new()`

### Boundary Rule

alfred-core has zero IO dependencies. Filesystem operations (read_dir, path classification) happen in alfred-bin or alfred-tui, which populate BrowserState with pre-read data.

---

## alfred-bin: Composition Root

### Modified: `main.rs` / `run_editor()`

**Changes**:
- Classify CLI argument: `Path::is_dir()` vs `Path::is_file()` vs nonexistent
- When directory: read directory entries via `std::fs::read_dir()`, construct `BrowserState`, set on EditorState, set mode to `MODE_BROWSE`, set active keymap to `browse-mode`
- When file: existing behavior (load into buffer)
- When nonexistent: error message to stderr, exit code 1
- When no argument: existing behavior (empty buffer)

**Boundary**: alfred-bin is the composition root. It performs initial IO, then hands pure state to the event loop.

---

## alfred-tui: Event Loop and Rendering

### Modified: `renderer.rs`

**Changes**:
- When `state.mode == MODE_BROWSE` and `state.browser.is_some()`: render browser view instead of buffer content
- Browser view renders in the main content area (same region as buffer text)
- Entry list with cursor highlight, directory path as header, `../` for non-root directories
- Status bar panel continues to render via existing panel system (no change)

**Boundary**: Rendering reads `BrowserState` from `EditorState` immutably. No mutation in rendering.

### Modified: `app.rs` (event loop)

**Changes**:
- Browser commands that need filesystem IO (enter subdirectory, open file) execute as deferred actions
- When `browser-enter` on a directory: read_dir() in the event loop, update BrowserState entries
- When `browser-enter` on a file: `Buffer::from_file()`, transition to normal mode
- When `browser-parent`: read_dir() for parent, update BrowserState

**Boundary**: Filesystem IO happens in the event loop (imperative shell), then pure state is updated.

### Key handling in browse mode

The existing `handle_key_event` function resolves keys through `active_keymaps`. In browse mode, `active_keymaps = ["browse-mode"]`. Keys resolve to browser command names (e.g., `"browser-cursor-down"`), which are dispatched through the existing `CommandRegistry`.

Special handling needed:
- `gg` (double-g) requires a `PendingG` input state variant (similar to existing `PendingChar`, `PendingRegister`, etc.) -- OR the `g` key in browse-mode maps directly to `browser-jump-first` without the two-key sequence, simplifying the implementation. The crafter decides which approach.

---

## plugins/browse-mode: Lisp Plugin

### New Plugin: `plugins/browse-mode/init.lisp`

**Responsibility**: Define the browse-mode keymap and any browse-mode-specific Lisp configuration.

**Contains**:
- `(make-keymap "browse-mode")` and `(define-key ...)` for all browse-mode bindings
- Cursor shape configuration for browse mode: `(set-cursor-shape "browse" "block")`

**Follows existing pattern**: Same structure as `plugins/vim-keybindings/init.lisp`.

### Plugin Metadata

```
;;; name: browse-mode
;;; version: 0.1.0
;;; description: Folder browser keymap and configuration
;;; depends: vim-keybindings
```

---

## Component Interaction Sequence

### Startup with Directory Argument

```
alfred-bin::main()
  |-- classify argument: Path::is_dir() = true
  |-- read_dir(path) -> Vec<DirEntry>
  |-- editor_state::new() with browser = Some(BrowserState { ... })
  |-- mode = "browse", active_keymaps = ["browse-mode"]
  |-- load plugins (including browse-mode/init.lisp -> registers keymap)
  |-- alfred_tui::app::run()
       |-- render_frame(): detects mode=browse, renders browser view
       |-- event loop: key -> resolve_key("browse-mode") -> "browser-cursor-down" -> dispatch
```

### File Open Transition

```
event loop:
  |-- key = Enter, resolve_key -> "browser-enter"
  |-- command: browser-enter reads entry at cursor_index
  |-- entry is File -> deferred action: open file
  |-- Buffer::from_file(selected_path) -> buffer
  |-- state.buffer = buffer
  |-- state.mode = "normal", state.active_keymaps = ["normal-mode"]
  |-- state.browser = None
  |-- highlighter.set_language_for_file(filename)
  |-- next render: mode=normal, renders buffer content
```

---

## Unchanged Components

These components require zero modification:
- `alfred-lisp` (bridge primitives already exist for keymap, mode, commands)
- `alfred-plugin` (discovery and loading works with new plugin)
- `alfred-syntax` (activated after file open, existing path)
- `alfred-core::buffer` (used as-is for file loading)
- `alfred-core::cursor` (browser uses its own cursor_index, not Cursor)
- `alfred-core::viewport` (browser uses its own scroll offset if needed)
- `alfred-core::panel` (status bar panel works via existing hooks)
