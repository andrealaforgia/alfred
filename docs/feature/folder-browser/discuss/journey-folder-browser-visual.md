# Journey Visual: Folder Browser

## Journey Overview

```
[Trigger]           [Step 1]          [Step 2]          [Step 3]          [Step 4]          [Goal]
alfred .  -------> Detect    ------> Display   ------> Navigate  ------> Open File ------> Editing
                   Directory         Tree View         & Select          in Buffer          in Alfred
                   Argument          (Browser)         Target File

Feels: Intentional  Feels: Smooth    Feels: Oriented   Feels: In Control  Feels: Seamless   Feels: Productive
       "Let me       "Recognized      "I can see         "Moving toward     "Right file,       "Ready to
        browse"       my intent"       the structure"     my target"         now editing"        work"
```

## Emotional Arc

```
Confidence
    ^
    |                                                    ****
    |                                              *****    ***
    |                                         ****             **
    |                                    ****                    **
    |                               ****                          *
    |                          ****                                 *
    |                     ****
    |                ****
    |           ****
    |      ****
    | ****
    +--------------------------------------------------------->  Time
    Trigger    Detect     Display     Navigate     Open      Editing
    (Curious)  (Smooth)   (Oriented)  (Confident)  (Relief)  (Productive)
```

**Arc Pattern**: Confidence Building
- Start: Curious / Intentional -- "I want to browse this project"
- Middle: Oriented / In Control -- "I can see the structure and move through it"
- End: Productive / Satisfied -- "I'm editing the right file, no friction"
- No jarring transitions -- the browser feels like a natural editor state, not a popup or overlay

---

## Step 1: Detect Directory Argument

**Trigger**: Kai types `alfred .` or `alfred /home/kai/projects/webapi`

**What happens**: Alfred detects the argument is a directory (not a file). Instead of showing an error or loading an empty buffer, it transitions to folder browser mode.

```
+-- Terminal -----------------------------------------------------------+
| $ alfred .                                                            |
|                                                                       |
|  (Alfred starts, detects "." is a directory, enters browser mode)     |
|                                                                       |
+-----------------------------------------------------------------------+
```

**Emotional state**: Curious -> Smooth ("It understood what I meant")

**Error paths**:
- Path does not exist: `alfred: no such file or directory: /bad/path`
- Path is a file: Open normally in editor (not browser)
- No argument: Open empty buffer (existing behavior unchanged)

---

## Step 2: Display Tree View

**What happens**: Alfred renders the folder browser. The current directory is shown at the top. Entries are listed below -- directories first, then files. Directories have a visual indicator. The cursor starts on the first entry.

```
+-- Alfred: Browse ~/projects/webapi ---------------------------------+
|  webapi/                                                             |
|  > src/                                                              |
|    .gitignore                                                        |
|    Cargo.lock                                                        |
|    Cargo.toml                                                        |
|    README.md                                                         |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
+----------------------------------------------------------------------+
| BROWSE  webapi/                                                      |
+----------------------------------------------------------------------+
```

**Key details**:
- `>` prefix indicates the cursor position (highlighted row)
- Directories listed first, then files (both alphabetical)
- Directories shown with trailing `/`
- Status bar shows `BROWSE` mode and current directory path
- Hidden files (dotfiles) shown by default but configurable
- No tree indentation yet -- flat listing of current directory contents

**Emotional state**: Oriented ("I can see the project layout")

**Error paths**:
- Empty directory: Show message "Directory is empty" with `q` to quit
- Permission denied on directory: `alfred: permission denied: /root/secrets`

---

## Step 3: Navigate and Select

**What happens**: Kai uses vim-style keys to navigate the tree. `j`/`k` move the cursor. `Enter` on a directory expands/enters it. `Enter` on a file opens it. `h` or `Backspace` goes to the parent directory. `q` or `Escape` quits the browser.

### Navigating into a subdirectory

```
+-- Alfred: Browse ~/projects/webapi/src ------------------------------+
|  ../                                                                 |
|    handlers/                                                         |
|  > models/                                                           |
|    routes/                                                           |
|    lib.rs                                                            |
|    main.rs                                                           |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
|                                                                      |
+----------------------------------------------------------------------+
| BROWSE  webapi/src/                                                  |
+----------------------------------------------------------------------+
```

**Key bindings** (all vim-native):

| Key | Action |
|-----|--------|
| `j` / Down | Move cursor down one entry |
| `k` / Up | Move cursor up one entry |
| `Enter` / `l` | Open file or enter directory |
| `h` / Backspace | Go to parent directory |
| `q` / Escape | Quit browser (exit Alfred) |
| `g` `g` | Jump to first entry |
| `G` | Jump to last entry |

**Key details**:
- `../` entry at the top navigates to parent directory (Enter or `h`)
- Cursor wraps: going past last entry does not wrap to top (vim behavior)
- Current directory path updates in status bar
- Previous cursor position remembered when navigating back to parent

**Emotional state**: In control ("I can move quickly, this feels like vim")

**Error paths**:
- Permission denied on subdirectory: Show inline message "Permission denied" on status bar, cursor stays
- Symlink to directory: Follow it, display resolved path
- Symlink to file: Open the file (follow symlink)
- Broken symlink: Show entry with visual indicator (dimmed or marked), show error on Enter

---

## Step 4: Open File in Buffer

**What happens**: Kai presses `Enter` on a file. The browser closes. The file loads into the editor buffer. The status bar shows the filename and mode switches to `NORMAL`. Alfred is now in its standard editing state, identical to having opened `alfred path/to/file.rs` directly.

```
+-- Alfred: src/models/user.rs ----------------------------------------+
|  1  use serde::{Deserialize, Serialize};                             |
|  2                                                                   |
|  3  #[derive(Debug, Clone, Serialize, Deserialize)]                  |
|  4  pub struct User {                                                |
|  5      pub id: u64,                                                 |
|  6      pub name: String,                                            |
|  7      pub email: String,                                           |
|  8  }                                                                |
|  9                                                                   |
| 10  impl User {                                                      |
| 11      pub fn new(id: u64, name: &str, email: &str) -> Self {       |
| 12          User {                                                   |
| 13              id,                                                  |
| 14              name: name.to_string(),                              |
| 15              email: email.to_string(),                            |
| 16          }                                                        |
| 17      }                                                            |
| 18  }                                                                |
| 19                                                                   |
+----------------------------------------------------------------------+
| NORMAL  src/models/user.rs                              1:1          |
+----------------------------------------------------------------------+
```

**Key details**:
- Transition is instant -- no visible delay
- All standard editor features active (syntax highlighting, gutter, status bar)
- File path shown relative to the original directory Alfred was opened with
- Buffer is unmodified (modified flag = false)

**Emotional state**: Seamless -> Productive ("I'm editing, as if I'd opened the file directly")

**Error paths**:
- File is binary: Show message "Binary file, cannot edit" on status bar, stay in browser
- File is unreadable: Show error message on status bar, stay in browser
- File is very large: Load normally (ropey handles large files efficiently)

---

## Step 5: Return to Browser (optional, future consideration)

**Note**: In the initial implementation, opening a file is a one-way transition. The user edits and then quits Alfred. Returning to the browser after opening a file would require multi-buffer support, which is a separate feature. This step is documented for journey completeness but is explicitly out of scope for the initial folder browser stories.

---

## Key Interaction Summary

```
                    alfred .
                       |
                       v
              +----------------+
              | Detect:        |
              | Is arg a dir?  |
              +-------+--------+
                 yes  |  no
                      |  +---> Open file (existing behavior)
                      v
              +----------------+
              | Display:       |       q / Esc
              | Tree view of   |  +--------------> Quit Alfred
              | directory      |  |
              +-------+--------+  |
                      |           |
               j/k    |  Enter on dir
              navigate|  +---> Enter subdirectory (recurse to Display)
                      |
                      | Enter on file
                      v
              +----------------+
              | Open:          |
              | Load file into |
              | editor buffer  |
              +-------+--------+
                      |
                      v
              +----------------+
              | Edit:          |
              | Standard Alfred|
              | editing mode   |
              +----------------+
```
