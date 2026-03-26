# User Stories: Browser File Search

## Story Map

```
Workflow: [Enter Search]  -->  [Filter Entries]  -->  [Act on Result]  -->  [Dismiss Search]
              |                      |                      |                     |
Row 1:    / activates           Substring filter       Enter opens file      Esc restores
          search mode           on each keystroke      or enters dir         full listing
          (US-BFS-01)           (US-BFS-02)            (US-BFS-03)           (US-BFS-04)
              |                      |                      |
Row 2:    Sidebar parity        No-match message       Sidebar open+unfocus
          (US-BFS-05)           (US-BFS-02)            (US-BFS-05)
```

Row 1 = MVP stories (each independently demonstrable). Row 2 = included within the stories above.

---

## US-BFS-01: Enter Search Mode in Browser

### Problem
Kai Nakamura is a backend developer who browses project directories in Alfred's full-screen browser. When he is in a directory with 20+ files, he has to press `j` or `k` repeatedly to reach the file he wants. There is no way to jump directly to a file by name. He wastes 5-10 seconds scrolling through entries he does not care about.

### Who
- Terminal-native developer | Browsing a large directory in Alfred | Wants to start a search rather than scrolling

### Solution
When in the full-screen browser (browse-mode), pressing `/` activates a search mode: a search prompt appears on line 2, and subsequent keystrokes are captured as search query input rather than navigation commands.

### Traces to Jobs
- JS-BFS-01: Find a Known File in a Large Directory
- JS-BFS-02: Locate a File When the Exact Name Is Uncertain

### Domain Examples

#### 1: Happy Path -- Activate search in full-screen browser
Kai is browsing `/home/kai/projects/webapi/src` (23 entries) in the full-screen browser. He presses `/`. The blank line between the header and the entry list is replaced by a search prompt showing just `/`. The full entry list remains visible below. Kai's cursor position is saved internally so it can be restored if he cancels the search.

#### 2: Edge Case -- Press / when directory has only one entry
Kai is browsing `/home/kai/projects/webapi/src/tests` which contains only `mod.rs`. He presses `/`. The search prompt appears. Search still works (filtering a single-entry list), though it is not particularly useful. No error occurs.

#### 3: Edge Case -- Press / when directory is empty
Kai is browsing an empty directory that shows only `..` (parent navigation entry). He presses `/`. The search prompt appears. The `..` entry is included in the searchable list. Typing "." keeps `..` visible.

### UAT Scenarios (BDD)

#### Scenario 1: Activate search mode from full-screen browser
```gherkin
Given Kai is browsing /home/kai/projects/webapi/src in the full-screen browser
And the directory listing shows 23 entries with the cursor on entry 5
When Kai presses /
Then a search prompt "/" appears on line 2 of the display
And the full directory listing remains visible below the prompt
And the cursor position 5 is saved for potential restoration
```

#### Scenario 2: Search mode with single-entry directory
```gherkin
Given Kai is browsing /home/kai/projects/webapi/src/tests in the full-screen browser
And the directory listing shows only "mod.rs"
When Kai presses /
Then the search prompt "/" appears
And "mod.rs" remains visible
```

#### Scenario 3: Search mode with empty directory (only parent entry)
```gherkin
Given Kai is browsing an empty directory showing only the ".." parent entry
When Kai presses /
Then the search prompt "/" appears
And the ".." entry remains visible
```

#### Scenario 4: Characters route to query input, not navigation
```gherkin
Given Kai has activated search mode by pressing /
When Kai types the character "j"
Then the search query becomes "j"
And the character is NOT interpreted as cursor-down navigation
```

### Acceptance Criteria
- [ ] Pressing `/` in browse-mode activates search mode and displays a `/` prompt on line 2
- [ ] The full directory listing remains visible until a character is typed
- [ ] The cursor position before search is saved for restoration on Escape
- [ ] Character keystrokes in search mode are routed to the search query, not to navigation commands
- [ ] Search mode activation works in directories with 0, 1, or many entries

### Technical Notes
- Requires new state variables: `browser-search-active`, `browser-search-query`, `browser-pre-search-cursor`
- The `/` key binding must be added to the `browse-mode` keymap: `(define-key "browse-mode" "Char:/" "browser-search-start")`
- Character input routing requires a mechanism to capture arbitrary character keys (may need a new keymap or integration with existing input handling)
- No Rust changes needed if character input can be handled via Lisp keymap bindings for each printable character, OR if a "text input capture" primitive exists

### Dependencies
- Existing browse-mode plugin (implemented, US-FB-01 through US-FB-05)
- Keymap system must support routing character input to a command (verified: `Char:x` format exists)

---

## US-BFS-02: Filter Browser Entries by Substring Match

### Problem
Kai Nakamura has activated search mode in the browser (pressing `/`) and now wants to narrow the visible entries by typing part of a filename. Without live filtering, the search prompt would be useless -- he would still be staring at the full list.

### Who
- Terminal-native developer | In search mode within Alfred's browser | Wants to see the list narrow as he types

### Solution
Each character typed in search mode is appended to the search query. On every keystroke, the displayed entries are filtered to only those whose names contain the query string (case-insensitive substring match). The cursor resets to the first matching entry. Backspace removes the last character and re-filters. The search prompt displays the current query.

### Traces to Jobs
- JS-BFS-01: Find a Known File in a Large Directory
- JS-BFS-02: Locate a File When the Exact Name Is Uncertain

### Domain Examples

#### 1: Happy Path -- Type "run" to find runtime.rs
Kai is in search mode in `/home/kai/projects/webapi/src` (23 entries). He types `r` -- the list filters to 10 entries (all containing "r"). He types `u` -- the list filters to 1 entry (`runtime.rs`). He types `n` -- still 1 entry. The prompt shows `/ run`. The cursor is on `runtime.rs`.

#### 2: Happy Path -- Type "br" to find bridge files
Kai types `b` -- 2 entries (`bridge.rs`, `bridge_helpers.rs`). Types `r` -- still 2 entries. The prompt shows `/ br`. The cursor is on `bridge.rs`. He can press `j` to move to `bridge_helpers.rs`.

#### 3: Edge Case -- No matches
Kai types `xyz`. The list is empty. Instead of a blank area, the message "(no matches)" is displayed. Kai can press Backspace to shorten the query, or Escape to dismiss search entirely.

#### 4: Edge Case -- Backspace restores broader results
Kai has typed `bri` (2 matches). He presses Backspace. The query becomes `br` (still 2 matches). He presses Backspace again. Query becomes `b`. More entries appear. Each Backspace triggers a re-filter.

#### 5: Edge Case -- Case-insensitive matching
The directory contains `README.md`. Kai types `readme` (all lowercase). `README.md` appears in the filtered results because matching is case-insensitive.

### UAT Scenarios (BDD)

#### Scenario 1: Typing filters entries incrementally
```gherkin
Given Kai is in search mode in the full-screen browser showing /home/kai/projects/webapi/src
When Kai types "run"
Then the search prompt shows "/ run"
And only "runtime.rs" is visible in the listing
And the cursor is on "runtime.rs"
```

#### Scenario 2: Multiple matches navigable with j/k
```gherkin
Given Kai is in search mode with the query "br"
And the filtered list shows "bridge.rs" and "bridge_helpers.rs"
And the cursor is on "bridge.rs"
When Kai presses j
Then the cursor moves to "bridge_helpers.rs"
```

#### Scenario 3: No matches shows informative message
```gherkin
Given Kai is in search mode in the full-screen browser
When Kai types "xyz"
Then the listing area shows "(no matches)"
And no entry is selectable
```

#### Scenario 4: Backspace shortens query and re-filters
```gherkin
Given Kai is in search mode with the query "bri"
And the filtered list shows "bridge.rs" and "bridge_helpers.rs"
When Kai presses Backspace
Then the search query becomes "br"
And the filtered list still shows "bridge.rs" and "bridge_helpers.rs"
```

#### Scenario 5: Case-insensitive matching
```gherkin
Given Kai is in search mode in the full-screen browser
And the directory contains "README.md"
When Kai types "readme"
Then "README.md" is visible in the filtered list
```

### Acceptance Criteria
- [ ] Each character typed appends to the query and triggers an incremental re-filter of entries
- [ ] Filtering uses case-insensitive substring matching (via `str-contains` on lowercased strings)
- [ ] The search prompt displays `/ {query}` on line 2, updated on every keystroke
- [ ] The cursor resets to the first matching entry after each filter
- [ ] When no entries match, "(no matches)" is displayed instead of a blank area
- [ ] Backspace removes the last character from the query and triggers re-filter
- [ ] j/k navigation works within the filtered entry list
- [ ] Directory entries are included in filtering (not only files)

### Technical Notes
- Filter implementation: iterate `browser-entries`, keep those where `(str-contains (str-lower (first entry)) (str-lower browser-search-query))` returns true
- The `.." entry should also be filterable (typing ".." keeps it visible)
- Cursor bounds must be computed from filtered list length, not full list length
- Available primitives: `str-contains`, `str-lower`, `str-length`, `str-concat`
- A helper function `browser-filter-entries` is needed that takes entries and query, returns filtered list

### Dependencies
- US-BFS-01 (search mode activation, search state variables)

---

## US-BFS-03: Open File or Enter Directory from Search Results

### Problem
Kai Nakamura has used search to filter the browser listing and sees the file he wants. He expects Enter to open it -- the same behavior as in normal browsing. Without this, search would filter the list but leave him stranded, unable to act on the result.

### Who
- Terminal-native developer | Viewing search results in browser | Wants to open the selected file or enter the selected directory

### Solution
When Enter is pressed during search mode with a filtered list visible, the selected entry is opened (if file) or navigated into (if directory). Search mode is dismissed. If the entry is a file, the editor transitions to normal mode. If the entry is a directory, the browser loads that directory with a full listing (no search filter carried over).

### Traces to Jobs
- JS-BFS-01: Find a Known File in a Large Directory

### Domain Examples

#### 1: Happy Path -- Open a file from search results
Kai is in search mode with query "run". The filtered list shows `runtime.rs` with the cursor on it. He presses Enter. Alfred opens `/home/kai/projects/webapi/src/runtime.rs` in the editor buffer. The mode changes to "normal". The search prompt disappears.

#### 2: Happy Path -- Enter a directory from search results
Kai is in search mode with query "mod". The filtered list shows `models/` with the cursor on it. He presses Enter. The browser navigates into `models/`, showing its full contents. Search mode is dismissed. No filter carries over.

#### 3: Edge Case -- Enter when no matches (no-op)
Kai is in search mode with query "xyz". The listing shows "(no matches)". He presses Enter. Nothing happens. He remains in search mode and can edit his query or press Escape.

#### 4: Edge Case -- Open ".." from search results
Kai is in search mode and types ".." to filter to the parent entry. The `..` entry appears. He presses Enter. The browser navigates to the parent directory with a full listing.

### UAT Scenarios (BDD)

#### Scenario 1: Open file from search results
```gherkin
Given Kai is in search mode in the full-screen browser
And the query is "run" showing "runtime.rs" with cursor on it
When Kai presses Enter
Then Alfred opens /home/kai/projects/webapi/src/runtime.rs
And the editor mode is "normal"
And search mode is dismissed
```

#### Scenario 2: Enter directory from search results
```gherkin
Given Kai is in search mode in the full-screen browser
And the query is "mod" showing "models/" with cursor on it
When Kai presses Enter
Then the browser navigates into /home/kai/projects/webapi/src/models
And the full listing of models/ is displayed
And search mode is dismissed
And no search filter is active in the new directory
```

#### Scenario 3: Enter with no matches is a no-op
```gherkin
Given Kai is in search mode in the full-screen browser
And the query is "xyz" and the listing shows "(no matches)"
When Kai presses Enter
Then nothing happens
And Kai remains in search mode
And the query is still "xyz"
```

#### Scenario 4: Open parent directory from search
```gherkin
Given Kai is in search mode in the full-screen browser
And the query is ".." showing the ".." entry with cursor on it
When Kai presses Enter
Then the browser navigates to the parent directory
And the full listing of the parent directory is displayed
```

### Acceptance Criteria
- [ ] Enter on a file entry opens the file and transitions to normal editing mode
- [ ] Enter on a directory entry navigates into that directory with a full listing
- [ ] Search mode is dismissed after Enter (query cleared, prompt removed)
- [ ] No search filter is carried into a newly entered directory
- [ ] Enter is a no-op when the filtered list is empty (no matches)
- [ ] The `..` parent entry is openable from search results

### Technical Notes
- The Enter handler in search mode must operate on the filtered entry list, not the full `browser-entries`
- The cursor index during search indexes into the filtered list
- After Enter, `browser-search-active` is set to false, `browser-search-query` is cleared
- File opening uses the existing `open-file` primitive with `(path-join browser-current-dir entry-name)`
- Directory navigation uses the existing `browser-do-enter-dir` / `browser-do-parent` functions

### Dependencies
- US-BFS-01 (search mode activation)
- US-BFS-02 (filtering provides the list Enter acts on)

---

## US-BFS-04: Dismiss Search and Restore Full Listing

### Problem
Kai Nakamura has activated search mode but wants to cancel it -- either because his query did not find what he wanted, or because he changed his mind. He expects Escape to return him to the full directory listing without losing his place. Without this, he would be trapped in a filtered view with no way to see all entries again.

### Who
- Terminal-native developer | In search mode wanting to cancel | Wants to return to full listing with cursor position preserved

### Solution
Pressing Escape during search mode dismisses the search: the query is cleared, the search prompt is removed, the full directory listing is restored, and the cursor returns to its pre-search position. Pressing Backspace when the query is empty also dismisses search mode (natural text-input behavior).

### Traces to Jobs
- JS-BFS-03: Return to Full Listing After a Search

### Domain Examples

#### 1: Happy Path -- Escape from a filtered view
Kai was on entry 5 (`bridge.rs`) when he pressed `/`. He typed "run" and sees `runtime.rs` in the filtered list. He decides he actually wants `bridge.rs` after all. He presses Escape. The full 23-entry listing is restored. The cursor is back on entry 5 (`bridge.rs`). The search prompt is gone.

#### 2: Happy Path -- Backspace on empty query
Kai pressed `/` but immediately changed his mind. He presses Backspace. Since the query is empty, search mode is dismissed. The full listing is restored. Cursor is back at its pre-search position.

#### 3: Edge Case -- Escape when no matches were shown
Kai typed "xyz" and saw "(no matches)". He presses Escape. The full listing is restored as if the search never happened. Cursor is at its pre-search position.

### UAT Scenarios (BDD)

#### Scenario 1: Escape restores full listing and cursor
```gherkin
Given Kai is in search mode in the full-screen browser
And the pre-search cursor was on entry 5 ("bridge.rs")
And the search query is "run" showing 1 filtered entry
When Kai presses Escape
Then the search prompt disappears
And the full directory listing with all 23 entries is restored
And the cursor is on entry 5 ("bridge.rs")
```

#### Scenario 2: Backspace on empty query dismisses search
```gherkin
Given Kai is in search mode in the full-screen browser
And the search query is empty
When Kai presses Backspace
Then search mode is dismissed
And the full directory listing is restored
And the cursor returns to its pre-search position
```

#### Scenario 3: Escape after no-match search
```gherkin
Given Kai is in search mode with query "xyz" showing "(no matches)"
When Kai presses Escape
Then the full directory listing is restored
And the cursor is at its pre-search position
And no trace of the search remains in the display
```

#### Scenario 4: Multiple search-dismiss cycles
```gherkin
Given Kai is browsing /home/kai/projects/webapi/src with cursor on entry 5
When Kai presses / and types "app" and presses Escape
And Kai presses / and types "run" and presses Escape
Then the cursor is still on entry 5
And the full listing is displayed
And no search state leaks between cycles
```

### Acceptance Criteria
- [ ] Escape clears the search query, removes the prompt, and restores the full listing
- [ ] The cursor returns to the position it was at before search was activated
- [ ] Backspace on an empty query dismisses search mode (same effect as Escape)
- [ ] No residual search state persists after dismissal (clean slate for next search)
- [ ] Multiple search-dismiss cycles do not corrupt cursor position or entry list

### Technical Notes
- Escape handler sets `browser-search-active` to false, `browser-search-query` to empty string
- Cursor restoration: `(set browser-cursor browser-pre-search-cursor)`
- Then call `browser-render` to re-render the full listing
- Must also handle the case where entries changed between search start and Escape (e.g., this would only happen if the filesystem changed, which is unlikely but worth noting)

### Dependencies
- US-BFS-01 (pre-search cursor saved during activation)

---

## US-BFS-05: File Search in Sidebar Panel

### Problem
Kai Nakamura uses the sidebar (Ctrl-e) while editing to quickly navigate to related files. The sidebar has the same scrolling problem as the full-screen browser -- in directories with many entries, j/k scrolling is tedious. He expects the same search behavior in the sidebar as in the full-screen browser, because both show the same directory listing.

### Who
- Terminal-native developer | Using sidebar for file navigation while editing | Wants search in sidebar to work identically to full-screen browser

### Solution
The `/` key in filetree-mode activates search mode in the sidebar panel, with identical filtering, Enter, Escape, and Backspace behavior as the full-screen browser. The only differences are rendering (panel lines instead of buffer text) and the Enter-on-file behavior (sidebar unfocuses and returns to normal editing mode after opening).

### Traces to Jobs
- JS-BFS-01: Find a Known File in a Large Directory

### Domain Examples

#### 1: Happy Path -- Search in sidebar, open file
Kai has the sidebar open showing `/home/kai/projects/webapi/src` (23 entries). He presses `/`. The search prompt appears on line 2 of the sidebar panel. He types "run". The sidebar shows only `runtime.rs`. He presses Enter. The file opens in the editor buffer, the sidebar unfocuses, and the editor mode returns to "normal".

#### 2: Happy Path -- Search in sidebar, enter directory
Kai presses `/` in the sidebar and types "mod". The sidebar shows `models/`. He presses Enter. The sidebar navigates into `models/` showing its full contents. Search mode is dismissed. The sidebar remains focused.

#### 3: Happy Path -- Dismiss search in sidebar
Kai presses `/` in the sidebar and types "xyz" (no matches). He presses Escape. The full listing is restored in the sidebar panel. The sidebar cursor is back at its pre-search position.

### UAT Scenarios (BDD)

#### Scenario 1: Activate and use search in sidebar
```gherkin
Given Kai has the sidebar open showing /home/kai/projects/webapi/src
And the sidebar is focused with keymap "filetree-mode"
When Kai presses /
Then a search prompt "/" appears on line 2 of the sidebar panel
And typing "run" filters the sidebar to show only "runtime.rs"
```

#### Scenario 2: Open file from sidebar search
```gherkin
Given Kai is in search mode in the sidebar
And the query is "run" showing "runtime.rs" with cursor on it
When Kai presses Enter
Then Alfred opens /home/kai/projects/webapi/src/runtime.rs in the editor buffer
And the sidebar unfocuses
And the editor mode is "normal"
And search mode is dismissed
```

#### Scenario 3: Enter directory from sidebar search
```gherkin
Given Kai is in search mode in the sidebar
And the query is "mod" showing "models/" with cursor on it
When Kai presses Enter
Then the sidebar navigates into /home/kai/projects/webapi/src/models
And the sidebar shows the full listing of models/
And the sidebar remains focused
And search mode is dismissed
```

#### Scenario 4: Escape from sidebar search
```gherkin
Given Kai is in search mode in the sidebar
And the search query is "br"
When Kai presses Escape
Then the search prompt disappears from the sidebar
And the full directory listing is restored in the sidebar
And the sidebar cursor returns to its pre-search position
```

#### Scenario 5: Behavioral parity with full-screen browser
```gherkin
Given the directory /home/kai/projects/webapi/src in both full-screen browser and sidebar
When Kai types "br" in search mode in the full-screen browser
And Kai types "br" in search mode in the sidebar
Then both display the same filtered entries: "bridge.rs" and "bridge_helpers.rs"
```

### Acceptance Criteria
- [ ] `/` in filetree-mode activates search mode in the sidebar panel
- [ ] Typing filters sidebar entries identically to the full-screen browser (same matching logic)
- [ ] Enter on a file opens the file, unfocuses sidebar, and transitions to normal mode
- [ ] Enter on a directory navigates into it within the sidebar (sidebar stays focused)
- [ ] Escape restores the full listing and pre-search cursor in the sidebar
- [ ] Same filtered entries appear in sidebar and full-screen browser for the same query and directory

### Technical Notes
- Sidebar search uses `sidebar-entries` as the filter source (parallel to `browser-entries`)
- New state variables: `sidebar-search-active`, `sidebar-search-query`, `sidebar-pre-search-cursor`
- Sidebar rendering uses `set-panel-line` instead of `buffer-set-content` -- the search render function must clear and repopulate panel lines
- The `/` key binding must be added to `filetree-mode` keymap
- Search character routing in sidebar needs the same mechanism as the browser (Char:a through Char:z etc.)
- Consider extracting shared search logic into reusable Lisp functions (e.g., `search-filter-entries`, `search-append-char`, `search-dismiss`) to avoid duplication

### Dependencies
- US-BFS-01 through US-BFS-04 (core search behavior established first)
- Existing sidebar implementation (implemented)

---

## Story Prioritization

| Story | MoSCoW | Effort | Value | Rationale |
|-------|--------|--------|-------|-----------|
| US-BFS-01 | Must Have | 1 day | High | Foundation: without search activation, nothing works |
| US-BFS-02 | Must Have | 1-2 days | High | Core value: filtering is the entire point of the feature |
| US-BFS-03 | Must Have | 0.5 days | High | Without acting on results, search is useless |
| US-BFS-04 | Must Have | 0.5 days | High | Without safe dismissal, users get trapped in search mode |
| US-BFS-05 | Should Have | 1-2 days | Medium | Parity: sidebar users expect the same capability |

**Recommended order**: US-BFS-01 -> US-BFS-02 -> US-BFS-04 -> US-BFS-03 -> US-BFS-05

Rationale: Build search activation first, then filtering (the core). Add dismissal before open-file because a safe exit path is needed for testing. Then add the action (Enter). Finally, extend to sidebar.

---

## Definition of Ready Validation

### US-BFS-01: Enter Search Mode in Browser

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "He wastes 5-10 seconds scrolling through entries he does not care about" |
| User/persona identified | PASS | Kai Nakamura, backend developer, vim/fzf user |
| 3+ domain examples | PASS | 3 examples: happy path, single-entry dir, empty dir |
| UAT scenarios (3-7) | PASS | 4 scenarios |
| AC derived from UAT | PASS | 5 criteria derived from scenarios |
| Right-sized | PASS | 1 day effort, 4 scenarios |
| Technical notes | PASS | New state vars, keymap binding, character routing |
| Dependencies tracked | PASS | Existing browse-mode (implemented) |

**DoR Status**: PASSED

### US-BFS-02: Filter Browser Entries by Substring Match

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Without live filtering, the search prompt would be useless" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 5 examples: "run", "br", no matches, backspace, case-insensitive |
| UAT scenarios (3-7) | PASS | 5 scenarios |
| AC derived from UAT | PASS | 8 criteria |
| Right-sized | PASS | 1-2 days effort, 5 scenarios |
| Technical notes | PASS | str-contains, str-lower, filter function, cursor bounds |
| Dependencies tracked | PASS | US-BFS-01 |

**DoR Status**: PASSED

### US-BFS-03: Open File or Enter Directory from Search Results

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Search would filter the list but leave him stranded" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 4 examples: open file, enter dir, no matches, open parent |
| UAT scenarios (3-7) | PASS | 4 scenarios |
| AC derived from UAT | PASS | 6 criteria |
| Right-sized | PASS | 0.5 days effort, 4 scenarios |
| Technical notes | PASS | Filtered list indexing, open-file, enter-dir reuse |
| Dependencies tracked | PASS | US-BFS-01, US-BFS-02 |

**DoR Status**: PASSED

### US-BFS-04: Dismiss Search and Restore Full Listing

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "He would be trapped in a filtered view" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 3 examples: escape, backspace-on-empty, escape-from-no-matches |
| UAT scenarios (3-7) | PASS | 4 scenarios |
| AC derived from UAT | PASS | 5 criteria |
| Right-sized | PASS | 0.5 days effort, 4 scenarios |
| Technical notes | PASS | State cleanup, cursor restoration |
| Dependencies tracked | PASS | US-BFS-01 |

**DoR Status**: PASSED

### US-BFS-05: File Search in Sidebar Panel

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "The sidebar has the same scrolling problem" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 3 examples: search+open, search+enter-dir, dismiss |
| UAT scenarios (3-7) | PASS | 5 scenarios |
| AC derived from UAT | PASS | 6 criteria |
| Right-sized | PASS | 1-2 days effort, 5 scenarios |
| Technical notes | PASS | Parallel state vars, set-panel-line rendering, shared logic extraction |
| Dependencies tracked | PASS | US-BFS-01 through US-BFS-04, existing sidebar |

**DoR Status**: PASSED
