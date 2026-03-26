# Component Boundaries: Browser File Search

## Boundary Map

```
+-------------------------------------------------------------------+
|                        browse-mode/init.lisp                       |
|                           (Lisp plugin)                            |
|                                                                    |
|  +------------------+  +------------------+  +------------------+  |
|  | Browser Filter   |  | Sidebar Filter   |  | Project Search   |  |
|  | (current-dir)    |  | (current-dir)    |  | (recursive)      |  |
|  |                  |  |                  |  |                  |  |
|  | State:           |  | State:           |  | State:           |  |
|  |  search-active   |  |  search-active   |  |  search-active   |  |
|  |  search-query    |  |  search-query    |  |  search-query    |  |
|  |  pre-search-     |  |  pre-search-     |  |  file-cache      |  |
|  |   cursor         |  |   cursor         |  |  saved-sidebar-  |  |
|  |  filtered-       |  |  filtered-       |  |   state          |  |
|  |   entries        |  |   entries        |  |  filtered-       |  |
|  +--------+---------+  +--------+---------+  |   results        |  |
|           |                      |            +--------+---------+  |
|           +----------+-----------+                     |            |
|                      |                                 |            |
|           +----------v-----------+                     |            |
|           | Shared Helpers       |                     |            |
|           |                      |                     |            |
|           | search-filter-list   |<--------------------+            |
|           | search-append-char   |                                  |
|           | search-backspace     |                                  |
|           | search-dismiss       |                                  |
|           +----------------------+                                  |
+-------------------------------------------------------------------+
                            |
                  calls Lisp primitives
                            |
+-------------------------------------------------------------------+
|                   alfred-lisp/bridge.rs                             |
|                      (Rust bridge)                                  |
|                                                                    |
|  Existing:                        New:                             |
|  - list-dir                       - list-dir-recursive             |
|  - str-contains                                                    |
|  - str-lower                                                       |
|  - path-join                                                       |
|  - open-file                                                       |
|  - set-panel-line                                                  |
|  - focus-panel / unfocus-panel                                     |
|  - panel-cursor-up / down                                          |
|  - message                                                         |
+-------------------------------------------------------------------+
                            |
                    mutates / reads
                            |
+-------------------------------------------------------------------+
|                   alfred-core                                       |
|                                                                    |
|  EditorState (unchanged)    PanelRegistry (unchanged)              |
|  - panels                   - lines HashMap                        |
|  - focused_panel            - cursor_line                          |
|  - message                  - line_styles                          |
|  - keymaps                                                         |
|  - mode                                                            |
+-------------------------------------------------------------------+
```

## Component Responsibilities

### Browser Filter (Lisp)
- **Owns**: current-directory search state for full-screen browser
- **Trigger**: `/` key in browse-mode
- **Data source**: `browser-entries` (already loaded)
- **Renders to**: buffer via `buffer-set-content`
- **Does not**: touch panel system, know about recursive search

### Sidebar Filter (Lisp)
- **Owns**: current-directory search state for sidebar panel
- **Trigger**: `/` key in filetree-mode
- **Data source**: `sidebar-entries` (already loaded)
- **Renders to**: panel lines via `set-panel-line`
- **Does not**: touch buffer, know about recursive search

### Project File Search (Lisp)
- **Owns**: project-wide search state, file cache, result rendering
- **Trigger**: `Ctrl-p` from any mode
- **Data source**: `list-dir-recursive` result (cached)
- **Renders to**: panel lines via `set-panel-line` + message line via `message`
- **Does not**: modify browser-entries or sidebar-entries directly

### Shared Helpers (Lisp)
- **Owns**: reusable search logic shared by all three search components
- **Functions**: filter a list by substring, append/delete char from query, dismiss search state
- **Does not**: own state, know about rendering target (buffer vs panel)

### list-dir-recursive (Rust)
- **Owns**: recursive filesystem traversal
- **Input**: root path string
- **Output**: flat Lisp list of `(relative-path type)` pairs
- **Does not**: cache results, filter results, know about panels or UI
- **Boundary rule**: pure function from path to list. No editor state mutation.

## Dependency Direction

```
Browser Filter  ---\
Sidebar Filter  ----+---> Shared Helpers ---> Bridge Primitives ---> Core State
Project Search  ---/                     \--> Filesystem (read-only)
```

All dependencies point inward (toward Rust). No Rust component depends on or knows about any Lisp component. This preserves the plugin-first boundary established in ADR-002.

## What Changes Where

| Layer | File | Change Type |
|-------|------|-------------|
| Rust bridge | `crates/alfred-lisp/src/bridge.rs` | Add `register_list_dir_recursive` function |
| Lisp plugin | `plugins/browse-mode/init.lisp` | Add filter logic + project search + keymaps |
| Core | (none) | No changes |
| TUI | (none) | No changes |
| Bin | (none) | No changes |

## Integration Points

1. **list-dir-recursive <-> Lisp**: new primitive registered alongside existing filesystem primitives
2. **Project search <-> Sidebar**: project search temporarily takes over the filetree panel content. On dismiss, it restores the previous sidebar state (entries + directory + cursor).
3. **Ctrl-p <-> Mode system**: `Ctrl-p` bound in multiple keymaps (normal-mode, browse-mode). Each binding calls the same command `"project-file-search"`.
4. **file-search-mode <-> keymap system**: new keymap with text-input character bindings (`Char:a` through `Char:z`, etc.) plus navigation keys.
