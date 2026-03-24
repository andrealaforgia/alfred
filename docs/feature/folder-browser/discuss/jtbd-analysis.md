# JTBD Analysis: Folder Browser

## Job Classification

**Job Type**: Build Something New (Greenfield feature on Brownfield codebase)
**Workflow**: `[research] -> discuss -> design -> distill -> baseline -> roadmap -> split -> execute -> review`
**Current Phase**: DISCUSS (requirements gathering)

Alfred is an existing editor (brownfield), but the folder browser is an entirely new capability (greenfield feature). Discovery is required because:
- The feature introduces a new interaction paradigm (browsing vs editing)
- It affects the entry point of the application (CLI argument handling)
- It creates a new relationship between the editor and the filesystem

---

## Job Stories

### JS-01: Navigate to a File in an Unfamiliar Project

**When** I open Alfred in a project directory I haven't worked in recently,
**I want to** see the project's file structure and browse through it,
**so I can** find and open the right file without memorizing paths or switching to another tool.

#### Functional Job
Locate and open a specific file within a directory tree using the editor itself.

#### Emotional Job
Feel oriented and in control when landing in a codebase -- not lost, not guessing, not dependent on an external tool.

#### Social Job
Work fluidly without pausing to use `find` or `ls` in front of colleagues -- appear competent with your tools.

---

### JS-02: Quickly Open a Known File from a Project Root

**When** I know roughly where a file is in my project and I've launched Alfred on the project root,
**I want to** navigate to the file with minimal keystrokes,
**so I can** start editing immediately without the overhead of typing the full path.

#### Functional Job
Open a file by navigating a tree structure using keyboard-driven interaction.

#### Emotional Job
Feel efficient -- the tool does not stand between me and my work. No wasted motion.

#### Social Job
Demonstrate terminal fluency -- the editor keeps pace with the developer's intent.

---

### JS-03: Explore a Project Structure to Understand Layout

**When** I've cloned a new repository or inherited a codebase,
**I want to** browse the folder structure to understand how the project is organized,
**so I can** build a mental model of the codebase before diving into specific files.

#### Functional Job
Visualize the hierarchical structure of a directory -- folders, nesting depth, file types present.

#### Emotional Job
Feel curious and welcomed rather than overwhelmed. Build confidence about the project gradually.

#### Social Job
Understand enough about the project to have informed conversations with the team.

---

## 8-Step Universal Job Map

### Step 1: DEFINE -- Determine what to open

| Aspect | Detail |
|--------|--------|
| User's goal | Open Alfred to browse or edit files in a directory |
| Information needed | Whether the argument is a file or a directory |
| Decision | Browse (directory) vs. edit (file) vs. new buffer (no argument) |
| Missing requirement risk | What happens with symlinks? What about `alfred .` inside an empty dir? |

### Step 2: LOCATE -- Find the target in the tree

| Aspect | Detail |
|--------|--------|
| User's action | Scan the tree visually; navigate with j/k; expand/collapse folders |
| Information needed | File names, folder structure, visual indicators for type (dir vs file) |
| Decision | Which item to select or which folder to enter |
| Missing requirement risk | How are hidden files (dotfiles) handled? Sort order? |

### Step 3: PREPARE -- Position cursor on target

| Aspect | Detail |
|--------|--------|
| User's action | Move cursor to the desired file entry in the tree |
| Information needed | Current cursor position, number of entries, visual feedback for selection |
| Decision | Confirm this is the right file before opening |
| Missing requirement risk | Can the user see file metadata (size, modified date) to confirm identity? |

### Step 4: CONFIRM -- Validate before opening

| Aspect | Detail |
|--------|--------|
| User's action | Press Enter to confirm file selection |
| Information needed | File path, file type, whether file is readable |
| Decision | Open the file for editing or show an error |
| Missing requirement risk | What happens if the file is binary? Permission denied? |

### Step 5: EXECUTE -- Open the file

| Aspect | Detail |
|--------|--------|
| User's action | File opens in the editor buffer, browser closes |
| Information needed | File content loaded into buffer, filename in status bar |
| Decision | None -- transition should be seamless |
| Missing requirement risk | How does the user return to the browser after opening a file? |

### Step 6: MONITOR -- Verify the right file opened

| Aspect | Detail |
|--------|--------|
| User's action | Glance at status bar showing filename, see file content |
| Information needed | Filename displayed, syntax highlighting active |
| Decision | Correct file? If not, return to browser |
| Missing requirement risk | Is there a way to go back to the browser? |

### Step 7: MODIFY -- Adjust if wrong file

| Aspect | Detail |
|--------|--------|
| User's action | Return to browser to pick a different file |
| Information needed | Browser state preserved (cursor position, expanded folders) |
| Decision | Navigate to correct file |
| Missing requirement risk | Is browser state preserved or reset when returning? |

### Step 8: CONCLUDE -- Finish the browsing session

| Aspect | Detail |
|--------|--------|
| User's action | Begin editing the selected file; browsing is complete |
| Information needed | None -- seamless transition to normal editing |
| Decision | None |
| Missing requirement risk | None significant |

---

## Four Forces Analysis

### Demand-Generating Forces

**Push (Frustration with current situation)**
- Developer must type the full file path to open a file: `alfred src/crates/alfred-core/src/editor_state.rs`
- If the developer doesn't know the exact path, they must switch to another terminal and run `ls`, `find`, or `tree` to discover it
- Context-switching between a file finder and the editor breaks flow
- Other terminal editors (vim, helix, kakoune) provide built-in file browsing, making Alfred feel incomplete
- The current Alfred can only open a single file via CLI argument -- running `alfred .` produces an error or opens a nonsensical buffer

**Pull (Attractiveness of new solution)**
- Single command `alfred .` to browse and open -- no context-switching
- Vim-style navigation (j/k/Enter/Escape) feels native to Alfred's existing modal model
- Tree view gives project overview at a glance
- Plugin-first philosophy means this can be implemented as a composable Lisp plugin, not a monolithic feature
- Aligns with how developers naturally launch editors: `vim .`, `hx .`, `code .`

### Demand-Reducing Forces

**Anxiety (Fears about the new solution)**
- Will the folder browser feel clunky or slow compared to vim's NERDTree or netrw?
- Will navigating deep directories (node_modules, .git) be painful?
- What happens to the current single-buffer model -- will opening from browser conflict with unsaved changes?
- Will key bindings conflict with existing normal-mode bindings?

**Habit (Inertia of current approach)**
- Developer already has a workflow: `tree | less` then `alfred path/to/file.rs`
- fuzzy-finder tools (fzf, telescope) feel faster for known projects
- Some developers never browse -- they always know exactly what file they want
- Opening Alfred with an explicit path is simple and predictable

### Assessment

| Dimension | Rating |
|-----------|--------|
| Switch likelihood | **High** -- running `alfred .` is such a natural action that its absence is noticed immediately |
| Key blocker | Anxiety about browser feeling clunky; must feel fast and native to vim muscle memory |
| Key enabler | Push from having to leave the editor to find files; Pull from single-command project browsing |
| Design implication | Browser must feel like a natural extension of Alfred's normal mode, not a separate tool bolted on. Navigation must use vim idioms (j/k/Enter/Esc). Performance must be instant for typical project sizes (<10k entries). Hidden files and large directories need smart defaults. |

---

## Outcome Statements

| ID | Outcome Statement | Priority |
|----|-------------------|----------|
| OS-01 | Minimize the time it takes to open a file when the exact path is unknown | Must Have |
| OS-02 | Minimize the number of tool switches required to find and open a file | Must Have |
| OS-03 | Minimize the likelihood of opening the wrong file when browsing | Should Have |
| OS-04 | Minimize the time it takes to orient in an unfamiliar project structure | Should Have |
| OS-05 | Maximize the likelihood that navigation feels native to vim muscle memory | Must Have |
| OS-06 | Minimize the cognitive load when transitioning between browsing and editing | Must Have |
| OS-07 | Minimize the time spent navigating past irrelevant entries (hidden files, build artifacts) | Should Have |

---

## Persona

**Kai Nakamura** -- Backend developer, 4 years experience. Uses the terminal for most development work. Comfortable with vim keybindings and modal editing. Works across 3-4 Rust and Python projects. Frequently clones new repos to review PRs and contribute patches. Finds it annoying to leave the editor to find files. Currently uses `tree` + `alfred specific/path.rs` as a workaround. Wants a lightweight editor that handles the full edit cycle without requiring a separate file manager.
