# ADR-009: File Search Architecture

## Status

Proposed

## Context

Alfred's folder browser (`browse-mode` plugin) supports directory listing with j/k navigation but has no search capability. Users need two related abilities:

1. **Current-directory filter**: press `/` while browsing to narrow visible entries by substring (vim-style)
2. **Project-wide file search**: press `Ctrl-p` from any mode to search the entire project tree (VS Code-style)

Both features must integrate with the existing Lisp plugin system, panel system, and keymap system. The browser is implemented entirely in Lisp (`plugins/browse-mode/init.lisp`) with Rust bridge primitives for filesystem access and panel manipulation.

**Quality attribute priorities**:
- Responsiveness: HIGH -- search must feel instant for projects up to ~5000 files
- Maintainability: HIGH -- implementable as Lisp plugin with minimal Rust changes
- Plugin-first: core must have zero knowledge of file search

**Constraints**:
- Functional-core / imperative-shell architecture
- rust_lisp limitations: no empty strings, no tail-call optimization, no define-inside-begin-lambda
- Existing `list-dir` primitive lists only a single directory (not recursive)
- Existing panel system supports left panels with focus, cursor, per-line content and styles

## Decision

Implement file search as a **Lisp plugin extension** with **one new Rust primitive** (`list-dir-recursive`).

### Current-directory filter
- Implemented entirely in Lisp within `browse-mode/init.lisp`
- Uses existing `str-contains` and `str-lower` primitives for case-insensitive substring matching
- Activates via `/` key in browse-mode and filetree-mode keymaps
- Filters existing `browser-entries` / `sidebar-entries` lists in-place
- No new Rust code required

### Project-wide file search
- One new Rust bridge primitive: `(list-dir-recursive path)` returning flat list of `(relative-path type)` pairs
- All search UI logic (keymap, filtering, rendering, state management) in Lisp
- Reuses existing `"filetree"` panel for result display (no new panel)
- Search query displayed on the message line
- New `file-search-mode` keymap for text input + result navigation
- `Ctrl-p` keybinding registered in normal-mode and browse-mode

## Alternatives Considered

### Alternative 1: Recursive Directory Walking in Lisp

**Description**: Use repeated `(list-dir path)` + `(is-dir? entry)` calls in Lisp to recursively walk the project tree. No new Rust primitive needed.

**Evaluation**:
- (+) Zero Rust changes
- (+) Fully aligned with plugin-first philosophy
- (-) rust_lisp has no tail-call optimization; deep project trees (10+ levels) risk stack overflow
- (-) Each `list-dir` call crosses the Rust-Lisp bridge (~0.1ms). A project with 200 directories = 200+ bridge calls = 20ms+ just for directory reads, plus Lisp recursion/cons overhead
- (-) Must build the recursive walker in Lisp working around no-define-in-lambda limitation
- (-) A single Rust `std::fs` walk of 5000 files takes <5ms total

**Rejection rationale**: Performance (4x-10x slower) and stack safety make this unacceptable for projects with hundreds of directories. The filesystem boundary is exactly where a Rust primitive belongs -- IO at the shell, logic in the plugin.

### Alternative 2: New Dedicated Panel for Search Results

**Description**: Create a new panel (e.g., `"search-results"`) separate from the sidebar `"filetree"` panel.

**Evaluation**:
- (+) No interference with sidebar state
- (+) Could display simultaneously with sidebar
- (-) Two left panels competing for horizontal space in a terminal editor (30 chars each = 60 chars gone from a typical 120-char terminal)
- (-) Additional panel management complexity (create, destroy, visibility toggling)
- (-) User explicitly said "maybe reusing the sidebar panel" -- aligns with reuse
- (-) The sidebar is conceptually "file navigation"; search is file navigation

**Rejection rationale**: UI real estate cost is prohibitive. The sidebar already serves the file-navigation purpose. Temporarily repurposing it for search results is simpler and matches user expectation.

### Alternative 3: Search Results in the Main Buffer

**Description**: Display search results in the editor buffer (like browse-mode does for directory listings).

**Evaluation**:
- (+) Full buffer width for displaying paths
- (+) Consistent with how browse-mode already renders
- (-) Replaces the current buffer content (user loses their editing context)
- (-) Returning from search requires re-opening the previous file
- (-) Users expect `Ctrl-p` to overlay, not replace their editing view
- (-) The sidebar approach preserves editing context while showing results alongside

**Rejection rationale**: Destroying the editing context for a search is hostile UX. The sidebar/panel approach keeps the editor buffer visible while showing results.

## Consequences

### Positive
- One new Rust primitive covers the entire IO need; all logic stays in Lisp
- Reusing the filetree panel avoids UI complexity and saves screen space
- `Ctrl-p` works from any mode -- the feature is globally accessible
- Current-directory filter (`/`) requires zero Rust changes
- Both search types share filtering logic (same `str-contains` + `str-lower` pattern)
- File cache strategy (re-walk on each Ctrl-p activation) is simple and avoids stale results

### Negative
- `list-dir-recursive` adds one more bridge primitive to maintain
- The filetree panel is temporarily unavailable during project search (sidebar shows search results, not directory listing)
- Restoring sidebar state after search dismiss adds complexity to the plugin
- `file-search-mode` keymap requires many character bindings (a-z, A-Z, 0-9, punctuation)
- j/k/g/G are reserved for navigation in search mode, so searches for paths containing only those characters would fail (extremely unlikely edge case)

### Neutral
- Hidden directory filtering (`.git`, `.target`) in `list-dir-recursive` is a reasonable default. Future enhancement could make it configurable via a Lisp variable.
- The message line is shared with other features (`message` primitive). During search, the message line shows the query. Other messages are suppressed until search ends.
