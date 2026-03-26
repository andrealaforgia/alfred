# Shared Artifacts Registry: Browser File Search

## Artifacts

### browser-current-dir

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- variable `browser-current-dir` |
| Consumers | Header line in browser display, `list-dir` call, `path-join` for file opening, search filter scope |
| Owner | browse-mode plugin |
| Integration Risk | **HIGH** -- if the displayed directory does not match the directory used for `list-dir`, filtered results will not correspond to what the user sees |
| Validation | Verify that the path shown in line 1 of the browser matches the path passed to `list-dir` and `path-join` |

### sidebar-current-dir

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- variable `sidebar-current-dir` |
| Consumers | Sidebar header line, `list-dir` call for sidebar, `path-join` for sidebar file opening, sidebar search filter scope |
| Owner | browse-mode plugin |
| Integration Risk | **HIGH** -- same risk as `browser-current-dir` but for sidebar context |
| Validation | Verify that the path shown in sidebar header matches the path used for sidebar `list-dir` |

### browser-entries

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- variable `browser-entries` (result of `list-dir` + parent-entry prepend) |
| Consumers | Full listing render, search filter input, cursor bounds calculation |
| Owner | browse-mode plugin |
| Integration Risk | **MEDIUM** -- entries must remain consistent between the unfiltered source and the filter function |
| Validation | After Escape from search, the rendered list must match the original `browser-entries` exactly |

### sidebar-entries

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- variable `sidebar-entries` |
| Consumers | Sidebar listing render, sidebar search filter input, sidebar cursor bounds |
| Owner | browse-mode plugin |
| Integration Risk | **MEDIUM** -- same as `browser-entries` for sidebar |
| Validation | After Escape from search in sidebar, the rendered list must match `sidebar-entries` |

### search-query (NEW)

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- new variable `browser-search-query` / `sidebar-search-query` |
| Consumers | Search prompt display (line 2), `str-contains` filter predicate, re-render trigger on keystroke |
| Owner | browse-mode plugin |
| Integration Risk | **HIGH** -- the query shown in the prompt must be identical to the query used for filtering. If they diverge, the user sees a prompt that does not match the displayed results |
| Validation | The string displayed after "/" in the prompt must be the exact string passed to the filter function |

### search-active (NEW)

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- new variable `browser-search-active` / `sidebar-search-active` |
| Consumers | Keymap routing (characters go to query vs navigation), render function (show prompt vs blank line), Escape handler, Enter handler |
| Owner | browse-mode plugin |
| Integration Risk | **HIGH** -- if the active flag is inconsistent with keymap state, characters could be routed to navigation instead of query input (or vice versa) |
| Validation | When search-active is true, the "/" prompt must be visible and character keys must append to query. When false, no prompt and character keys must route to navigation |

### pre-search-cursor (NEW)

| Property | Value |
|----------|-------|
| Source of Truth | `plugins/browse-mode/init.lisp` -- new variable `browser-pre-search-cursor` / `sidebar-pre-search-cursor` |
| Consumers | Escape handler (restore cursor position) |
| Owner | browse-mode plugin |
| Integration Risk | **LOW** -- only consumed in one place (Escape handler) |
| Validation | After Escape, the cursor must be at the same index it was before "/" was pressed |

### filtered-entries (NEW, computed)

| Property | Value |
|----------|-------|
| Source of Truth | Computed at render time from `browser-entries` (or `sidebar-entries`) + `search-query` using `str-contains` + `str-lower` |
| Consumers | Rendered listing during search, cursor bounds during search, Enter handler during search |
| Owner | browse-mode plugin |
| Integration Risk | **MEDIUM** -- the filter result must be recomputed consistently. If cursor navigation uses a different filter result than rendering, the cursor could point to the wrong entry |
| Validation | The entry under the cursor in the rendered list must be the same entry that `Enter` would open |

---

## Integration Checkpoints

### Checkpoint 1: Search Prompt Consistency
- **When**: Every render while search is active
- **Verify**: The string after "/" in the prompt matches `search-query` exactly
- **Risk if broken**: User types "br" but sees "b" in the prompt (or vice versa)

### Checkpoint 2: Filter-Display Consistency
- **When**: Every render while search is active
- **Verify**: The displayed entries are exactly those entries from `browser-entries` whose names contain `search-query` (case-insensitive)
- **Risk if broken**: Entries appear that should not match, or matching entries are hidden

### Checkpoint 3: Cursor-Entry Consistency
- **When**: j/k navigation and Enter in search mode
- **Verify**: The entry the cursor visually points to is the same entry that Enter would open/navigate into
- **Risk if broken**: User sees cursor on `runtime.rs` but Enter opens `server.rs`

### Checkpoint 4: Escape Restores Full State
- **When**: After Escape from search mode
- **Verify**: `browser-entries` is unchanged, cursor position matches `pre-search-cursor`, search prompt is gone, keymap routes characters to navigation
- **Risk if broken**: Entries are missing after Escape, or cursor is at wrong position, or characters still go to search input

### Checkpoint 5: Context Parity
- **When**: Same search query in full-screen browser vs sidebar for same directory
- **Verify**: Identical set of matching entries displayed in both contexts
- **Risk if broken**: User finds a file in full-screen browser but cannot find it in sidebar (or vice versa)
