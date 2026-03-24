# Shared Artifacts Registry: Folder Browser

## Artifact Inventory

### target_path

| Property | Value |
|----------|-------|
| Source of truth | CLI `argv[1]` parsed in `alfred-bin/src/main.rs` |
| Consumers | Step 1: directory detection logic; Step 2: initial `current_dir` |
| Owner | alfred-bin (CLI entry point) |
| Integration risk | **HIGH** -- if the path is not correctly classified as directory vs file, the wrong mode activates |
| Validation | Integration test: `alfred .` enters browser; `alfred file.rs` enters editor |

### current_dir

| Property | Value |
|----------|-------|
| Source of truth | Browser state (initially `target_path`, updated on directory navigation) |
| Consumers | Step 2: title bar display; Step 2: status bar display; Step 3: entry listing source; Step 3: parent navigation target |
| Owner | Browser mode state (new state added to EditorState or separate BrowserState) |
| Integration risk | **HIGH** -- must update atomically with `dir_entries` when navigating |
| Validation | After every Enter on a directory, `current_dir` matches the rendered entries' parent |

### dir_entries

| Property | Value |
|----------|-------|
| Source of truth | `std::fs::read_dir(current_dir)` filtered and sorted |
| Consumers | Step 2: rendered listing; Step 3: cursor bounds; Step 4: file path resolution |
| Owner | Browser mode state |
| Integration risk | **HIGH** -- stale entries (if filesystem changes) could cause Enter to open wrong file |
| Validation | Entries re-read from filesystem on every directory change; cursor clamped to valid range |

### cursor_index

| Property | Value |
|----------|-------|
| Source of truth | Browser state, updated by j/k/gg/G |
| Consumers | Step 2: visual highlight (> prefix); Step 3: navigation bounds; Step 4: determines `selected_file_path` |
| Owner | Browser mode state |
| Integration risk | **MEDIUM** -- index must stay within `0..dir_entries.len()` |
| Validation | After every navigation key, `cursor_index < dir_entries.len()` (or 0 if empty) |

### navigation_history

| Property | Value |
|----------|-------|
| Source of truth | Browser state, stack of `(PathBuf, usize)` tuples |
| Consumers | Step 3: cursor restoration when navigating to parent |
| Owner | Browser mode state |
| Integration risk | **LOW** -- failure means cursor resets to 0 (annoying but not breaking) |
| Validation | After h/Backspace, cursor lands on the directory entry the user came from |

### selected_file_path

| Property | Value |
|----------|-------|
| Source of truth | `current_dir.join(dir_entries[cursor_index].name)` computed at selection time |
| Consumers | Step 4: `Buffer::from_file()`; Step 4: status bar filename |
| Owner | Derived at point of use (not stored) |
| Integration risk | **HIGH** -- must be resolved to canonical path to handle symlinks correctly |
| Validation | After opening, `buffer.file_path()` matches `selected_file_path` |

### mode_indicator

| Property | Value |
|----------|-------|
| Source of truth | `EditorState.mode` |
| Consumers | Status bar display; keybinding dispatch |
| Owner | alfred-core EditorState |
| Integration risk | **HIGH** -- browser mode must use a distinct mode string (e.g., "browse") so keymaps dispatch correctly |
| Validation | In browser mode, `state.mode == "browse"`; after opening file, `state.mode == "normal"` |

---

## Integration Checkpoints

### Checkpoint 1: CLI Argument Classification

| Aspect | Requirement |
|--------|-------------|
| When | Alfred starts with an argument |
| Validates | `target_path` correctly classified as file, directory, or nonexistent |
| Failure mode | Directory opens as empty buffer; file triggers browser |
| Test | `Path::new(arg).is_dir()` before `Buffer::from_file()` |

### Checkpoint 2: Browser Mode Activation

| Aspect | Requirement |
|--------|-------------|
| When | Directory argument detected |
| Validates | `state.mode` set to "browse"; browser keymap activated; browser state initialized |
| Failure mode | Normal mode keybindings active in browser (j inserts character) |
| Test | After `alfred .`, `state.mode == "browse"` and `state.active_keymaps` includes "browse" |

### Checkpoint 3: Directory Entry Consistency

| Aspect | Requirement |
|--------|-------------|
| When | Browser displays or navigates |
| Validates | `dir_entries` matches filesystem; `cursor_index` within bounds |
| Failure mode | Stale entries; index out of bounds panic |
| Test | After navigation, `cursor_index < dir_entries.len()` |

### Checkpoint 4: File Opening Transition

| Aspect | Requirement |
|--------|-------------|
| When | User presses Enter on a file entry |
| Validates | `selected_file_path` loaded into buffer; mode switches to "normal"; browser state cleared; all editor features active (gutter, syntax, status bar) |
| Failure mode | Buffer empty; mode stuck in "browse"; panels not rendering |
| Test | After opening file, `buffer.line_count() > 0` and `state.mode == "normal"` |

### Checkpoint 5: CLI Vocabulary Consistency

| Aspect | Requirement |
|--------|-------------|
| When | Throughout the feature |
| Validates | Mode is called "BROWSE" (uppercase in status bar, lowercase in state); key bindings documented consistently |
| Failure mode | Status bar says "BROWSER" while mode string is "browse" |
| Test | Status bar rendering uses consistent formatting |

---

## Horizontal Coherence Checks

| Check | Status | Notes |
|-------|--------|-------|
| CLI vocabulary consistent | OK | "browse" mode, "BROWSE" in status bar, consistent with NORMAL/INSERT/VISUAL pattern |
| Emotional arc smooth | OK | Curious -> Oriented -> Confident -> Productive, no jarring transitions |
| Shared artifacts have single source | OK | Each artifact has one source of truth documented above |
| Key bindings do not conflict | VERIFY | j/k/h/l/gg/G in browse mode must not conflict with existing normal mode if both keymaps are ever active simultaneously. Mitigation: browse mode has its own keymap, not layered on normal keymap. |
| Panel system integration | OK | Browser can use existing panel system for status bar; tree view renders in the main content area (not a panel) |
| Plugin-first alignment | VERIFY | Initial implementation may be in Rust core; should be designed so a future Lisp plugin could replace or extend it. Expose Lisp primitives for directory listing and browser state. |
