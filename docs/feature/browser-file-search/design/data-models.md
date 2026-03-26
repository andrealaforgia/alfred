# Data Models: Browser File Search

## Lisp State Variables

### Browser Filter State (current-dir, full-screen)

| Variable | Type | Default | Purpose |
|----------|------|---------|---------|
| `browser-search-active` | boolean (nil/1) | nil | Whether filter mode is active |
| `browser-search-query` | string | `""` | Current filter query text |
| `browser-pre-search-cursor` | integer | 0 | Cursor position before filter activated |
| `browser-filtered-entries` | list | `()` | Entries matching current query |

### Sidebar Filter State (current-dir, panel)

| Variable | Type | Default | Purpose |
|----------|------|---------|---------|
| `sidebar-search-active` | boolean (nil/1) | nil | Whether filter mode is active |
| `sidebar-search-query` | string | `""` | Current filter query text |
| `sidebar-pre-search-cursor` | integer | 0 | Panel cursor line before filter |
| `sidebar-filtered-entries` | list | `()` | Entries matching current query |

### Project File Search State (recursive, global)

| Variable | Type | Default | Purpose |
|----------|------|---------|---------|
| `project-search-active` | boolean (nil/1) | nil | Whether project search is active |
| `project-search-query` | string | `""` | Current search query text |
| `project-file-cache` | list | `()` | All files from last `list-dir-recursive` |
| `project-search-results` | list | `()` | Filtered subset of cache |
| `project-search-cursor` | integer | 0 | Index into results list |
| `project-search-saved-sidebar` | list | `()` | Previous sidebar-entries for restore |
| `project-search-saved-dir` | string | `""` | Previous sidebar-current-dir for restore |
| `project-search-saved-visible` | boolean | nil | Whether sidebar was visible before search |
| `project-search-saved-mode` | string | `""` | Mode before search (for restore) |
| `project-search-saved-keymap` | string | `""` | Active keymap before search (for restore) |

## Data Structures

### Directory Entry (existing)

Used by `list-dir` and the browser/sidebar. Already defined in `init.lisp`:

```
(name type)
```

- `name`: string, filename only (e.g., `"main.rs"`)
- `type`: string, one of `"file"`, `"dir"`, `"symlink"`

### Recursive Entry (new)

Returned by `list-dir-recursive`:

```
(relative-path type)
```

- `relative-path`: string, path relative to root (e.g., `"src/main.rs"`, `"crates/alfred-core/src/lib.rs"`)
- `type`: string, one of `"file"`, `"dir"`, `"symlink"`

Same shape as directory entry -- the only difference is that `name` becomes `relative-path`. This means shared filter helpers work on both.

### Filtered Result

Same structure as the input list. Filtering produces a sublist -- no transformation needed.

## State Transitions

### Browser Filter

```
IDLE --(/ pressed)--> ACTIVE --(char typed)--> ACTIVE (query updated, entries re-filtered)
ACTIVE --(Escape)--> IDLE (cursor restored)
ACTIVE --(backspace on empty)--> IDLE (cursor restored)
ACTIVE --(Enter on file)--> FILE OPENED (search dismissed, mode changes)
ACTIVE --(Enter on dir)--> DIR ENTERED (search dismissed, dir loaded)
ACTIVE --(Enter on no matches)--> ACTIVE (no-op)
```

### Project File Search

```
ANY_MODE --(Ctrl-p)--> LOADING (call list-dir-recursive, save sidebar state)
LOADING --> SEARCH_ACTIVE (sidebar shows results, message shows query)
SEARCH_ACTIVE --(char typed)--> SEARCH_ACTIVE (query updated, results re-filtered)
SEARCH_ACTIVE --(Escape)--> RESTORED (sidebar state restored, mode restored)
SEARCH_ACTIVE --(Enter on file)--> FILE OPENED (search dismissed, sidebar restored)
SEARCH_ACTIVE --(Enter on no matches)--> SEARCH_ACTIVE (no-op)
```

## Keymap Definitions

### file-search-mode (NEW)

This keymap needs bindings for every printable character (a-z, A-Z, 0-9, punctuation) to route them to `project-search-type-char` instead of navigation. Plus navigation keys.

Characters that serve double duty in file-search-mode:
- `j`, `k` -- navigation, NOT appended to query (matches vim convention in the DISCUSS analysis)
- `g`, `G` -- jump first/last, NOT appended to query

**Alternative considered**: j/k as literal text input. Rejected because the DISCUSS analysis and user stories explicitly show j/k for navigating filtered results, which is the vim-native expectation.

Characters that ARE query input: all printable chars except j, k, g, G.

**Note**: this means you cannot search for filenames containing only "j", "k", "g", or "G" characters. This is acceptable given the substring-match approach -- any other character in the path will work.

### Keybinding registration pattern

```
;; Illustrative (crafter decides actual structure):
;; (define-key "file-search-mode" "Char:a" "project-search-char-a")
;; ... repeated for all printable chars except j/k/g/G
;; (define-key "file-search-mode" "Char:j" "project-search-cursor-down")
;; (define-key "file-search-mode" "Char:k" "project-search-cursor-up")
;; (define-key "file-search-mode" "Enter" "project-search-open")
;; (define-key "file-search-mode" "Escape" "project-search-dismiss")
;; (define-key "file-search-mode" "Backspace" "project-search-backspace")
```

The crafter will determine the most efficient implementation pattern (individual commands per character vs. a single command that reads the last key event).

## Panel Layout During Project Search

```
Line 0: " Search results"          (header, styled blue)
Line 1: ""                          (separator)
Line 2: " > src/main.rs"           (first result, cursor here)
Line 3: "   src/lib.rs"            (second result)
Line 4: "   crates/core/src/lib.rs" (third result)
...
Line N: "   plugins/browse/init.lisp" (last visible result)

Message line (bottom): "Search: que|"  (query with implied cursor)
```

When no matches:
```
Line 0: " Search results"
Line 1: ""
Line 2: "   (no matches)"

Message line: "Search: xyz|"
```
