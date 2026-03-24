# Alfred

A plugin-first, Emacs-inspired terminal text editor built in Rust with an embedded Lisp extension language.

Alfred proves that AI agents can build architecturally sound software. Every feature beyond core text editing — keybindings, line numbers, status bar, themes, and even vim-style modal editing — is a plugin written in Alfred Lisp.

## Features

- **Vim-style modal editing** — Normal, Insert, and Visual modes with operator composability (`dw`, `cw`, `y$`, `diw`, etc.)
- **Plugin system** — Discover, load, and manage plugins written in Alfred Lisp
- **Lisp extension language** — Customize everything: keybindings, themes, commands, cursor shapes
- **Color themes** — Fully Lisp-driven with `set-theme-color`, `define-theme`, `load-theme`
- **Line numbers** — Plugin-driven gutter with auto-adjusting width
- **Status bar** — Shows filename, cursor position, modified indicator, and mode
- **Search** — `/pattern`, `n`/`N` repeat, find-char `f`/`t`/`F`/`T`
- **Undo/redo** — Rope-based snapshots with `u` and `Ctrl-r`
- **File operations** — `:w`, `:wq`, `:q!`, `:e filename`
- **Marks** — `m{a-z}` to set, `'{a-z}` to jump
- **Macros** — `q{a-z}` to record, `@{a-z}` to replay
- **Registers** — `"a-z` named registers for yank/delete/paste
- **Text objects** — `iw`, `aw`, `i"`, `a"`, `i(`, `a(`, `i{`, `a{`, `i[`, `a[`
- **Visual mode** — Character-wise (`v`) and line-wise (`V`) selection
- **Rainbow CSV** — Colorize CSV columns (`:rainbow-csv`)
- **Search & replace** — `:s/old/new/g`, `:%s/old/new/g`
- **Global commands** — `:g/pattern/d`, `:v/pattern/d`
- **Config file** — `~/.config/alfred/init.lisp` loaded at startup

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

### Install

```bash
git clone git@github.com:andrealaforgia/alfred.git
cd alfred
make install
```

This builds the release binary and installs it to `~/.cargo/bin/alfred`. It also adds `~/.cargo/bin` to your PATH if not already present.

### Development Setup

```bash
make dev_install
```

This installs the binary plus development tools (rustfmt, clippy) and sets up pre-commit hooks.

### Uninstall

```bash
make uninstall
```

## Usage

### Open a file

```bash
alfred myfile.txt
```

### Open without a file

```bash
alfred
```

### Basic workflow

Alfred starts in **Normal mode** (like vim):

| Action | Keys |
|--------|------|
| Move cursor | `h` `j` `k` `l` or arrow keys |
| Enter Insert mode | `i` (before cursor), `a` (after), `A` (end of line), `o` (new line below) |
| Return to Normal mode | `Escape` |
| Save | `:w` Enter |
| Save and quit | `:wq` Enter |
| Quit | `:q` Enter |
| Force quit (discard changes) | `:q!` Enter |
| Open another file | `:e filename` Enter |

### Editing commands

| Command | Description |
|---------|-------------|
| `x` | Delete character under cursor |
| `dw` | Delete word |
| `dd` | Delete entire line |
| `d$` | Delete to end of line |
| `D` | Delete to end of line (same as `d$`) |
| `cw` | Change word (delete + insert mode) |
| `cc` | Change entire line |
| `C` | Change to end of line |
| `r{char}` | Replace character under cursor |
| `s` | Substitute character (delete + insert) |
| `S` | Substitute entire line |
| `J` | Join current line with next |
| `u` | Undo |
| `Ctrl-r` | Redo |
| `.` | Repeat last change |
| `~` | Toggle case of character |
| `>` | Indent line |
| `<` | Unindent line |
| `P` | Paste before cursor |
| `p` | Paste after/below cursor |
| `X` | Delete character before cursor |

### Navigation

| Command | Description |
|---------|-------------|
| `w` / `b` / `e` | Word forward / backward / end |
| `0` / `$` | Line start / end |
| `^` | First non-blank character |
| `gg` / `G` | Document start / end |
| `H` / `M` / `L` | Screen top / middle / bottom |
| `Ctrl-d` / `Ctrl-u` | Half-page down / up |
| `f{char}` / `F{char}` | Find character forward / backward |
| `t{char}` / `T{char}` | Till character forward / backward |
| `;` / `,` | Repeat / reverse last find |
| `%` | Jump to matching bracket |
| `/{pattern}` | Search forward |
| `n` / `N` | Next / previous search match |

### Visual mode

| Command | Description |
|---------|-------------|
| `v` | Enter character-wise visual mode |
| `V` | Enter line-wise visual mode |
| `d` | Delete selection |
| `y` | Yank (copy) selection |
| `c` | Change selection (delete + insert) |
| `Escape` | Cancel selection |

### Text objects (used with operators)

| Object | Description |
|--------|-------------|
| `iw` / `aw` | Inner / around word |
| `i"` / `a"` | Inner / around double quotes |
| `i'` / `a'` | Inner / around single quotes |
| `i(` / `a(` | Inner / around parentheses |
| `i{` / `a{` | Inner / around braces |
| `i[` / `a[` | Inner / around brackets |

### Marks and macros

| Command | Description |
|---------|-------------|
| `m{a-z}` | Set mark |
| `'{a-z}` | Jump to mark |
| `q{a-z}` | Start recording macro |
| `q` | Stop recording |
| `@{a-z}` | Play macro |
| `@@` | Repeat last macro |

### Command line

| Command | Description |
|---------|-------------|
| `:w` | Save |
| `:w path` | Save to path |
| `:wq` | Save and quit |
| `:q` | Quit (warns if unsaved) |
| `:q!` | Force quit |
| `:e path` | Open file |
| `:s/old/new/` | Replace first on line |
| `:s/old/new/g` | Replace all on line |
| `:%s/old/new/g` | Replace all in file |
| `:g/pattern/d` | Delete lines matching pattern |
| `:v/pattern/d` | Delete lines NOT matching pattern |
| `:eval (expr)` | Evaluate Lisp expression |
| `:rainbow-csv` | Colorize CSV columns |

## Plugins

Alfred loads plugins from the `plugins/` directory. Each plugin is a subdirectory containing an `init.lisp` file.

### Built-in plugins

| Plugin | Description |
|--------|-------------|
| `vim-keybindings` | Vim-style modal editing (Normal, Insert, Visual modes) |
| `line-numbers` | Line numbers in the gutter |
| `status-bar` | Status bar with filename, position, mode |
| `default-theme` | Catppuccin-inspired color theme |
| `test-plugin` | Example plugin that registers a `:hello` command |
| `rainbow-csv` | Colorize CSV columns with rainbow colors |

### Creating a plugin

Create a directory under `plugins/` with an `init.lisp` file:

```
plugins/my-plugin/init.lisp
```

Example plugin that adds a word count command:

```lisp
;;; name: word-count
;;; version: 0.1.0
;;; description: Shows word count in the message bar

(define-command "word-count"
  (lambda ()
    (message
      (str "Words: "
        (length
          (split (buffer-content) " "))))))
```

Run `:word-count` to see the word count in the message bar.

### Available Lisp primitives

**Buffer**: `buffer-insert`, `buffer-delete`, `buffer-content`, `buffer-get-line`, `buffer-filename`, `buffer-modified?`, `save-buffer`

**Cursor**: `cursor-position`, `cursor-move`

**Mode**: `current-mode`, `set-mode`

**Commands**: `define-command`

**Keymaps**: `make-keymap`, `define-key`, `set-active-keymap`

**Hooks**: `add-hook`, `dispatch-hook`, `remove-hook`

**Theme**: `set-theme-color`, `get-theme-color`, `define-theme`, `load-theme`

**Cursor shape**: `set-cursor-shape`, `get-cursor-shape`

**Tab**: `set-tab-width`, `get-tab-width`

**Display**: `message`, `set-line-style`, `clear-line-styles`

## Configuration

Create `~/.config/alfred/init.lisp` to customize Alfred at startup:

```lisp
;; Set tab width to 2 spaces
(set-tab-width 2)

;; Use a blinking cursor in insert mode
(set-cursor-shape "insert" "blinking-bar")

;; Custom theme colors
(set-theme-color "status-bar-bg" "#1e1e2e")
(set-theme-color "gutter-fg" "#6c7086")
```

## Architecture

Alfred is built as a 5-crate Rust workspace:

| Crate | Purpose |
|-------|---------|
| `alfred-core` | Pure domain logic: buffer, cursor, viewport, commands, hooks, theme |
| `alfred-lisp` | Lisp interpreter integration and bridge to core primitives |
| `alfred-plugin` | Plugin discovery, loading, lifecycle, dependency ordering |
| `alfred-tui` | Terminal UI: renderer, event loop, key handling |
| `alfred-bin` | Binary entry point that wires everything together |

The architecture follows **functional-core / imperative-shell**: all domain logic in `alfred-core` is pure (no I/O), while terminal I/O lives at the boundary in `alfred-tui`.

## Development

### Run tests

```bash
make test          # Unit tests
make e2e           # End-to-end tests (requires Docker)
make format        # Check formatting
make lint          # Run clippy
make ci-local      # Run full CI locally via act (requires Docker)
```

### Project stats

- ~110 vim commands implemented
- ~566 unit tests
- ~66 E2E tests (Docker-based, pexpect)
- Farley Test Quality Index: 8.4/10 (Excellent)
- Zero mock tautology

## License

This project is a personal learning and demonstration project.
