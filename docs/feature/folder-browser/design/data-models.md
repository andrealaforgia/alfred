# Data Models: Folder Browser

## Overview

All data types live in `alfred-core::browser` (new module). They are pure data with no IO dependencies, following the functional-core / imperative-shell architecture.

---

## Core Types

### EntryKind

Algebraic data type representing the type of a filesystem entry.

```
EntryKind = Directory | File | Symlink(target_kind: EntryKind) | ParentDir
```

- `Directory` -- a directory entry, displayed with trailing `/`
- `File` -- a regular file entry
- `Symlink(target_kind)` -- a symlink; `target_kind` indicates what it points to (Directory or File). For broken symlinks, this could be modeled as a separate variant or a flag.
- `ParentDir` -- the synthetic `../` entry shown at the top of non-root directories

### DirEntry

Represents a single entry in a directory listing. Immutable value type.

```
DirEntry {
    name: String,          -- entry name (filename only, no path)
    kind: EntryKind,       -- type discriminator
    is_hidden: bool,       -- starts with '.' (dotfile)
}
```

**Sorting rule**: Directories before files, both groups sorted case-insensitively alphabetical. `ParentDir` always first.

**Display rules**:
- `ParentDir` -> `"../"`
- `Directory` -> `"{name}/"`
- `File` -> `"{name}"`
- `Symlink(Directory)` -> `"{name}/"` (follows symlink type)
- `Symlink(File)` -> `"{name}"`

### NavigationEntry

A stack entry recording where the user was before entering a subdirectory.

```
NavigationEntry {
    dir: PathBuf,          -- the directory path the user was in
    cursor_index: usize,   -- cursor position within that directory
}
```

### BrowserState

The aggregate state of the folder browser. Owned by `EditorState` as `Option<BrowserState>`.

```
BrowserState {
    root_dir: PathBuf,           -- the original directory Alfred was opened with
    current_dir: PathBuf,        -- the directory currently being browsed
    entries: Vec<DirEntry>,      -- sorted directory entries (directories first, then files)
    cursor_index: usize,         -- index into entries for the highlighted entry
    scroll_offset: usize,        -- first visible entry index (for long listings)
    navigation_history: Vec<NavigationEntry>,  -- stack for parent navigation with cursor restore
    error_message: Option<String>,             -- transient error (permission denied, binary file, etc.)
}
```

**Invariants**:
- `cursor_index < entries.len()` when `entries` is non-empty; `cursor_index == 0` when empty
- `scroll_offset <= cursor_index` (cursor is always visible)
- `scroll_offset + visible_height > cursor_index` (cursor is within viewport)
- `entries` is always sorted: ParentDir first (if present), then directories (alpha), then files (alpha)
- `current_dir` always matches the parent of `entries` (entries are re-read on every directory change)

---

## EditorState Extension

The existing `EditorState` struct gains one new field:

```
EditorState {
    // ... all existing fields unchanged ...
    pub browser: Option<BrowserState>,   -- None when not in browse mode; Some when browsing
}
```

And one new constant:

```
pub const MODE_BROWSE: &str = "browse";
```

**Lifecycle**:
- `browser = None` on normal startup (no argument or file argument)
- `browser = Some(...)` when directory argument detected (set in alfred-bin before event loop)
- `browser = None` after file open transition (cleared when transitioning to normal mode)

---

## Command Model

Browser commands are registered as named commands in the `CommandRegistry`, following the same pattern as existing commands (e.g., `"cursor-down"`, `"enter-insert-mode"`).

| Command Name | Behavior |
|-------------|----------|
| `browser-cursor-down` | Move cursor_index down by 1, clamped to entries.len() - 1 |
| `browser-cursor-up` | Move cursor_index up by 1, clamped to 0 |
| `browser-enter` | If cursor on directory: navigate into it. If cursor on file: open it. If cursor on ParentDir: go to parent |
| `browser-parent` | Navigate to parent directory, restore cursor from history |
| `browser-quit` | Set `state.running = false` to exit Alfred |
| `browser-jump-first` | Set cursor_index to 0 |
| `browser-jump-last` | Set cursor_index to entries.len() - 1 |

**Pure vs IO boundary**: `browser-cursor-down`, `browser-cursor-up`, `browser-jump-first`, `browser-jump-last`, `browser-quit` are purely state transformations. `browser-enter` (on directory or file) and `browser-parent` require filesystem IO -- these return a deferred action that the event loop in alfred-tui fulfills.

---

## Error Model

Errors in browse mode are transient messages, not fatal. They use the existing `state.message` mechanism (shown on the message line at the bottom).

| Error Condition | Message | Behavior |
|----------------|---------|----------|
| Nonexistent path (startup) | `"alfred: no such file or directory: {path}"` | Print to stderr, exit code 1 (before event loop) |
| Permission denied on dir (startup) | `"alfred: permission denied: {path}"` | Print to stderr, exit code 1 |
| Permission denied on subdirectory | `"Permission denied: {dirname}/"` | Set `state.message`, stay in browser |
| Binary file (open attempt) | `"Cannot open binary file: {filename}"` | Set `state.message`, stay in browser |
| Permission denied on file (open) | `"Permission denied: {filename}"` | Set `state.message`, stay in browser |
| Broken symlink (enter) | `"Broken symlink: {name}"` | Set `state.message`, stay in browser |
| Empty directory | No error -- show "Directory is empty" in the content area | Stay in browser, q/h still work |
| IO error reading directory | `"Error reading directory: {dirname}"` | Set `state.message`, stay in browser |

---

## Data Flow Diagram

```
CLI arg "."
    |
    v
[alfred-bin] classify: is_dir() = true
    |
    v
[std::fs::read_dir()] -> raw DirEntry list
    |
    v
[sort & classify] -> Vec<DirEntry> (pure)
    |
    v
BrowserState { current_dir: ".", entries: [...], cursor_index: 0, ... }
    |
    v
EditorState { browser: Some(BrowserState), mode: "browse", ... }
    |
    v
[event loop]
    |-- render: read BrowserState -> draw entries with cursor
    |-- key j -> browser-cursor-down -> BrowserState { cursor_index: +1 }
    |-- key Enter on dir -> read_dir(subdir) -> BrowserState { current_dir: subdir, entries: [...] }
    |-- key Enter on file -> Buffer::from_file() -> EditorState { browser: None, mode: "normal", buffer: file }
```
