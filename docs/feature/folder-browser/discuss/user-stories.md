# User Stories: Folder Browser

## Story Map

```
Workflow: [Launch]  -->  [Display]  -->  [Navigate]  -->  [Open]
            |               |               |               |
Row 1:   Detect dir      List entries    j/k movement    Open file
         argument        sorted          Enter dir       in buffer
         (US-FB-01)      (US-FB-02)      (US-FB-03)      (US-FB-05)
            |               |               |
Row 2:   Error paths     Empty dir       Parent nav      Binary/error
         (US-FB-01)      (US-FB-02)      + quit          handling
                                         (US-FB-04)      (US-FB-05)
```

Row 1 = MVP (walking skeleton). Row 2 = included in same stories for completeness.

---

## US-FB-01: Detect Directory Argument and Enter Browser Mode

### Problem
Kai Nakamura is a backend developer who frequently opens Alfred to edit files. When he runs `alfred .` or `alfred /home/kai/projects/webapi`, Alfred currently fails or opens a nonsensical empty buffer because it doesn't distinguish directories from files. He has to remember the exact file path, often switching to another terminal to run `tree` or `ls` first.

### Who
- Terminal-native developer | Working in project directory | Wants to browse files without memorizing paths

### Solution
When the CLI argument is a directory, Alfred enters a dedicated "browse" mode instead of trying to open it as a file. When the argument is a file, existing behavior is unchanged. When no argument is given, existing behavior is unchanged.

### Traces to Jobs
- JS-01: Navigate to a file in an unfamiliar project
- JS-02: Quickly open a known file from a project root

### Domain Examples

#### 1: Happy Path -- Open current directory
Kai is inside ~/projects/webapi and runs `alfred .`. Alfred detects that `.` resolves to a directory, initializes browser state with that path, sets the mode to "browse", and renders the folder browser.

#### 2: Happy Path -- Open absolute path
Kai runs `alfred /home/kai/projects/webapi` from any directory. Alfred detects the argument is a directory, enters browser mode showing the contents of /home/kai/projects/webapi.

#### 3: Edge Case -- Argument is a file (unchanged behavior)
Kai runs `alfred src/main.rs`. Alfred detects it is a file, opens it in the editor buffer as usual. Browser mode is not activated.

#### 4: Error -- Nonexistent path
Kai runs `alfred /bad/path`. Alfred prints `alfred: no such file or directory: /bad/path` to stderr and exits with code 1.

#### 5: Error -- Permission denied on directory
Kai runs `alfred /root/secrets` without root privileges. Alfred prints `alfred: permission denied: /root/secrets` to stderr and exits with code 1.

### UAT Scenarios (BDD)

#### Scenario 1: Directory argument enters browser mode
```gherkin
Given Kai runs "alfred ." in the directory ~/projects/webapi
When Alfred starts
Then Alfred enters folder browser mode
And the editor mode is "browse"
```

#### Scenario 2: Absolute directory path enters browser mode
```gherkin
Given Kai runs "alfred /home/kai/projects/webapi"
When Alfred starts
Then Alfred enters folder browser mode
And the browser displays the contents of /home/kai/projects/webapi
```

#### Scenario 3: File argument opens editor normally
```gherkin
Given Kai runs "alfred src/main.rs" in ~/projects/webapi
When Alfred starts
Then Alfred opens src/main.rs in the editor buffer
And the mode is "normal"
```

#### Scenario 4: Nonexistent path shows error
```gherkin
Given Kai runs "alfred /nonexistent/path"
When Alfred starts
Then Alfred prints "alfred: no such file or directory: /nonexistent/path" to stderr
And Alfred exits with a non-zero exit code
```

#### Scenario 5: Permission denied shows error
```gherkin
Given Kai runs "alfred /root/secrets" without read permission
When Alfred starts
Then Alfred prints "alfred: permission denied: /root/secrets" to stderr
And Alfred exits with a non-zero exit code
```

### Acceptance Criteria
- [ ] Running `alfred <directory>` enters browser mode (mode = "browse")
- [ ] Running `alfred <file>` opens the file in editor mode (existing behavior unchanged)
- [ ] Running `alfred` with no arguments opens empty buffer (existing behavior unchanged)
- [ ] Nonexistent paths produce a clear error message on stderr and exit code 1
- [ ] Permission-denied paths produce a clear error message on stderr and exit code 1

### Technical Notes
- Argument classification happens in `alfred-bin/src/main.rs` before `Buffer::from_file()`
- Uses `std::path::Path::is_dir()` and `Path::is_file()` for classification
- A new mode string "browse" is needed in EditorState (alongside "normal", "insert", "visual")
- Browser state (current directory, entries, cursor) needs a home -- either in EditorState or a separate struct
- Depends on: nothing (first story in the chain)

### Dependencies
- None -- this is the entry point story

---

## US-FB-02: Display Directory Contents in Browser View

### Problem
Kai Nakamura has entered browser mode by running `alfred .`, but he sees nothing useful unless the directory contents are rendered. He needs to see files and folders listed clearly, sorted in a way that makes sense (directories first, then files), so he can orient himself in the project.

### Who
- Terminal-native developer | Browsing a project directory | Needs visual orientation in the file structure

### Solution
The browser view renders the contents of the current directory: directories listed first (alphabetically), then files (alphabetically). Each directory has a trailing `/` indicator. The cursor highlights the first entry. The status bar shows "BROWSE" and the current directory path.

### Traces to Jobs
- JS-03: Explore a project structure to understand layout
- JS-01: Navigate to a file in an unfamiliar project

### Domain Examples

#### 1: Happy Path -- Typical Rust project directory
Kai browses ~/projects/webapi which contains: `src/` (dir), `tests/` (dir), `.gitignore`, `Cargo.lock`, `Cargo.toml`, `README.md`. The browser displays:
```
  src/
  tests/
  .gitignore
  Cargo.lock
  Cargo.toml
  README.md
```
Directories `src/` and `tests/` appear first (alphabetical), then files `.gitignore`, `Cargo.lock`, `Cargo.toml`, `README.md` (alphabetical). Cursor highlights `src/`.

#### 2: Edge Case -- Directory with only dotfiles
Kai browses ~/projects/webapi/.github which contains: `workflows/` (dir), `CODEOWNERS`, `dependabot.yml`. All are displayed. Dotfiles are not hidden by default.

#### 3: Edge Case -- Empty directory
Kai browses ~/projects/empty-dir which contains nothing. The browser shows "Directory is empty" and Kai can press `q` to quit or `h` to go to parent.

#### 4: Happy Path -- Subdirectory with parent entry
Kai has navigated into ~/projects/webapi/src. The browser shows `../` as the first entry, followed by the directory's own contents.

### UAT Scenarios (BDD)

#### Scenario 1: Entries sorted directories-first then files
```gherkin
Given Kai has entered folder browser mode in ~/projects/webapi
And the directory contains: src/ (dir), tests/ (dir), Cargo.toml (file), README.md (file), .gitignore (file)
When the browser renders
Then the entries are displayed in this order:
  | entry       | type |
  | src/        | dir  |
  | tests/      | dir  |
  | .gitignore  | file |
  | Cargo.toml  | file |
  | README.md   | file |
```

#### Scenario 2: Cursor starts on first entry
```gherkin
Given Kai has entered folder browser mode in ~/projects/webapi
When the browser renders
Then the cursor highlights the first entry
```

#### Scenario 3: Status bar shows mode and path
```gherkin
Given Kai has entered folder browser mode in ~/projects/webapi
When the browser renders
Then the status bar displays "BROWSE" as the mode indicator
And the status bar displays the current directory path
```

#### Scenario 4: Empty directory shows message
```gherkin
Given Kai has entered folder browser mode in ~/projects/empty-dir
And the directory contains no entries
When the browser renders
Then the browser displays "Directory is empty"
```

#### Scenario 5: Directories have trailing slash indicator
```gherkin
Given Kai has entered folder browser mode in ~/projects/webapi
When the browser renders
Then each directory entry ends with "/"
And file entries do not end with "/"
```

### Acceptance Criteria
- [ ] Directories listed before files, both groups sorted alphabetically
- [ ] Directories display with trailing `/` indicator
- [ ] Cursor highlights the first entry on initial render
- [ ] Status bar shows "BROWSE" mode and current directory path
- [ ] Empty directories show "Directory is empty" message
- [ ] Dotfiles are shown (not hidden by default)
- [ ] When in a subdirectory, `../` appears as the first entry

### Technical Notes
- Uses `std::fs::read_dir()` to enumerate entries
- Entry classification uses `DirEntry::file_type()` (handles symlinks via `metadata()`)
- Rendering reuses the existing panel system for the status bar
- The tree listing renders in the main content area (where the buffer normally displays)
- Entries should be sorted case-insensitively for natural ordering
- Depends on: US-FB-01 (browser mode must exist to render into)

### Dependencies
- US-FB-01 (directory detection and browser mode activation)

---

## US-FB-03: Navigate Directory Entries with Vim Keys

### Problem
Kai Nakamura can see the directory contents but cannot interact with them. He needs to move a cursor through the entries using familiar vim-style keys (j/k for up/down, gg/G for top/bottom) so he can get to his target file quickly without learning new keybindings.

### Who
- Vim-fluent developer | Browsing a project directory | Expects j/k/gg/G to work as in vim

### Solution
In browser mode, j moves the cursor down, k moves up, gg jumps to the first entry, G jumps to the last. Cursor stays within bounds (no wrapping, no overflow). Arrow keys also work as alternatives (Down/Up).

### Traces to Jobs
- JS-02: Quickly open a known file from project root
- OS-05: Navigation feels native to vim muscle memory

### Domain Examples

#### 1: Happy Path -- Move down through entries
Kai is browsing ~/projects/webapi. Cursor is on `src/` (first entry). He presses `j` three times. Cursor moves to `tests/`, then `.gitignore`, then `Cargo.lock`.

#### 2: Boundary -- Cursor at last entry, press j
Kai's cursor is on `README.md` (last entry). He presses `j`. Cursor stays on `README.md` -- no wrapping.

#### 3: Boundary -- Cursor at first entry, press k
Kai's cursor is on `src/` (first entry). He presses `k`. Cursor stays on `src/`.

#### 4: Happy Path -- Jump to end with G
Kai's cursor is on `src/` (first entry). He presses `G`. Cursor jumps to `README.md` (last entry).

#### 5: Happy Path -- Jump to top with gg
Kai's cursor is on `Cargo.toml` (middle). He presses `g` then `g`. Cursor jumps to `src/` (first entry).

### UAT Scenarios (BDD)

#### Scenario 1: j moves cursor down
```gherkin
Given Kai is in the folder browser with cursor on "src/" (first entry)
When Kai presses "j"
Then the cursor moves to "tests/" (second entry)
```

#### Scenario 2: k moves cursor up
```gherkin
Given Kai is in the folder browser with cursor on "tests/" (second entry)
When Kai presses "k"
Then the cursor moves to "src/" (first entry)
```

#### Scenario 3: Cursor does not go past last entry
```gherkin
Given Kai is in the folder browser with cursor on "README.md" (last entry)
When Kai presses "j"
Then the cursor remains on "README.md"
```

#### Scenario 4: Cursor does not go past first entry
```gherkin
Given Kai is in the folder browser with cursor on "src/" (first entry)
When Kai presses "k"
Then the cursor remains on "src/"
```

#### Scenario 5: G jumps to last entry
```gherkin
Given Kai is in the folder browser with cursor on "src/" (first entry)
When Kai presses "G"
Then the cursor moves to "README.md" (last entry)
```

#### Scenario 6: gg jumps to first entry
```gherkin
Given Kai is in the folder browser with cursor on "Cargo.toml"
When Kai presses "g" followed by "g"
Then the cursor moves to "src/" (first entry)
```

### Acceptance Criteria
- [ ] `j` and Down arrow move cursor down one entry
- [ ] `k` and Up arrow move cursor up one entry
- [ ] Cursor does not move past the first or last entry (no wrapping)
- [ ] `G` jumps cursor to the last entry
- [ ] `gg` jumps cursor to the first entry
- [ ] Visual highlight (cursor indicator) updates immediately on key press

### Technical Notes
- Browser mode needs its own keymap (registered as "browse" keymap)
- `gg` requires a "pending g" input state (similar to existing `g` prefix handling in normal mode)
- Cursor index must be clamped to `0..entries.len().saturating_sub(1)`
- Arrow keys (Up/Down) should be bound as aliases for k/j
- Depends on: US-FB-02 (entries must be displayed for cursor to navigate)

### Dependencies
- US-FB-02 (directory display provides the entry list to navigate)

---

## US-FB-04: Enter Subdirectories and Navigate to Parent

### Problem
Kai Nakamura can move the cursor through entries but cannot dive into subdirectories or go back to the parent. Without this, browsing nested project structures is impossible -- he is stuck in the top-level directory.

### Who
- Developer navigating a nested project | Needs to enter src/ then handlers/ to find the right file | Expects h and Enter to work like vim's directional model

### Solution
Pressing Enter or `l` on a directory entry navigates into it (updates current_dir, reads new entries, positions cursor at top). Pressing `h` or Backspace goes to the parent directory, restoring the cursor to the directory entry the user came from. Pressing Enter on `../` also navigates to parent. `q` and Escape exit Alfred entirely.

### Traces to Jobs
- JS-01: Navigate to a file in an unfamiliar project
- JS-03: Explore a project structure to understand layout

### Domain Examples

#### 1: Happy Path -- Enter src/ subdirectory
Kai is browsing ~/projects/webapi with cursor on `src/`. He presses Enter. The browser now shows the contents of ~/projects/webapi/src with `../` at the top. Status bar shows "BROWSE webapi/src/".

#### 2: Happy Path -- Return to parent with h
Kai is in ~/projects/webapi/src. He presses `h`. Browser shows ~/projects/webapi again. The cursor is on `src/` (the entry he came from).

#### 3: Happy Path -- Navigate via ../ entry
Kai is in ~/projects/webapi/src with cursor on `../`. He presses Enter. Browser shows ~/projects/webapi. Cursor is on `src/`.

#### 4: Error -- Permission denied on subdirectory
Kai is browsing ~/projects/webapi with cursor on `secrets/` (no read permission). He presses Enter. The status bar shows "Permission denied: secrets/". Cursor stays on `secrets/`.

#### 5: Edge Case -- Quit with q
Kai is browsing ~/projects/webapi. He presses `q`. Alfred exits cleanly.

#### 6: Edge Case -- Symlink directory
Kai's cursor is on `config/` which is a symlink to `/etc/webapi`. He presses Enter. Browser shows the contents of /etc/webapi.

### UAT Scenarios (BDD)

#### Scenario 1: Enter subdirectory with Enter
```gherkin
Given Kai is in the folder browser with cursor on "src/"
When Kai presses Enter
Then the browser displays the contents of ~/projects/webapi/src
And the first entry is "../"
And the status bar shows "BROWSE webapi/src/"
```

#### Scenario 2: Navigate to parent with h
```gherkin
Given Kai is browsing ~/projects/webapi/src
When Kai presses "h"
Then the browser displays the contents of ~/projects/webapi
And the cursor is restored to the "src/" entry
```

#### Scenario 3: Navigate to parent via ../ entry
```gherkin
Given Kai is browsing ~/projects/webapi/src with cursor on "../"
When Kai presses Enter
Then the browser displays the contents of ~/projects/webapi
And the cursor is restored to the "src/" entry
```

#### Scenario 4: Permission denied on subdirectory
```gherkin
Given Kai is in the folder browser with cursor on "secrets/" (no read permission)
When Kai presses Enter
Then the status bar shows "Permission denied: secrets/"
And Kai remains in the folder browser
And the cursor stays on "secrets/"
```

#### Scenario 5: Quit browser with q
```gherkin
Given Kai is in the folder browser
When Kai presses "q"
Then Alfred exits cleanly with exit code 0
```

#### Scenario 6: Quit browser with Escape
```gherkin
Given Kai is in the folder browser
When Kai presses Escape
Then Alfred exits cleanly with exit code 0
```

### Acceptance Criteria
- [ ] Enter on a directory entry navigates into that directory
- [ ] `l` on a directory entry navigates into that directory (vim alias)
- [ ] `h` navigates to the parent directory
- [ ] Backspace navigates to the parent directory
- [ ] Enter on `../` navigates to the parent directory
- [ ] Cursor is restored to the correct entry when returning to parent
- [ ] Permission denied on a subdirectory shows an error message and keeps cursor in place
- [ ] `q` exits Alfred cleanly
- [ ] Escape exits Alfred cleanly
- [ ] Status bar updates with new directory path after navigation

### Technical Notes
- Navigation history is a stack of `(PathBuf, usize)` storing directory path and cursor position
- When entering a subdirectory: push `(current_dir, cursor_index)` onto history
- When going to parent: pop from history to restore cursor position
- If history is empty (user presses h at the root they opened), navigate to filesystem parent but cursor starts at 0
- `q`/Esc bind to the existing "quit" command (or a new "browser-quit" command that exits cleanly)
- Symlinks: use `std::fs::read_link()` + `canonicalize()` for display, `read_dir()` follows symlinks naturally
- Depends on: US-FB-03 (cursor navigation must work for Enter to select the right entry)

### Dependencies
- US-FB-03 (cursor navigation)

---

## US-FB-05: Open Selected File in Editor Buffer

### Problem
Kai Nakamura has navigated to the file he wants to edit but pressing Enter on a file does nothing yet. Without this final step, the folder browser is a dead end -- he can see files but not open them. This is the payoff step that makes the entire browsing journey worthwhile.

### Who
- Developer who has found the right file | Expects Enter to open it seamlessly | Wants to start editing immediately

### Solution
Pressing Enter or `l` on a file entry loads the file into the editor buffer, closes the browser, switches the mode to "normal", and activates all standard editor features (gutter, syntax highlighting, status bar showing filename). The transition is seamless -- the user is now in the same state as if they had run `alfred path/to/file.rs` directly.

### Traces to Jobs
- JS-01: Navigate to a file in an unfamiliar project
- JS-02: Quickly open a known file from project root
- OS-06: Minimize cognitive load transitioning between browsing and editing

### Domain Examples

#### 1: Happy Path -- Open a Rust file
Kai is browsing ~/projects/webapi/src with cursor on `main.rs`. He presses Enter. The browser disappears. Alfred shows the contents of main.rs with syntax highlighting, line numbers, and the status bar reading "NORMAL src/main.rs 1:1". He begins editing.

#### 2: Happy Path -- Open a TOML file from project root
Kai is browsing ~/projects/webapi with cursor on `Cargo.toml`. He presses Enter. Alfred opens Cargo.toml. Syntax highlighting is active for TOML. Status bar shows "NORMAL Cargo.toml".

#### 3: Error -- Binary file
Kai is browsing ~/projects/webapi/assets with cursor on `logo.png`. He presses Enter. The status bar shows "Cannot open binary file: logo.png". Kai remains in the browser.

#### 4: Error -- Permission denied
Kai is browsing with cursor on `secret.key` (no read permission). He presses Enter. The status bar shows "Permission denied: secret.key". Kai remains in the browser.

#### 5: Happy Path -- File opened via l key
Kai is browsing with cursor on `README.md`. He presses `l`. Alfred opens README.md in the editor. Same behavior as Enter.

### UAT Scenarios (BDD)

#### Scenario 1: Open a Rust file
```gherkin
Given Kai is in the folder browser at ~/projects/webapi/src with cursor on "main.rs"
When Kai presses Enter
Then the browser closes
And the file ~/projects/webapi/src/main.rs is loaded into the editor buffer
And the mode changes to "NORMAL"
And the status bar shows "NORMAL" and the filename "src/main.rs"
And syntax highlighting is active for Rust
And the cursor is at line 1, column 1
```

#### Scenario 2: Opened file is unmodified
```gherkin
Given Kai opens "main.rs" from the folder browser
When the file is loaded into the editor buffer
Then the buffer modified flag is false
```

#### Scenario 3: Open file with l key
```gherkin
Given Kai is in the folder browser with cursor on "Cargo.toml"
When Kai presses "l"
Then the file is loaded into the editor buffer
And the mode changes to "NORMAL"
```

#### Scenario 4: Binary file stays in browser
```gherkin
Given Kai is in the folder browser with cursor on "logo.png"
When Kai presses Enter
Then the status bar shows "Cannot open binary file: logo.png"
And Kai remains in the folder browser
And the cursor stays on "logo.png"
```

#### Scenario 5: Permission denied stays in browser
```gherkin
Given Kai is in the folder browser with cursor on "secret.key" (no read permission)
When Kai presses Enter
Then the status bar shows "Permission denied: secret.key"
And Kai remains in the folder browser
```

### Acceptance Criteria
- [ ] Enter on a file entry loads the file into the editor buffer
- [ ] `l` on a file entry loads the file into the editor buffer
- [ ] Mode switches from "browse" to "normal" after opening a file
- [ ] Status bar shows "NORMAL" and the file path after opening
- [ ] Syntax highlighting activates based on file extension
- [ ] Cursor starts at line 1, column 1
- [ ] Buffer modified flag is false after opening
- [ ] Binary files show an error message and stay in browser
- [ ] Permission-denied files show an error message and stay in browser
- [ ] All standard editor features are active after opening (gutter, hooks, keymaps)

### Technical Notes
- File loading reuses existing `Buffer::from_file()` from alfred-core
- Binary detection: attempt to read as UTF-8; if decoding fails, treat as binary
- After opening, browser state should be cleared (or at minimum, browser keymap deactivated)
- The mode transition triggers a `mode-changed` hook (existing hook system), which activates the normal-mode keymap and triggers status bar / gutter updates
- Syntax highlighting detection reuses existing `SyntaxHighlighter` filename matching
- File path in status bar should be relative to the directory Alfred was originally opened with
- Depends on: US-FB-04 (must be able to navigate to the file first)

### Dependencies
- US-FB-04 (subdirectory navigation to reach deeply nested files)
- US-FB-01 through US-FB-04 implicitly (the entire browse journey)

---

## Story Dependency Graph

```
US-FB-01  Detect directory argument, enter browser mode
    |
    v
US-FB-02  Display directory contents in browser view
    |
    v
US-FB-03  Navigate entries with vim keys (j/k/gg/G)
    |
    v
US-FB-04  Enter subdirectories and navigate to parent (Enter/h/q)
    |
    v
US-FB-05  Open selected file in editor buffer
```

Each story builds on the previous one. Together they form the complete "browse and open" journey.

---

## MoSCoW Classification

| Story | Priority | Rationale |
|-------|----------|-----------|
| US-FB-01 | Must Have | Without directory detection, the feature cannot start |
| US-FB-02 | Must Have | Without rendering, user sees nothing |
| US-FB-03 | Must Have | Without navigation, user cannot reach files |
| US-FB-04 | Must Have | Without directory traversal, only top-level browsing works |
| US-FB-05 | Must Have | Without file opening, the browser is a dead end |

All stories are Must Have because they form a single chain. Removing any link breaks the journey. There are no Should Have or Could Have stories in this initial set -- those would be future enhancements like:
- Could Have: File search/filter within browser (type to filter)
- Could Have: Return to browser after opening a file (requires multi-buffer)
- Could Have: File preview (show first N lines of file under cursor)
- Could Have: Toggle hidden files on/off
- Could Have: Show file metadata (size, modified date)

---

## Definition of Ready Validation

### US-FB-01: Detect Directory Argument

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Alfred fails or opens nonsensical buffer when given a directory path" |
| User/persona identified | PASS | Kai Nakamura, backend developer, terminal-native, vim-comfortable |
| 3+ domain examples | PASS | 5 examples: happy path (2), edge case (1), errors (2) |
| UAT scenarios (3-7) | PASS | 5 scenarios |
| AC derived from UAT | PASS | 5 AC items traced to scenarios |
| Right-sized | PASS | ~1 day effort, 5 scenarios, single capability |
| Technical notes | PASS | Argument classification in main.rs, Path::is_dir(), new mode string |
| Dependencies tracked | PASS | None (first story) |

**DoR Status**: PASSED

### US-FB-02: Display Directory Contents

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "User enters browser mode but sees nothing without rendering" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 4 examples: typical dir, dotfiles, empty dir, subdirectory |
| UAT scenarios (3-7) | PASS | 5 scenarios |
| AC derived from UAT | PASS | 7 AC items covering sort, indicators, cursor, status, empty state |
| Right-sized | PASS | ~2 days effort, 5 scenarios |
| Technical notes | PASS | read_dir(), DirEntry, panel system, case-insensitive sort |
| Dependencies tracked | PASS | Depends on US-FB-01 |

**DoR Status**: PASSED

### US-FB-03: Navigate with Vim Keys

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Cannot interact with directory entries without cursor movement" |
| User/persona identified | PASS | Kai Nakamura, vim-fluent |
| 3+ domain examples | PASS | 5 examples: move down, boundary last, boundary first, G, gg |
| UAT scenarios (3-7) | PASS | 6 scenarios |
| AC derived from UAT | PASS | 6 AC items |
| Right-sized | PASS | ~1 day effort, 6 scenarios |
| Technical notes | PASS | Browser keymap, gg input state, cursor clamping, arrow aliases |
| Dependencies tracked | PASS | Depends on US-FB-02 |

**DoR Status**: PASSED

### US-FB-04: Subdirectory Navigation and Quit

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Cannot browse nested directories or exit the browser" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 6 examples: enter dir, parent with h, parent via ../, permission denied, quit, symlink |
| UAT scenarios (3-7) | PASS | 6 scenarios |
| AC derived from UAT | PASS | 10 AC items |
| Right-sized | PASS | ~2 days effort, 6 scenarios |
| Technical notes | PASS | Navigation history stack, symlink handling, quit binding |
| Dependencies tracked | PASS | Depends on US-FB-03 |

**DoR Status**: PASSED

### US-FB-05: Open File in Buffer

| DoR Item | Status | Evidence |
|----------|--------|----------|
| Problem statement clear | PASS | "Browser is a dead end without file opening capability" |
| User/persona identified | PASS | Kai Nakamura |
| 3+ domain examples | PASS | 5 examples: Rust file, TOML file, binary, permission denied, l key |
| UAT scenarios (3-7) | PASS | 5 scenarios |
| AC derived from UAT | PASS | 10 AC items |
| Right-sized | PASS | ~2 days effort, 5 scenarios |
| Technical notes | PASS | Buffer::from_file(), binary detection, mode transition, hook system |
| Dependencies tracked | PASS | Depends on US-FB-04 |

**DoR Status**: PASSED
