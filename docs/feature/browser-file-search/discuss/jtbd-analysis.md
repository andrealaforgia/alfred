# JTBD Analysis: Browser File Search

## Job Classification

**Job Type**: Build Something New (Greenfield feature on Brownfield codebase)
**Workflow**: `[research] -> discuss -> design -> distill -> baseline -> roadmap -> split -> execute -> review`
**Current Phase**: DISCUSS (requirements gathering)

Alfred's folder browser exists (brownfield), but file search within it is a new capability (greenfield feature). Discovery is required because:
- The feature introduces a new interaction mode within the browser (text input overlay on navigation mode)
- It changes the browser's rendering pipeline (full list vs filtered list)
- It must work identically in two distinct contexts (full-screen browser and sidebar panel)
- It bridges two paradigms: vim-style modal navigation and incremental text input

---

## Job Stories

### JS-BFS-01: Find a Known File in a Large Directory

**When** I am browsing a project directory in Alfred that has many files and I know the approximate name of the file I need,
**I want to** type a few characters to narrow the visible entries to those matching my query,
**so I can** jump to the right file in seconds instead of scrolling through dozens of entries with j/k.

#### Functional Job
Filter the browser's visible entries by partial name match, reducing a long list to a handful of candidates.

#### Emotional Job
Feel efficient and in control -- the browser keeps pace with my intent rather than forcing tedious scrolling. The experience should feel snappy, like a fuzzy finder.

#### Social Job
Demonstrate fluency with the editor -- file navigation that looks effortless to pair programming partners or screen-share audiences.

---

### JS-BFS-02: Locate a File When the Exact Name Is Uncertain

**When** I am browsing a project directory and I can only remember part of a filename (e.g., "bridge" or "input"),
**I want to** type that fragment and see all files whose names contain it,
**so I can** recognize the correct file visually without guessing the full name.

#### Functional Job
Perform substring matching against entry names in the current directory, surfacing partial matches.

#### Emotional Job
Feel supported rather than stuck -- the tool helps me find what I half-remember instead of punishing me for forgetting.

#### Social Job
Avoid the embarrassment of asking a colleague "what's that file called again?" when pair programming.

---

### JS-BFS-03: Return to Full Listing After a Search

**When** I have filtered the browser entries with a search and either found what I need or want to start over,
**I want to** clear the search and see the full directory listing again,
**so I can** continue browsing without losing my place or having to re-enter the directory.

#### Functional Job
Dismiss the search filter and restore the complete entry list for the current directory.

#### Emotional Job
Feel safe experimenting with search -- I can always get back to where I was. No fear of getting trapped in a filtered view.

#### Social Job
Not applicable (internal action).

---

## 8-Step Universal Job Map

### Step 1: DEFINE -- Decide to search

| Aspect | Detail |
|--------|--------|
| User's goal | Find a specific file in the current directory listing |
| Information needed | Current directory contents visible; user recalls part of the filename |
| Decision | Search visually (scroll) or invoke search filter |
| Missing requirement risk | How does the user know search is available? Discoverability of the `/` key |

### Step 2: LOCATE -- Enter search mode

| Aspect | Detail |
|--------|--------|
| User's action | Press `/` to activate search input |
| Information needed | Visual indicator that search mode is active (search prompt visible) |
| Decision | None -- single key press |
| Missing requirement risk | `/` is already used for text search in normal mode; conflict in browser context? No -- browser has its own keymap |

### Step 3: PREPARE -- Type the query

| Aspect | Detail |
|--------|--------|
| User's action | Type characters incrementally; list filters in real time |
| Information needed | Current query displayed; matching entries highlighted; non-matching entries hidden |
| Decision | Keep typing to narrow further, or stop when candidate visible |
| Missing requirement risk | Should matching be case-sensitive or case-insensitive? What about entries with special characters? |

### Step 4: CONFIRM -- Identify the target

| Aspect | Detail |
|--------|--------|
| User's action | Visually scan the filtered list; use j/k to position cursor on the desired entry |
| Information needed | Filtered entry count; cursor position; which part of the name matched |
| Decision | This is the right file -- press Enter |
| Missing requirement risk | What if the filtered list is empty (no matches)? What feedback does the user see? |

### Step 5: EXECUTE -- Open the file

| Aspect | Detail |
|--------|--------|
| User's action | Press Enter to open the selected file (or enter directory) |
| Information needed | Full file path constructed from current-dir + entry name |
| Decision | None -- same as normal browser Enter behavior |
| Missing requirement risk | Should search mode be dismissed automatically when Enter is pressed? |

### Step 6: MONITOR -- Verify the result

| Aspect | Detail |
|--------|--------|
| User's action | See the file open in the editor buffer (or new directory listing if directory) |
| Information needed | Filename in status bar confirms correct file |
| Decision | Correct file? If not, return to browser |
| Missing requirement risk | If user entered a directory during search, should search query carry over? |

### Step 7: MODIFY -- Correct course

| Aspect | Detail |
|--------|--------|
| User's action | Press Escape to cancel search and return to full listing; or press Backspace to edit query |
| Information needed | Full listing restored; cursor returns to reasonable position |
| Decision | Refine query or abandon search |
| Missing requirement risk | Does Escape clear the query only, or exit browser entirely? Should be clear-query-only |

### Step 8: CONCLUDE -- Resume browsing or editing

| Aspect | Detail |
|--------|--------|
| User's action | File is open for editing, or user is browsing the full listing again |
| Information needed | Clean state -- no residual search filter obscuring entries |
| Decision | None |
| Missing requirement risk | If search was dismissed, is browser cursor position preserved or reset? |

---

## Four Forces Analysis

### Demand-Generating Forces

**Push (Frustration with current situation)**
- In a directory with 30+ files, finding a specific file requires pressing `j` or `k` repeatedly -- tedious and error-prone
- The browser has no shortcut for "jump to file starting with X" -- every navigation is sequential
- Developers accustomed to fuzzy finders (fzf, Telescope, Ctrl-P) find sequential j/k browsing frustratingly slow
- The sidebar (30-char width) can display only a fraction of entries at once, making scrolling even worse
- In deeply nested projects, returning to the browser and scrolling to a known file after opening the wrong one wastes time

**Pull (Attractiveness of new solution)**
- Type 2-3 characters and the list collapses from 40 entries to 2-3 candidates -- near-instant file finding
- Familiar interaction pattern: vim's `/` for search, fzf's type-to-filter, VS Code's Ctrl-P
- Works identically in both full-screen browser and sidebar -- one mental model, two contexts
- Substring matching handles partial recall ("bridge" finds "bridge.rs" and "bridge_helpers.rs")
- Implementable entirely in Lisp plugin using existing primitives (`str-contains`, `str-lower`)

### Demand-Reducing Forces

**Anxiety (Fears about the new solution)**
- Will typing characters in the browser accidentally trigger navigation commands (j/k/l/h)?
- Will the search input feel laggy with large directories?
- What if I accidentally enter search mode and can't figure out how to exit?
- Will the search break the existing j/k navigation flow?

**Habit (Inertia of current approach)**
- Developers who know their project structure well can navigate by j/k muscle memory in a few keystrokes
- Some users never search -- they always know the file is "3 entries down"
- Existing workflow of navigating directories manually is functional, if slow
- The `g`/`G` keys (jump to first/last) partially mitigate long lists

### Assessment

| Dimension | Rating |
|-----------|--------|
| Switch likelihood | **High** -- any developer who has used a fuzzy finder will immediately look for search in a browser |
| Key blocker | Anxiety about mode confusion: pressing `/` must feel safe and reversible. Escape must reliably return to normal browsing |
| Key enabler | Push from tedious j/k scrolling in large directories; Pull from familiar type-to-filter pattern |
| Design implication | Search must be a clean overlay on existing navigation. `/` enters search mode, typing filters incrementally, Escape cancels completely. No mode confusion -- the search prompt must be visually distinct. Filter must be instant (no perceptible delay). Empty results must show a clear message, not a confusing blank screen |

---

## Outcome Statements

| ID | Outcome Statement | Priority |
|----|-------------------|----------|
| OS-BFS-01 | Minimize the time it takes to locate a specific file in a directory with many entries | Must Have |
| OS-BFS-02 | Minimize the number of keystrokes required to narrow the listing to the target file | Must Have |
| OS-BFS-03 | Minimize the likelihood of getting stuck in search mode with no way to return to full listing | Must Have |
| OS-BFS-04 | Minimize the cognitive load of switching between navigation mode and search mode | Must Have |
| OS-BFS-05 | Maximize the likelihood that the search interaction feels familiar to fuzzy-finder users | Should Have |
| OS-BFS-06 | Minimize the time to recover from an unsuccessful search (no matches) | Should Have |
| OS-BFS-07 | Maximize the consistency of search behavior between full-screen browser and sidebar | Must Have |

---

## Persona

**Kai Nakamura** -- Backend developer, 4 years experience (same persona as folder-browser feature). Uses Alfred as his primary terminal editor. Comfortable with vim keybindings and modal editing. Works across 3-4 Rust and Python projects, some with 50+ files per directory (e.g., `src/` directories with many modules). Has used fzf, Telescope, and VS Code's Ctrl-P extensively. Finds Alfred's browser useful but slow for large directories -- currently scrolls with j/k or mentally counts lines. Expects `/` to activate search because that's how vim search works.

---

## Design Decisions (Pre-Resolved)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Search activation key | `/` | Matches vim convention; already free in browse-mode and filetree-mode keymaps |
| Matching strategy | Case-insensitive substring | Simpler to implement with `str-contains` + `str-lower`; sufficient for filename search |
| Search scope | Current directory only (not recursive) | Matches what the browser displays; recursive search is a different feature (file finder) |
| Behavior on Enter | Open file/enter dir AND dismiss search | Natural: user found what they wanted |
| Behavior on Escape | Dismiss search, restore full listing | Safe exit; matches vim pattern |
| Backspace behavior | Delete last character of query; if query empty, dismiss search | Standard text input behavior |
