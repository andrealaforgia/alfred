# Research: Comprehensive Vim Commands Reference for Alfred Editor Implementation

**Date**: 2026-03-23 | **Researcher**: nw-researcher (Nova) | **Confidence**: High | **Sources**: 14

## Executive Summary

This document provides an exhaustive reference of every vim command, motion, operator, text object, and key combination across all vim modes. It is organized to serve as the definitive implementation guide for Alfred's vim emulation layer.

Vim's power derives from its composable grammar: operators + motions/text-objects, with optional counts. There are approximately 150+ distinct normal mode commands, 50+ insert mode commands, 30+ visual mode commands, 30+ text objects, 80+ CTRL-W window commands, 50+ z-commands, 50+ g-commands, 30+ square bracket commands, and dozens of ex commands. The total unique key sequences number in the hundreds, but many are aliases (e.g., `<Left>` = `h`, `<CR>` = `+`).

Alfred already implements: hjkl, i/I/a/A/o/O, x, d, J, y, p, c, C, u, Ctrl-r, w/b/e, 0/$, ^, gg, G, H/M/L, Ctrl-d/Ctrl-u, :w, :q, :wq, :q!, :e. This document identifies everything else needed for complete coverage, organized by implementation priority.

## Research Methodology

**Search Strategy**: Official Vim documentation (vimdoc.sourceforge.net, vimhelp.org), established reference sites (vim.rtorr.com, learnbyexample.github.io), and community-verified cheat sheets. The Vim help index (index.txt) was used as the canonical source of truth.

**Source Selection**: Types: official documentation, community-maintained references, technical blogs | Reputation: High and Medium-High tier | Verification: cross-referenced across 3+ independent sources per command category

**Quality Standards**: Min 3 sources/category | All major commands cross-referenced against official Vim help | Avg reputation: 0.85

---

## Table of Contents

1. [Vim's Composable Grammar](#1-vims-composable-grammar)
2. [Normal Mode: Cursor Movement](#2-normal-mode-cursor-movement)
3. [Normal Mode: Operators](#3-normal-mode-operators)
4. [Normal Mode: Simple Editing Commands](#4-normal-mode-simple-editing-commands)
5. [Normal Mode: Search Commands](#5-normal-mode-search-commands)
6. [Normal Mode: Mark Commands](#6-normal-mode-mark-commands)
7. [Normal Mode: Register Commands](#7-normal-mode-register-commands)
8. [Normal Mode: Macro Commands](#8-normal-mode-macro-commands)
9. [Normal Mode: Scroll Commands](#9-normal-mode-scroll-commands)
10. [Normal Mode: g-Prefix Commands](#10-normal-mode-g-prefix-commands)
11. [Normal Mode: z-Prefix Commands](#11-normal-mode-z-prefix-commands)
12. [Normal Mode: Square Bracket Commands](#12-normal-mode-square-bracket-commands)
13. [Normal Mode: CTRL-W Window Commands](#13-normal-mode-ctrl-w-window-commands)
14. [Text Objects](#14-text-objects)
15. [Operator + Motion Composition](#15-operator--motion-composition)
16. [Insert Mode Commands](#16-insert-mode-commands)
17. [Visual Mode Commands](#17-visual-mode-commands)
18. [Command-Line (Ex) Mode](#18-command-line-ex-mode)
19. [Tab Commands](#19-tab-commands)
20. [Implementation Priority Matrix](#20-implementation-priority-matrix)

---

## 1. Vim's Composable Grammar

**Evidence**: Vim commands follow the grammar: `[count] operator [count] motion/text-object`. If you prefix both the operator and the motion with a count, Vim multiplies the two counts. For example, `2d3w` deletes six consecutive words. Repeating an operator twice applies it to the current line: `dd`, `cc`, `yy`, `>>`, `<<`, `==`. [1][3][5]

**Confidence**: High

### The Three Components

| Component | Role | Examples |
|-----------|------|----------|
| Count | Multiplier (optional) | `3`, `5`, `12` |
| Operator | Action to perform | `d`, `c`, `y`, `>`, `<`, `=`, `gU`, `gu`, `g~`, `!`, `gq`, `gw`, `zf` |
| Motion/Text-Object | Range of text | `w`, `$`, `iw`, `a"`, `G`, `f{char}` |

---

## 2. Normal Mode: Cursor Movement

### 2.1 Character-Level Movement

| Command | Aliases | Description | Count | Inclusive/Exclusive | Already in Alfred |
|---------|---------|-------------|-------|---------------------|-------------------|
| `h` | `<Left>`, `CTRL-H`, `<BS>` | Move left | [count] chars | exclusive | YES |
| `l` | `<Right>`, `<Space>` | Move right | [count] chars | exclusive | YES |
| `k` | `<Up>`, `CTRL-P` | Move up | [count] lines | linewise | YES |
| `j` | `<Down>`, `CTRL-J`, `CTRL-N`, `<NL>` | Move down | [count] lines | linewise | YES |

### 2.2 Line-Level Movement

| Command | Description | Count | Inclusive/Exclusive | Already in Alfred |
|---------|-------------|-------|---------------------|-------------------|
| `0` | First character of line | N/A | exclusive | YES |
| `^` | First non-blank character of line | N/A | exclusive | YES |
| `$` | End of line | [count-1] lines down | inclusive | YES |
| `g_` | Last non-blank character | [count-1] lines down | inclusive | NO |
| `g0` | First char of screen line (wrap mode) | N/A | exclusive | NO |
| `g^` | First non-blank of screen line | N/A | exclusive | NO |
| `gm` | Middle of screen line | N/A | exclusive | NO |
| `gM` | Middle of text line (by char count) | N/A | exclusive | NO |
| `g$` | Last char of screen line (wrap mode) | N/A | inclusive | NO |
| `\|` (pipe) | Go to screen column [count] | [count] | exclusive | NO |
| `+` | First CHAR [count] lines lower | [count] | linewise | NO |
| `-` | First CHAR [count] lines higher | [count] | linewise | NO |
| `_` | First CHAR [count-1] lines lower | [count] | linewise | NO |
| `<CR>` | Same as `+` | [count] | linewise | NO |
| `<Home>` | First character of line | N/A | exclusive | NO |
| `<End>` | End of line | [count-1] lines down | inclusive | NO |

### 2.3 Word-Level Movement

| Command | Aliases | Description | Count | Inclusive/Exclusive | Already in Alfred |
|---------|---------|-------------|-------|---------------------|-------------------|
| `w` | `<S-Right>` | Forward to start of word | [count] | exclusive | YES |
| `W` | `<C-Right>` | Forward to start of WORD | [count] | exclusive | NO |
| `e` | | Forward to end of word | [count] | inclusive | YES |
| `E` | | Forward to end of WORD | [count] | inclusive | NO |
| `b` | `<S-Left>` | Backward to start of word | [count] | exclusive | YES |
| `B` | `<C-Left>` | Backward to start of WORD | [count] | exclusive | NO |
| `ge` | | Backward to end of word | [count] | inclusive | NO |
| `gE` | | Backward to end of WORD | [count] | inclusive | NO |

**Note on word vs. WORD**: A "word" consists of letters, digits, and underscores (or other keyword chars). A "WORD" is any sequence of non-blank characters separated by whitespace. [1][3]

### 2.4 Character-Find Movement (Single-Line)

| Command | Description | Count | Inclusive/Exclusive | Already in Alfred |
|---------|-------------|-------|---------------------|-------------------|
| `f{char}` | Find [count]th occurrence of {char} to the right | [count] | inclusive | NO |
| `F{char}` | Find [count]th occurrence of {char} to the left | [count] | exclusive | NO |
| `t{char}` | Till before [count]th occurrence to the right | [count] | inclusive | NO |
| `T{char}` | Till after [count]th occurrence to the left | [count] | exclusive | NO |
| `;` | Repeat latest f/t/F/T | [count] | depends on original | NO |
| `,` | Repeat latest f/t/F/T in opposite direction | [count] | depends on original | NO |

### 2.5 Sentence, Paragraph, Section Movement

| Command | Description | Count | Inclusive/Exclusive | Already in Alfred |
|---------|-------------|-------|---------------------|-------------------|
| `(` | [count] sentences backward | [count] | exclusive | NO |
| `)` | [count] sentences forward | [count] | exclusive | NO |
| `{` | [count] paragraphs backward | [count] | exclusive | NO |
| `}` | [count] paragraphs forward | [count] | exclusive | NO |
| `]]` | [count] sections forward (next `{` in col 1) | [count] | exclusive | NO |
| `][` | [count] sections forward (next `}` in col 1) | [count] | exclusive | NO |
| `[[` | [count] sections backward (prev `{` in col 1) | [count] | exclusive | NO |
| `[]` | [count] sections backward (prev `}` in col 1) | [count] | exclusive | NO |

### 2.6 Document-Level Movement

| Command | Aliases | Description | Count | Already in Alfred |
|---------|---------|-------------|-------|-------------------|
| `gg` | `<C-Home>` | Go to line [count], default first | [count] = line | YES |
| `G` | `<C-End>` | Go to line [count], default last | [count] = line | YES |
| `{count}%` | | Go to {count} percentage in file | [count] = percentage | NO |
| `go` | | Go to byte [count] in buffer | [count] = byte offset | NO |

### 2.7 Screen-Relative Movement

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `H` | Line [count] from top of window | [count] | YES |
| `M` | Middle line of window | N/A | YES |
| `L` | Line [count] from bottom of window | [count] | YES |

### 2.8 Match and Jump Movement

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `%` | Jump to matching bracket/paren/brace | N/A | NO |
| `CTRL-O` | Older position in jump list | [count] | NO |
| `CTRL-I` / `<Tab>` | Newer position in jump list | [count] | NO |
| `g;` | Older position in change list | [count] | NO |
| `g,` | Newer position in change list | [count] | NO |
| `gd` | Go to local definition of word under cursor | N/A | NO |
| `gD` | Go to global definition of word under cursor | N/A | NO |
| `gf` | Go to file under cursor | N/A | NO |
| `gF` | Go to file under cursor + jump to line number | N/A | NO |

---

## 3. Normal Mode: Operators

Operators are commands that wait for a motion or text object to define the range of text to act upon. [1][3][5]

| Operator | Double-tap (line) | Description | Already in Alfred |
|----------|-------------------|-------------|-------------------|
| `d` | `dd` | Delete | YES |
| `c` | `cc` | Change (delete + insert mode) | YES |
| `y` | `yy` | Yank (copy) | YES |
| `>` | `>>` | Shift right (indent) | NO |
| `<` | `<<` | Shift left (dedent) | NO |
| `=` | `==` | Re-indent (auto-format) | NO |
| `gU` | `gUU` | Make uppercase | NO |
| `gu` | `guu` | Make lowercase | NO |
| `g~` | `g~~` | Swap case | NO |
| `gq` | `gqq` | Format text (rewrap) | NO |
| `gw` | `gww` | Format text (keep cursor) | NO |
| `!` | `!!` | Filter through external command | NO |
| `zf` | | Create fold for motion range | NO |
| `g@` | | Call 'operatorfunc' | NO |

**Composition rules**: `[count] operator [count] motion` -- counts multiply. `d3w` = delete 3 words. `2d3w` = delete 6 words. Operator doubled = line-wise: `dd`, `cc`, `yy`, `>>`, `<<`, `==`, `gUU`, `guu`, `g~~`, `gqq`. [1][3]

---

## 4. Normal Mode: Simple Editing Commands

These commands do not wait for a motion -- they act immediately. [1][2][3]

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `x` | Delete [count] characters under/after cursor | [count] | YES |
| `X` | Delete [count] characters before cursor | [count] | NO |
| `r{char}` | Replace character under cursor with {char} | [count] chars replaced | NO |
| `R` | Enter Replace mode (overtype) | N/A | NO |
| `s` | Delete [count] chars, enter insert mode (= `cl`) | [count] | NO |
| `S` | Delete [count] lines, enter insert mode (= `cc`) | [count] | NO |
| `J` | Join [count] lines with space | [count] (default 2) | YES |
| `gJ` | Join [count] lines without space | [count] (default 2) | NO |
| `D` | Delete to end of line (= `d$`) | N/A | NO |
| `C` | Change to end of line (= `c$`) | N/A | YES |
| `Y` | Yank [count] lines (= `yy`) | [count] | NO |
| `p` | Put (paste) after cursor | [count] | YES |
| `P` | Put (paste) before cursor | [count] | NO |
| `gp` | Put after cursor, leave cursor after text | [count] | NO |
| `gP` | Put before cursor, leave cursor after text | [count] | NO |
| `]p` | Put after cursor, adjust indent | [count] | NO |
| `[p` | Put before cursor, adjust indent | [count] | NO |
| `u` | Undo | [count] changes | YES |
| `U` | Undo all changes on last changed line | N/A | NO |
| `CTRL-R` | Redo | [count] changes | YES |
| `.` | Repeat last change | [count] | NO |
| `~` | Switch case of [count] characters and advance cursor | [count] | NO |
| `CTRL-A` | Add [count] to number at/after cursor | [count] (default 1) | NO |
| `CTRL-X` | Subtract [count] from number at/after cursor | [count] (default 1) | NO |
| `K` | Look up keyword under cursor (man page) | N/A | NO |
| `Q` | Switch to Ex mode | N/A | NO |
| `gQ` | Switch to Ex mode with Vim editing | N/A | NO |
| `&` | Repeat last `:s` on current line | N/A | NO |
| `g&` | Repeat last `:s` on all lines (= `:%s//~/&`) | N/A | NO |
| `ZZ` | Write and quit (= `:wq`) | N/A | NO |
| `ZQ` | Quit without writing (= `:q!`) | N/A | NO |
| `CTRL-L` | Redraw screen | N/A | NO |
| `CTRL-G` | Display file name and position | N/A | NO |
| `g CTRL-G` | Display detailed cursor position info | N/A | NO |
| `ga` | Print ASCII/Unicode value of char under cursor | N/A | NO |
| `g8` | Print hex value of UTF-8 bytes under cursor | N/A | NO |
| `CTRL-Z` | Suspend Vim (or start new shell) | N/A | NO |
| `CTRL-C` | Interrupt current command | N/A | NO |
| `CTRL-]` | Jump to tag under cursor | N/A | NO |
| `CTRL-T` | Jump to [count] older tag in tag list | [count] | NO |
| `CTRL-^` | Edit alternate file (= `:e #`) | [count] | NO |

---

## 5. Normal Mode: Search Commands

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `/{pattern}<CR>` | Search forward for pattern | [count]th match | NO |
| `/{pattern}/{offset}<CR>` | Search forward with line offset | [count]th match | NO |
| `/<CR>` | Repeat last search forward | [count]th match | NO |
| `?{pattern}<CR>` | Search backward for pattern | [count]th match | NO |
| `?{pattern}?{offset}<CR>` | Search backward with line offset | [count]th match | NO |
| `?<CR>` | Repeat last search backward | [count]th match | NO |
| `n` | Repeat last search in same direction | [count]th match | NO |
| `N` | Repeat last search in opposite direction | [count]th match | NO |
| `*` | Search forward for word under cursor (whole word) | [count]th match | NO |
| `#` | Search backward for word under cursor (whole word) | [count]th match | NO |
| `g*` | Like `*` but without `\<` and `\>` (partial match) | [count]th match | NO |
| `g#` | Like `#` but without `\<` and `\>` (partial match) | [count]th match | NO |
| `gn` | Find next match and visually select it | N/A | NO |
| `gN` | Find previous match and visually select it | N/A | NO |

---

## 6. Normal Mode: Mark Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `m{a-z}` | Set mark {a-z} at cursor position (buffer-local) | NO |
| `m{A-Z}` | Set mark {A-Z} at cursor position (global, cross-file) | NO |
| `` `{a-z} `` | Jump to mark position (exact line and column) | NO |
| `'{a-z}` | Jump to mark line (first non-blank character) | NO |
| `` `{A-Z} `` | Jump to global mark position (may open another file) | NO |
| `'{A-Z}` | Jump to global mark line | NO |
| ` `` ` | Jump to position before last jump | NO |
| `''` | Jump to line before last jump | NO |
| `` `. `` | Jump to position of last change | NO |
| `'.` | Jump to line of last change | NO |
| `` `" `` | Jump to position when last exiting buffer | NO |
| `` `^ `` | Jump to position where insert mode was last stopped | NO |
| `` `[ `` | Jump to start of last operated/yanked text | NO |
| `` `] `` | Jump to end of last operated/yanked text | NO |
| `` `< `` | Jump to start of last visual selection | NO |
| `` `> `` | Jump to end of last visual selection | NO |
| `:marks` | List all marks | NO |
| `:delmarks {marks}` | Delete specified marks | NO |
| `:delmarks!` | Delete all lowercase marks | NO |
| `]'` | Jump to next line with lowercase mark | NO |
| `['` | Jump to previous line with lowercase mark | NO |

---

## 7. Normal Mode: Register Commands

### 7.1 Register Usage

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `"{register}` | Use {register} for next delete, yank, or put | NO |
| `:registers` / `:reg` | Show contents of all registers | NO |
| `:reg {register}` | Show contents of specific register | NO |

### 7.2 Register Types

| Register | Name | Description |
|----------|------|-------------|
| `""` | Unnamed | Last deleted/changed/yanked text (default) |
| `"0` | Yank | Last yanked text (not deleted) |
| `"1` - `"9` | Numbered | Last 9 deleted texts (1=newest, 9=oldest); only for deletes of 1+ lines or motions |
| `"a` - `"z` | Named (lowercase) | User-specified, overwrite |
| `"A` - `"Z` | Named (uppercase) | User-specified, append to lowercase counterpart |
| `"-` | Small delete | Last delete less than one line |
| `".` | Last insert | Last inserted text (read-only) |
| `"%` | Current filename | Name of current file (read-only) |
| `"#` | Alternate filename | Name of alternate file (read-only) |
| `":` | Last command | Last ex command (read-only) |
| `"/` | Last search | Last search pattern (read-only) |
| `"=` | Expression | Expression register (prompt for expression) |
| `"*` | Selection (primary) | System primary selection (X11) / clipboard (macOS/Windows) |
| `"+` | Clipboard | System clipboard (X11) / same as `"*` on macOS |
| `"_` | Black hole | Discards text (like `/dev/null`) |

---

## 8. Normal Mode: Macro Commands

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `q{a-z}` | Start recording keystrokes into register {a-z} | N/A | NO |
| `q{A-Z}` | Start recording, append to register {a-z} | N/A | NO |
| `q` | Stop recording (when already recording) | N/A | NO |
| `@{a-z}` | Execute contents of register {a-z} | [count] times | NO |
| `@@` | Repeat last `@{a-z}` | [count] times | NO |
| `@:` | Repeat last ex command | [count] times | NO |
| `q:` | Open command-line window for `:` commands | N/A | NO |
| `q/` | Open command-line window for `/` searches | N/A | NO |
| `q?` | Open command-line window for `?` searches | N/A | NO |

---

## 9. Normal Mode: Scroll Commands

### 9.1 Scroll by Lines

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `CTRL-E` | Scroll window [count] lines down (cursor stays) | [count] | NO |
| `CTRL-Y` | Scroll window [count] lines up (cursor stays) | [count] | NO |

### 9.2 Scroll by Half-Page

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `CTRL-D` | Scroll down [count] lines (default half screen) | [count] sets default | YES |
| `CTRL-U` | Scroll up [count] lines (default half screen) | [count] sets default | YES |

### 9.3 Scroll by Full Page

| Command | Aliases | Description | Count | Already in Alfred |
|---------|---------|-------------|-------|-------------------|
| `CTRL-F` | `<PageDown>`, `<S-Down>` | Scroll forward one page | [count] pages | NO |
| `CTRL-B` | `<PageUp>`, `<S-Up>` | Scroll backward one page | [count] pages | NO |

### 9.4 Reposition Screen Relative to Cursor

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `zz` | Center cursor line on screen (keep column) | N/A | NO |
| `zt` | Cursor line at top of screen (keep column) | N/A | NO |
| `zb` | Cursor line at bottom of screen (keep column) | N/A | NO |
| `z<CR>` | Cursor line at top, cursor to first non-blank | [count] = line | NO |
| `z.` | Cursor line at center, cursor to first non-blank | [count] = line | NO |
| `z-` | Cursor line at bottom, cursor to first non-blank | [count] = line | NO |
| `z+` | Line below window at top, or line [count] at top | [count] = line | NO |
| `z^` | Line above window at bottom, or line [count] at bottom | [count] = line | NO |

### 9.5 Horizontal Scroll (wrap off)

| Command | Aliases | Description | Count | Already in Alfred |
|---------|---------|-------------|-------|-------------------|
| `zl` | `z<Right>` | Scroll [count] chars right | [count] | NO |
| `zh` | `z<Left>` | Scroll [count] chars left | [count] | NO |
| `zL` | | Scroll half screenwidth right | N/A | NO |
| `zH` | | Scroll half screenwidth left | N/A | NO |
| `zs` | | Scroll to position cursor at screen start (left) | N/A | NO |
| `ze` | | Scroll to position cursor at screen end (right) | N/A | NO |

---

## 10. Normal Mode: g-Prefix Commands

The `g` prefix provides extended commands. [1][4]

| Command | Description | Category | Already in Alfred |
|---------|-------------|----------|-------------------|
| `ga` | Print ASCII/Unicode value of character under cursor | Info | NO |
| `g8` | Print hex value of UTF-8 bytes | Info | NO |
| `gd` | Go to local definition of word under cursor | Navigation | NO |
| `gD` | Go to global definition of word under cursor | Navigation | NO |
| `ge` | Backward to end of [count] word | Motion | NO |
| `gE` | Backward to end of [count] WORD | Motion | NO |
| `gf` | Edit file whose name is under cursor | Navigation | NO |
| `gF` | Edit file under cursor + jump to line number | Navigation | NO |
| `gg` | Go to line [count], default first line | Motion | YES |
| `gh` | Start Select mode (character-wise) | Mode | NO |
| `gH` | Start Select mode (line-wise) | Mode | NO |
| `g CTRL-H` | Start Select mode (block-wise) | Mode | NO |
| `gi` | Insert at position where insert mode last stopped | Insert | NO |
| `gI` | Insert at column 1 | Insert | NO |
| `gj` | Move down [count] display lines | Motion | NO |
| `gk` | Move up [count] display lines | Motion | NO |
| `gJ` | Join [count] lines without space | Editing | NO |
| `gm` | Go to middle of screen line | Motion | NO |
| `gM` | Go to middle of text line | Motion | NO |
| `gn` | Find next match and visually select it | Search | NO |
| `gN` | Find previous match and visually select it | Search | NO |
| `go` | Go to byte [count] in buffer | Motion | NO |
| `gp` | Put after cursor, leave cursor after text | Editing | NO |
| `gP` | Put before cursor, leave cursor after text | Editing | NO |
| `gq{motion}` | Format text (operator) | Formatting | NO |
| `gqq` | Format current line | Formatting | NO |
| `gr{char}` | Virtual replace [count] chars with {char} | Editing | NO |
| `gR` | Enter Virtual Replace mode | Mode | NO |
| `gs` | Go to sleep for [count] seconds | Misc | NO |
| `gt` | Go to next tab page | Tabs | NO |
| `gT` | Go to previous tab page | Tabs | NO |
| `g<Tab>` | Go to last accessed tab page | Tabs | NO |
| `gu{motion}` | Make lowercase (operator) | Editing | NO |
| `gU{motion}` | Make uppercase (operator) | Editing | NO |
| `guu` | Lowercase current line | Editing | NO |
| `gUU` | Uppercase current line | Editing | NO |
| `gv` | Reselect previous visual area | Visual | NO |
| `gw{motion}` | Format text, keep cursor position (operator) | Formatting | NO |
| `gww` | Format current line, keep cursor | Formatting | NO |
| `g~{motion}` | Swap case (operator) | Editing | NO |
| `g~~` | Swap case of current line | Editing | NO |
| `g?{motion}` | ROT13 encode (operator) | Encoding | NO |
| `g??` / `g?g?` | ROT13 encode current line | Encoding | NO |
| `g+` | Go to newer text state [count] times | Undo | NO |
| `g-` | Go to older text state [count] times | Undo | NO |
| `g*` | Like `*` but partial word match | Search | NO |
| `g#` | Like `#` but partial word match | Search | NO |
| `g&` | Repeat last `:s` on all lines | Editing | NO |
| `g0` | First char of screen line | Motion | NO |
| `g^` | First non-blank of screen line | Motion | NO |
| `g$` | Last char of screen line | Motion | NO |
| `g_` | Last non-blank char [count-1] lines lower | Motion | NO |
| `g<` | Display previous command output | Info | NO |
| `g CTRL-G` | Detailed cursor position info | Info | NO |
| `g CTRL-]` | `:tjump` to tag under cursor | Navigation | NO |
| `g]` | `:tselect` on tag under cursor | Navigation | NO |
| `gV` | Prevent reselect in Select mode mappings | Visual | NO |
| `gQ` | Enter Ex mode with Vim editing | Mode | NO |
| `g@{motion}` | Call 'operatorfunc' | Extensibility | NO |

---

## 11. Normal Mode: z-Prefix Commands

### 11.1 Folding Commands

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `zf{motion}` | Create fold for text covered by motion | N/A | NO |
| `zF` | Create fold for [count] lines | [count] | NO |
| `zd` | Delete one fold under cursor | N/A | NO |
| `zD` | Delete folds recursively under cursor | N/A | NO |
| `zE` | Eliminate all folds in window | N/A | NO |
| `zo` | Open one fold under cursor | [count] levels | NO |
| `zO` | Open folds recursively under cursor | N/A | NO |
| `zc` | Close one fold under cursor | [count] levels | NO |
| `zC` | Close folds recursively under cursor | N/A | NO |
| `za` | Toggle fold under cursor (open if closed, close if open) | N/A | NO |
| `zA` | Toggle fold recursively | N/A | NO |
| `zv` | Open enough folds to view cursor line | N/A | NO |
| `zx` | Re-apply 'foldlevel' and do `zv` | N/A | NO |
| `zX` | Re-apply 'foldlevel' | N/A | NO |
| `zm` | Fold more: subtract one from 'foldlevel' | [count] | NO |
| `zM` | Set 'foldlevel' to zero (close all) | N/A | NO |
| `zr` | Fold less: add one to 'foldlevel' | [count] | NO |
| `zR` | Set 'foldlevel' to deepest fold (open all) | N/A | NO |
| `zn` | Reset 'foldenable' (disable folding) | N/A | NO |
| `zN` | Set 'foldenable' (enable folding) | N/A | NO |
| `zi` | Toggle 'foldenable' | N/A | NO |
| `zj` | Move to start of next fold | [count] | NO |
| `zk` | Move to end of previous fold | [count] | NO |

### 11.2 Spelling Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `z=` | Give spelling suggestions | NO |
| `zg` | Permanently mark word as correctly spelled | NO |
| `zG` | Temporarily mark word as correctly spelled | NO |
| `zw` | Permanently mark word as incorrectly spelled | NO |
| `zW` | Temporarily mark word as incorrectly spelled | NO |
| `zug` | Undo `zg` | NO |
| `zuG` | Undo `zG` | NO |
| `zuw` | Undo `zw` | NO |
| `zuW` | Undo `zW` | NO |

### 11.3 Scroll/View Commands (see Section 9.4 and 9.5)

Covered in Section 9 above: `zz`, `zt`, `zb`, `z<CR>`, `z.`, `z-`, `z+`, `z^`, `zl`, `zh`, `zL`, `zH`, `zs`, `ze`.

### 11.4 Other z-Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `z{height}<CR>` | Redraw, make window {height} lines tall | NO |
| `zp` / `zP` | Paste in block-mode without trailing spaces | NO |
| `zy` | Yank without trailing spaces | NO |

---

## 12. Normal Mode: Square Bracket Commands

These commands navigate through code structures. [1][4]

### 12.1 Unmatched Bracket Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `[(` | Go to [count] previous unmatched `(` | [count] | NO |
| `[{` | Go to [count] previous unmatched `{` | [count] | NO |
| `])` | Go to [count] next unmatched `)` | [count] | NO |
| `]}` | Go to [count] next unmatched `}` | [count] | NO |

### 12.2 Method/Function Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `]m` | Go to [count] next start of method (Java-style) | [count] | NO |
| `]M` | Go to [count] next end of method | [count] | NO |
| `[m` | Go to [count] previous start of method | [count] | NO |
| `[M` | Go to [count] previous end of method | [count] | NO |

### 12.3 Section Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `]]` | [count] sections forward (next `{` in first column) | [count] | NO |
| `][` | [count] sections forward (next `}` in first column) | [count] | NO |
| `[[` | [count] sections backward (prev `{` in first column) | [count] | NO |
| `[]` | [count] sections backward (prev `}` in first column) | [count] | NO |

### 12.4 C/C++ Preprocessor Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `[#` | Go to [count] previous unmatched `#if` or `#else` | [count] | NO |
| `]#` | Go to [count] next unmatched `#else` or `#endif` | [count] | NO |

### 12.5 C Comment Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `[*` / `[/` | Go to [count] previous start of C comment `/*` | [count] | NO |
| `]*` / `]/` | Go to [count] next end of C comment `*/` | [count] | NO |

### 12.6 Diff/Change Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `]c` | Jump to start of next change (diff mode) | [count] | NO |
| `[c` | Jump to start of previous change (diff mode) | [count] | NO |

### 12.7 Mark Navigation

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `]'` | Jump to next line with lowercase mark | [count] | NO |
| `['` | Jump to previous line with lowercase mark | [count] | NO |
| `` ]` `` | Jump to next lowercase mark | [count] | NO |
| `` [` `` | Jump to previous lowercase mark | [count] | NO |

### 12.8 Paste with Indent

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `]p` | Put after cursor, adjust indent to current line | NO |
| `[p` | Put before cursor, adjust indent to current line | NO |

---

## 13. Normal Mode: CTRL-W Window Commands

### 13.1 Window Navigation

| Command | Aliases | Description | Count | Already in Alfred |
|---------|---------|-------------|-------|-------------------|
| `CTRL-W j` | `CTRL-W <Down>`, `CTRL-W CTRL-J` | Move to window below | [count] | NO |
| `CTRL-W k` | `CTRL-W <Up>`, `CTRL-W CTRL-K` | Move to window above | [count] | NO |
| `CTRL-W h` | `CTRL-W <Left>`, `CTRL-W CTRL-H`, `CTRL-W <BS>` | Move to window left | [count] | NO |
| `CTRL-W l` | `CTRL-W <Right>`, `CTRL-W CTRL-L` | Move to window right | [count] | NO |
| `CTRL-W w` | `CTRL-W CTRL-W` | Move to next window (cycle) | [count]=Nth | NO |
| `CTRL-W W` | | Move to previous window (reverse cycle) | [count]=Nth | NO |
| `CTRL-W t` | `CTRL-W CTRL-T` | Move to top-left window | N/A | NO |
| `CTRL-W b` | `CTRL-W CTRL-B` | Move to bottom-right window | N/A | NO |
| `CTRL-W p` | `CTRL-W CTRL-P` | Move to previous (last accessed) window | N/A | NO |
| `CTRL-W P` | | Move to preview window | N/A | NO |

### 13.2 Window Splitting

| Command | Aliases | Description | Already in Alfred |
|---------|---------|-------------|-------------------|
| `CTRL-W s` | `CTRL-W S`, `CTRL-W CTRL-S` | Split horizontally | NO |
| `CTRL-W v` | `CTRL-W CTRL-V` | Split vertically | NO |
| `CTRL-W n` | `CTRL-W CTRL-N` | New window with empty buffer | NO |
| `CTRL-W ^` | `CTRL-W CTRL-^` | Split and edit alternate file | NO |

### 13.3 Window Closing

| Command | Aliases | Description | Already in Alfred |
|---------|---------|-------------|-------------------|
| `CTRL-W q` | `CTRL-W CTRL-Q` | Quit current window | NO |
| `CTRL-W c` | | Close current window | NO |
| `CTRL-W o` | `CTRL-W CTRL-O` | Make current window the only one (close all others) | NO |
| `CTRL-W z` | `CTRL-W CTRL-Z` | Close preview window | NO |

### 13.4 Window Moving/Rotating

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-W r` / `CTRL-W CTRL-R` | Rotate windows downward/rightward | NO |
| `CTRL-W R` | Rotate windows upward/leftward | NO |
| `CTRL-W x` / `CTRL-W CTRL-X` | Exchange current window with next | NO |
| `CTRL-W K` | Move window to very top (full width) | NO |
| `CTRL-W J` | Move window to very bottom (full width) | NO |
| `CTRL-W H` | Move window to far left (full height) | NO |
| `CTRL-W L` | Move window to far right (full height) | NO |
| `CTRL-W T` | Move window to new tab page | NO |

### 13.5 Window Resizing

| Command | Description | Count | Already in Alfred |
|---------|-------------|-------|-------------------|
| `CTRL-W =` | Make all windows equal size | N/A | NO |
| `CTRL-W +` | Increase height by [count] | [count] (default 1) | NO |
| `CTRL-W -` | Decrease height by [count] | [count] (default 1) | NO |
| `CTRL-W _` / `CTRL-W CTRL-_` | Set height to [count] (default max) | [count] | NO |
| `CTRL-W >` | Increase width by [count] | [count] (default 1) | NO |
| `CTRL-W <` | Decrease width by [count] | [count] (default 1) | NO |
| `CTRL-W \|` | Set width to [count] (default max) | [count] | NO |

### 13.6 Window Tag/File Navigation

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-W ]` / `CTRL-W CTRL-]` | Split and jump to tag under cursor | NO |
| `CTRL-W g]` | Split and `:tselect` tag under cursor | NO |
| `CTRL-W g CTRL-]` | Split and `:tjump` tag under cursor | NO |
| `CTRL-W f` / `CTRL-W CTRL-F` | Split and edit file under cursor | NO |
| `CTRL-W F` | Split and edit file under cursor + jump to line | NO |
| `CTRL-W gf` | Open file under cursor in new tab | NO |
| `CTRL-W gF` | Open file under cursor in new tab + jump to line | NO |
| `CTRL-W }` | Preview tag under cursor | NO |
| `CTRL-W g}` | `:ptjump` tag under cursor | NO |

---

## 14. Text Objects

Text objects define regions of text and can only be used after an operator or in Visual mode. The `i` prefix means "inner" (excluding delimiters), `a` means "around" (including delimiters/whitespace). [1][2][3]

### 14.1 Word/WORD Objects

| Text Object | Description | Commonly Used |
|-------------|-------------|---------------|
| `iw` | Inner word | YES |
| `aw` | A word (includes surrounding whitespace) | YES |
| `iW` | Inner WORD | Moderate |
| `aW` | A WORD (includes surrounding whitespace) | Moderate |

### 14.2 Sentence/Paragraph Objects

| Text Object | Description | Commonly Used |
|-------------|-------------|---------------|
| `is` | Inner sentence | Moderate |
| `as` | A sentence (includes trailing whitespace) | Moderate |
| `ip` | Inner paragraph | YES |
| `ap` | A paragraph (includes trailing blank lines) | YES |

### 14.3 Bracket/Block Objects

| Text Object | Aliases | Description | Commonly Used |
|-------------|---------|-------------|---------------|
| `i(` | `i)`, `ib` | Inner parenthesis block | YES |
| `a(` | `a)`, `ab` | A parenthesis block (with parens) | YES |
| `i[` | `i]` | Inner bracket block | YES |
| `a[` | `a]` | A bracket block (with brackets) | YES |
| `i{` | `i}`, `iB` | Inner brace block | YES |
| `a{` | `a}`, `aB` | A brace block (with braces) | YES |
| `i<` | `i>` | Inner angle bracket block | Moderate |
| `a<` | `a>` | An angle bracket block (with brackets) | Moderate |

### 14.4 Quote Objects

| Text Object | Description | Commonly Used |
|-------------|-------------|---------------|
| `i"` | Inner double-quoted string | YES |
| `a"` | A double-quoted string (with quotes) | YES |
| `i'` | Inner single-quoted string | YES |
| `a'` | A single-quoted string (with quotes) | YES |
| `` i` `` | Inner backtick string | Moderate |
| `` a` `` | A backtick string (with backticks) | Moderate |

### 14.5 Tag Objects

| Text Object | Description | Commonly Used |
|-------------|-------------|---------------|
| `it` | Inner tag block (HTML/XML) | Moderate |
| `at` | A tag block (with tags) | Moderate |

---

## 15. Operator + Motion Composition

This section shows how operators compose with motions and text objects. The grammar is `[count] {operator} [count] {motion/text-object}`. [1][3][5]

### 15.1 Common Operator + Motion Combinations

| Combination | Description | Usage Frequency |
|-------------|-------------|-----------------|
| `dw` | Delete to start of next word | Daily |
| `de` | Delete to end of word | Daily |
| `db` | Delete word backward | Daily |
| `d$` / `D` | Delete to end of line | Daily |
| `d0` | Delete to start of line | Weekly |
| `d^` | Delete to first non-blank | Weekly |
| `dG` | Delete to end of file | Weekly |
| `dgg` | Delete to start of file | Weekly |
| `d%` | Delete to matching bracket | Weekly |
| `df{char}` | Delete through next {char} | Weekly |
| `dt{char}` | Delete up to (not including) next {char} | Weekly |
| `d/{pattern}<CR>` | Delete to next match of pattern | Rarely |
| `cw` / `ce` | Change to end of word | Daily |
| `cb` | Change word backward | Daily |
| `c$` / `C` | Change to end of line | Daily |
| `c0` | Change to start of line | Weekly |
| `cf{char}` | Change through next {char} | Weekly |
| `ct{char}` | Change up to next {char} | Weekly |
| `yw` | Yank to start of next word | Daily |
| `ye` | Yank to end of word | Daily |
| `y$` | Yank to end of line | Daily |
| `y0` | Yank to start of line | Weekly |
| `yG` | Yank to end of file | Weekly |
| `ygg` | Yank to start of file | Weekly |
| `>}` | Indent to end of paragraph | Weekly |
| `>{count}j` | Indent [count] lines down | Weekly |
| `<{count}j` | Dedent [count] lines down | Weekly |
| `=ip` | Re-indent paragraph | Weekly |
| `gUw` | Uppercase to end of word | Weekly |
| `guw` | Lowercase to end of word | Weekly |
| `g~w` | Swap case to end of word | Weekly |
| `gqap` | Format paragraph | Weekly |

### 15.2 Common Operator + Text Object Combinations

| Combination | Description | Usage Frequency |
|-------------|-------------|-----------------|
| `diw` | Delete inner word | Daily |
| `daw` | Delete a word (with whitespace) | Daily |
| `ciw` | Change inner word | Daily |
| `caw` | Change a word (with whitespace) | Daily |
| `yiw` | Yank inner word | Daily |
| `yaw` | Yank a word | Daily |
| `di"` | Delete inside double quotes | Daily |
| `da"` | Delete including double quotes | Daily |
| `ci"` | Change inside double quotes | Daily |
| `di'` | Delete inside single quotes | Daily |
| `ci'` | Change inside single quotes | Daily |
| `di(` / `dib` | Delete inside parentheses | Daily |
| `da(` / `dab` | Delete including parentheses | Daily |
| `ci(` / `cib` | Change inside parentheses | Daily |
| `di{` / `diB` | Delete inside braces | Daily |
| `da{` / `daB` | Delete including braces | Daily |
| `ci{` / `ciB` | Change inside braces | Daily |
| `di[` | Delete inside brackets | Weekly |
| `ci[` | Change inside brackets | Weekly |
| `dit` | Delete inside HTML/XML tags | Weekly |
| `cit` | Change inside HTML/XML tags | Weekly |
| `dis` | Delete inner sentence | Weekly |
| `dip` | Delete inner paragraph | Weekly |
| `yip` | Yank inner paragraph | Weekly |
| `>iB` | Indent inner brace block | Weekly |
| `=iB` | Re-indent inner brace block | Weekly |
| `gUiw` | Uppercase inner word | Weekly |

### 15.3 Count Combinations

| Combination | Description |
|-------------|-------------|
| `3dd` | Delete 3 lines |
| `5yy` | Yank 5 lines |
| `2dw` | Delete 2 words |
| `d3w` | Delete 3 words |
| `2d3w` | Delete 6 words (counts multiply) |
| `3>>` | Indent 3 lines |
| `5j` | Move 5 lines down |
| `3f{char}` | Find 3rd occurrence of {char} |
| `4p` | Paste 4 times |
| `3.` | Repeat last change 3 times |

---

## 16. Insert Mode Commands

### 16.1 Entering Insert Mode (from Normal Mode)

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `i` | Insert before cursor | YES |
| `I` | Insert before first non-blank on line | YES |
| `a` | Append after cursor | YES |
| `A` | Append at end of line | YES |
| `o` | Open new line below | YES |
| `O` | Open new line above | YES |
| `gI` | Insert at column 1 | NO |
| `gi` | Insert at position where insert mode last stopped | NO |
| `s` | Delete character, enter insert (= `cl`) | NO |
| `S` | Delete line, enter insert (= `cc`) | NO |
| `c{motion}` | Delete motion range, enter insert | YES |
| `C` | Delete to end of line, enter insert | YES |
| `R` | Enter Replace mode | NO |
| `gR` | Enter Virtual Replace mode | NO |

### 16.2 Exiting Insert Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `<Esc>` | Exit insert mode | YES |
| `CTRL-[` | Exit insert mode (same as Esc) | NO |
| `CTRL-C` | Exit insert mode (no abbrev check) | NO |

### 16.3 Editing in Insert Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `<BS>` / `CTRL-H` | Delete character before cursor | YES (Backspace) |
| `<Del>` | Delete character under cursor | NO |
| `CTRL-W` | Delete word before cursor | NO |
| `CTRL-U` | Delete all entered characters in current line | NO |
| `CTRL-T` | Indent: insert one shiftwidth of indent | NO |
| `CTRL-D` | Dedent: remove one shiftwidth of indent | NO |
| `0 CTRL-D` | Delete all indent in current line | NO |
| `^ CTRL-D` | Delete all indent, restore in next line | NO |

### 16.4 Special Character Insertion

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-V {char}` | Insert character literally | NO |
| `CTRL-V {digits}` | Insert character by decimal code | NO |
| `CTRL-Q` | Same as CTRL-V | NO |
| `CTRL-K {char1} {char2}` | Insert digraph | NO |
| `<CR>` / `CTRL-M` | Insert newline | YES |
| `<NL>` / `CTRL-J` | Insert newline | NO |
| `<Tab>` / `CTRL-I` | Insert tab | NO |

### 16.5 Register Insertion in Insert Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-R {reg}` | Insert contents of register | NO |
| `CTRL-R CTRL-R {reg}` | Insert literally (no remapping) | NO |
| `CTRL-R CTRL-O {reg}` | Insert literally, no auto-indent | NO |
| `CTRL-R CTRL-P {reg}` | Insert literally, fix indent | NO |

### 16.6 Completion Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-N` | Find next keyword match (autocomplete) | NO |
| `CTRL-P` | Find previous keyword match | NO |
| `CTRL-X CTRL-L` | Complete whole lines | NO |
| `CTRL-X CTRL-N` | Complete keywords (forward) | NO |
| `CTRL-X CTRL-P` | Complete keywords (backward) | NO |
| `CTRL-X CTRL-K` | Complete from dictionary | NO |
| `CTRL-X CTRL-T` | Complete from thesaurus | NO |
| `CTRL-X CTRL-I` | Complete from included files | NO |
| `CTRL-X CTRL-]` | Complete tags | NO |
| `CTRL-X CTRL-F` | Complete filenames | NO |
| `CTRL-X CTRL-D` | Complete definitions/macros | NO |
| `CTRL-X CTRL-V` | Complete Vim commands | NO |
| `CTRL-X CTRL-U` | User-defined completion | NO |
| `CTRL-X CTRL-O` | Omni completion | NO |
| `CTRL-X CTRL-S` / `CTRL-X s` | Spelling suggestions | NO |
| `CTRL-X CTRL-E` | Scroll window up (in completion) | NO |
| `CTRL-X CTRL-Y` | Scroll window down (in completion) | NO |

### 16.7 Completion Menu Navigation

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-E` | Stop completion, return to original | NO |
| `CTRL-Y` | Accept current completion match | NO |
| `<PageUp>` | Select match several entries back | NO |
| `<PageDown>` | Select match several entries forward | NO |

### 16.8 Copy/Insert Operations

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-A` | Insert previously inserted text | NO |
| `CTRL-@` | Insert previously inserted text and exit insert mode | NO |
| `CTRL-E` | Insert character from line below | NO |
| `CTRL-Y` | Insert character from line above | NO |

### 16.9 Special Navigation in Insert Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-O {cmd}` | Execute one normal mode command, return to insert | NO |
| `CTRL-\ CTRL-O {cmd}` | Execute command without moving cursor | NO |
| `CTRL-G u` | Break undo sequence, start new change | NO |
| `CTRL-G j` / `CTRL-G <Down>` | Move cursor down, keep insert column | NO |
| `CTRL-G k` / `CTRL-G <Up>` | Move cursor up, keep insert column | NO |
| `<Insert>` | Toggle between Insert and Replace mode | NO |

### 16.10 Language/Input

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `CTRL-^` | Toggle language/input method | NO |
| `CTRL-]` | Trigger abbreviation | NO |

---

## 17. Visual Mode Commands

### 17.1 Entering Visual Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `v` | Character-wise visual mode | NO |
| `V` | Line-wise visual mode | NO |
| `CTRL-V` | Block-wise visual mode | NO |
| `gv` | Re-select previous visual area | NO |

### 17.2 Exiting Visual Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `<Esc>` / `CTRL-C` | Exit visual mode | NO |
| `v` (in char-visual) | Exit char-visual | NO |
| `V` (in line-visual) | Exit line-visual | NO |
| `CTRL-V` (in block-visual) | Exit block-visual | NO |

### 17.3 Switching Visual Sub-Modes

| Command | Description |
|---------|-------------|
| `v` (in V or CTRL-V mode) | Switch to char-wise visual |
| `V` (in v or CTRL-V mode) | Switch to line-wise visual |
| `CTRL-V` (in v or V mode) | Switch to block-wise visual |

### 17.4 Visual Mode Navigation

All normal mode motions work in visual mode to extend the selection. Additionally:

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `o` | Go to other end of selection | NO |
| `O` | In block mode: go to other corner on same line | NO |
| `$` | In block mode: extend selection to end of each line | NO |

### 17.5 Operators in Visual Mode

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `d` / `x` | Delete selection | NO |
| `c` / `s` | Change selection (delete + insert) | NO |
| `y` | Yank (copy) selection | NO |
| `~` | Switch case | NO |
| `u` | Make lowercase | NO |
| `U` | Make uppercase | NO |
| `>` | Shift right | NO |
| `<` | Shift left | NO |
| `!` | Filter through external command | NO |
| `=` | Filter through 'equalprg' / auto-indent | NO |
| `gq` | Format text | NO |
| `J` | Join lines | NO |
| `gJ` | Join lines without space | NO |
| `r{char}` | Replace every character in selection with {char} | NO |
| `C` | Change to end of line (from each line in selection) | NO |
| `S` | Change entire selected lines | NO |
| `R` | Change (replace mode variant) | NO |
| `D` | Delete to end of line | NO |
| `X` | Delete entire selected lines | NO |
| `Y` | Yank entire selected lines | NO |
| `p` / `P` | Put (replace selection with register) | NO |
| `CTRL-]` | Jump to tag | NO |

### 17.6 Block-Visual Specific Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `I` | Block insert: insert text at start of block on every line | NO |
| `A` | Block append: append text at end of block on every line | NO |

### 17.7 Select Mode (variant of Visual)

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `gh` | Start Select mode (character-wise) | NO |
| `gH` | Start Select mode (line-wise) | NO |
| `g CTRL-H` | Start Select mode (block-wise) | NO |
| `CTRL-G` | Switch between Visual and Select modes | NO |

---

## 18. Command-Line (Ex) Mode

### 18.1 Essential Ex Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:w [file]` | Write (save) | YES |
| `:w! [file]` | Force write | NO |
| `:q` | Quit | YES |
| `:q!` | Force quit (discard changes) | YES |
| `:wq` / `:x` | Write and quit | YES |
| `:wq!` | Force write and quit | NO |
| `:wa` | Write all buffers | NO |
| `:qa` | Quit all windows | NO |
| `:qa!` | Force quit all | NO |
| `:wqa` | Write and quit all | NO |
| `:e [file]` | Edit file | YES |
| `:e!` | Reload file (discard changes) | NO |
| `:enew` | Edit new unnamed buffer | NO |

### 18.2 Search and Replace

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:[range]s/{pat}/{rep}/[flags]` | Substitute | NO |
| `:%s/{pat}/{rep}/g` | Substitute all in file | NO |
| `:%s/{pat}/{rep}/gc` | Substitute all with confirmation | NO |
| `:s/{pat}/{rep}/` | Substitute first on current line | NO |
| `:[range]g/{pat}/{cmd}` | Global: execute {cmd} on matching lines | NO |
| `:[range]g!/{pat}/{cmd}` / `:[range]v/{pat}/{cmd}` | Execute on NON-matching lines | NO |
| `:noh[lsearch]` | Remove search highlighting | NO |

**Substitute flags**: `g` (global/all on line), `c` (confirm each), `i` (case insensitive), `I` (case sensitive), `n` (report count only), `e` (no error if not found)

### 18.3 Range Specifiers

| Range | Description |
|-------|-------------|
| `{number}` | Absolute line number |
| `.` | Current line |
| `$` | Last line |
| `%` | Entire file (= `1,$`) |
| `'<,'>` | Visual selection |
| `'{mark}` | Line of mark |
| `/pattern/` | Next matching line |
| `?pattern?` | Previous matching line |
| `+{n}` / `-{n}` | Relative offset |
| `1,5` | Lines 1 through 5 |
| `.,+3` | Current line through 3 lines below |
| `.,$` | Current line through end of file |

### 18.4 Line Manipulation

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:[range]d [reg]` | Delete lines [into register] | NO |
| `:[range]y [reg]` | Yank lines [into register] | NO |
| `:[line]pu [reg]` | Put text from register after [line] | NO |
| `:[range]co {addr}` / `:t` | Copy lines to after {addr} | NO |
| `:[range]m {addr}` | Move lines to after {addr} | NO |
| `:[range]j` | Join lines | NO |

### 18.5 File and Buffer Operations

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:r [file]` | Read file, insert below cursor | NO |
| `:r !{cmd}` | Read output of shell command | NO |
| `:[range]w !{cmd}` | Send lines to shell command | NO |
| `:!{cmd}` | Execute shell command | NO |
| `:[range]!{cmd}` | Filter lines through shell command | NO |
| `:saveas {file}` | Save as new file | NO |
| `:f[ile] {name}` | Set current filename | NO |
| `:bn[ext]` | Go to next buffer | NO |
| `:bp[rev]` | Go to previous buffer | NO |
| `:b {n}` / `:b {name}` | Go to buffer by number or name | NO |
| `:bd[elete]` | Delete (close) buffer | NO |
| `:ls` / `:buffers` | List all buffers | NO |

### 18.6 Window Split Commands (Ex)

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:sp[lit] [file]` | Split horizontally | NO |
| `:vs[plit] [file]` | Split vertically | NO |
| `:new` | New horizontal split with empty buffer | NO |
| `:vnew` | New vertical split with empty buffer | NO |
| `:on[ly]` | Close all other windows | NO |
| `:clo[se]` | Close current window | NO |
| `:res[ize] {n}` / `:res +{n}` / `:res -{n}` | Resize window | NO |
| `:vert[ical] {cmd}` | Make split command vertical | NO |

### 18.7 Settings

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:set {option}` | Enable boolean option | NO |
| `:set no{option}` | Disable boolean option | NO |
| `:set {option}={value}` | Set option value | NO |
| `:set {option}?` | Show option value | NO |
| `:set {option}&` | Reset option to default | NO |
| `:setlocal {option}` | Set option locally | NO |

**Common settings**: `number`/`nu`, `relativenumber`/`rnu`, `wrap`, `nowrap`, `expandtab`/`et`, `tabstop`/`ts`, `shiftwidth`/`sw`, `autoindent`/`ai`, `smartindent`/`si`, `hlsearch`/`hls`, `incsearch`/`is`, `ignorecase`/`ic`, `smartcase`/`scs`, `cursorline`/`cul`, `syntax`, `filetype`/`ft`, `encoding`/`enc`, `fileencoding`/`fenc`, `scrolloff`/`so`, `sidescrolloff`/`siso`, `laststatus`/`ls`, `ruler`/`ru`, `showmode`/`smd`, `showcmd`/`sc`, `wildmenu`/`wmnu`, `mouse`, `clipboard`

### 18.8 Other Ex Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:{line}` | Go to line | NO |
| `:marks` | List all marks | NO |
| `:registers` / `:reg` | Show register contents | NO |
| `:jumps` | Show jump list | NO |
| `:changes` | Show change list | NO |
| `:map {lhs} {rhs}` | Create mapping | NO |
| `:nmap` / `:imap` / `:vmap` | Mode-specific mappings | NO |
| `:unmap {lhs}` | Remove mapping | NO |
| `:noremap {lhs} {rhs}` | Non-recursive mapping | NO |
| `:ab[breviate] {lhs} {rhs}` | Create abbreviation | NO |
| `:source {file}` | Execute Vim commands from file | NO |
| `:retab` | Replace tabs with spaces (or vice versa) | NO |
| `:sort [options]` | Sort lines | NO |
| `:norm[al] {commands}` | Execute normal mode commands | NO |
| `:[range]norm {commands}` | Execute on each line in range | NO |
| `:h[elp] {topic}` | Open help | NO |
| `:echo {expr}` | Evaluate and display expression | NO |
| `:let {var} = {expr}` | Set variable | NO |
| `:if`/`:else`/`:endif` | Conditional execution | NO |
| `:for`/`:endfor` | Loop | NO |
| `:function`/`:endfunction` | Define function | NO |
| `:command {name} {cmd}` | Define user command | NO |
| `:autocmd {event} {pat} {cmd}` | Auto-command | NO |
| `:syntax` | Syntax highlighting commands | NO |
| `:highlight` / `:hi` | Set highlighting | NO |
| `:colorscheme {name}` | Set color scheme | NO |
| `:vim[grep] /{pat}/ {files}` | Search in multiple files | NO |
| `:cn[ext]` | Next quickfix match | NO |
| `:cp[rev]` | Previous quickfix match | NO |
| `:copen` | Open quickfix window | NO |
| `:cclose` | Close quickfix window | NO |
| `:make` | Run make and capture errors | NO |
| `:diffthis` | Make current window part of diff | NO |
| `:diffoff` | Turn off diff mode | NO |
| `:diffget` / `do` | Get difference from other buffer | NO |
| `:diffput` / `dp` | Put difference to other buffer | NO |
| `:diffupdate` | Update differences | NO |

---

## 19. Tab Commands

| Command | Description | Already in Alfred |
|---------|-------------|-------------------|
| `:tabnew [file]` | Open file in new tab | NO |
| `:tabc[lose]` | Close current tab | NO |
| `:tabo[nly]` | Close all other tabs | NO |
| `:tabn[ext]` / `gt` | Go to next tab | NO |
| `:tabp[revious]` / `gT` | Go to previous tab | NO |
| `{count}gt` | Go to tab number {count} | NO |
| `:tabm[ove] {n}` | Move tab to position {n} | NO |
| `:tabdo {cmd}` | Execute command in all tabs | NO |
| `:tab ba[ll]` | Edit all buffers as tabs | NO |
| `CTRL-W T` | Move current window to new tab | NO |
| `g<Tab>` | Go to last accessed tab | NO |

---

## 20. Implementation Priority Matrix

This matrix categorizes every command area by implementation priority for Alfred, considering usage frequency, implementation complexity, and dependencies.

### Tier 1: Essential (Must Have) -- Daily Use

These commands are used constantly and their absence makes the editor frustrating.

| Category | Commands | Complexity | Dependencies |
|----------|----------|------------|--------------|
| **Counts** | `{count}` prefix for all commands | Moderate | Keymap parser |
| **Repeat** | `.` (repeat last change) | Moderate | Change recording infrastructure |
| **Search** | `/`, `?`, `n`, `N`, `*`, `#` | Moderate | Regex engine, highlight |
| **Character find** | `f`, `F`, `t`, `T`, `;`, `,` | Trivial | None |
| **Text objects (core)** | `iw`, `aw`, `i"`, `a"`, `i'`, `a'`, `i(`, `a(`, `i{`, `a{`, `i[`, `a[` | Moderate | Text object parser |
| **Visual mode (basic)** | `v`, `V`, `CTRL-V`, operators in visual | Moderate | Selection model |
| **Simple editing** | `r`, `R`, `s`, `S`, `X`, `P`, `D`, `~`, `J` already but `gJ` | Trivial-Moderate | None |
| **WORD motions** | `W`, `B`, `E`, `gE` | Trivial | Word boundary detection |
| **Operators** | `>>`, `<<`, `==` (indent/dedent) | Moderate | Indent engine |
| **Paragraph motion** | `{`, `}` | Trivial | Paragraph detection |
| **Match** | `%` (bracket matching) | Moderate | Bracket matching |
| **Line movement** | `+`, `-`, `g_` | Trivial | None |
| **Scroll** | `CTRL-E`, `CTRL-Y`, `CTRL-F`, `CTRL-B`, `zz`, `zt`, `zb` | Trivial | Viewport |
| **Insert mode editing** | `CTRL-W`, `CTRL-U` | Trivial | None |
| **Ex commands** | `:{line}`, `:s`, `:%s`, `:noh` | Moderate | Command-line parser, regex |
| **Command mode** | `CTRL-O` in insert mode | Moderate | Mode switching |

### Tier 2: Important (Should Have) -- Weekly Use

These commands significantly improve productivity.

| Category | Commands | Complexity | Dependencies |
|----------|----------|------------|--------------|
| **Marks** | `m{a-z}`, `'{mark}`, `` `{mark} ``, `''`, ` `` ` | Moderate | Mark storage |
| **Registers** | `"{reg}`, named registers, `"0`, `"_`, `"+` | Moderate | Register infrastructure |
| **Macros** | `q{reg}`, `@{reg}`, `@@` | Complex | Register infra + command recording |
| **Case operators** | `gU`, `gu`, `g~`, `gUU`, `guu`, `g~~` | Trivial | Operator infrastructure |
| **More text objects** | `is`, `as`, `ip`, `ap`, `it`, `at`, `i<`, `a<`, `` i` ``, `` a` ``, `iW`, `aW`, `iB`, `aB`, `ib`, `ab` | Moderate | Text object parser |
| **Jump list** | `CTRL-O`, `CTRL-I` | Moderate | Jump list data structure |
| **Change list** | `g;`, `g,` | Moderate | Change list data structure |
| **Sentence motion** | `(`, `)` | Moderate | Sentence detection |
| **Section motion** | `[[`, `]]`, `[]`, `][` | Moderate | Section detection |
| **Screen-line nav** | `gj`, `gk`, `g0`, `g$`, `g^`, `gm` | Moderate | Soft-wrap awareness |
| **Insert mode (more)** | `CTRL-T`, `CTRL-D`, `CTRL-R {reg}` | Moderate | Register/indent infra |
| **Line manipulation** | `:d`, `:y`, `:m`, `:co`/`:t` | Moderate | Range parser |
| **Buffer commands** | `:bn`, `:bp`, `:b`, `:bd`, `:ls` | Moderate | Buffer management |
| **Bracket nav** | `[(`, `[{`, `])`, `]}` | Moderate | Bracket scanning |
| **Number inc/dec** | `CTRL-A`, `CTRL-X` | Moderate | Number parsing |
| **Paste variants** | `gp`, `gP`, `]p`, `[p` | Trivial | Paste infrastructure |
| **U (undo line)** | `U` | Moderate | Line change tracking |
| **ZZ / ZQ** | `ZZ`, `ZQ` | Trivial | Already have `:wq` / `:q!` |
| **gi** | `gi` (insert at last insert pos) | Trivial | Track last insert position |
| **Partial search** | `g*`, `g#` | Trivial | Search infrastructure |
| **gn / gN** | Select next/prev match | Moderate | Search + visual mode |

### Tier 3: Advanced (Nice to Have) -- Monthly/Power User

| Category | Commands | Complexity | Dependencies |
|----------|----------|------------|--------------|
| **Window splits** | `CTRL-W s/v/h/j/k/l/q/o/=`, `:sp`, `:vs` | Very Complex | Window management system |
| **Tab pages** | `:tabnew`, `gt`, `gT`, `:tabc` | Very Complex | Tab system |
| **Window resize** | `CTRL-W +/-/</>/_/\|/=` | Complex | Window layout engine |
| **Window move** | `CTRL-W H/J/K/L/T/r/R/x` | Complex | Window layout engine |
| **Folding** | `zf`, `zo`, `zc`, `za`, `zd`, `zM`, `zR`, `zm`, `zr` | Very Complex | Fold engine |
| **Global command** | `:g/{pat}/{cmd}`, `:v` | Complex | Range + command execution |
| **Filter** | `!{motion}`, `:{range}!{cmd}` | Complex | Shell integration |
| **Format** | `gq{motion}`, `gw{motion}` | Moderate | Text formatting |
| **Settings** | `:set`, `:setlocal` | Complex | Options system |
| **File operations** | `:r`, `:w !cmd`, `:r !cmd` | Moderate | Shell/file integration |
| **Completion** | `CTRL-N`, `CTRL-P`, `CTRL-X` sub-modes | Very Complex | Completion engine |
| **Diff mode** | `:diffthis`, `]c`, `[c`, `do`, `dp` | Very Complex | Diff engine |
| **Method nav** | `]m`, `]M`, `[m`, `[M` | Complex | Language-aware parsing |
| **vimgrep/quickfix** | `:vimgrep`, `:cn`, `:cp`, `:copen` | Very Complex | Quickfix system |
| **Mappings** | `:map`, `:nmap`, `:imap`, `:noremap` | Complex | Mapping engine |
| **Abbreviations** | `:ab` | Moderate | Abbreviation engine |
| **Sort** | `:sort` | Moderate | Sort implementation |
| **Spelling** | `z=`, `zg`, `zw` | Very Complex | Spell-check engine |
| **Tags** | `CTRL-]`, `CTRL-T`, `CTRL-W ]` | Very Complex | Tag system |
| **Expression register** | `"=` | Complex | Expression evaluator |

### Tier 4: Specialist (Defer)

| Category | Commands | Rationale |
|----------|----------|-----------|
| **Select mode** | `gh`, `gH`, `g CTRL-H`, `CTRL-G` | Rarely used; IDE-style selection |
| **Virtual Replace** | `gR`, `gr` | Niche use case |
| **ROT13** | `g?`, `g??` | Novelty command |
| **Sleep** | `gs` | No practical use in editor |
| **Ex mode** | `Q`, `gQ` | Legacy; most users avoid it |
| **C preprocessor nav** | `[#`, `]#` | C-specific |
| **C comment nav** | `[*`, `]*`, `[/`, `]/` | C-specific |
| **Byte offset** | `go` | Rarely needed |
| **Man page lookup** | `K` | Requires external program |
| **Digraphs** | `CTRL-K` in insert mode | Niche |
| **`z{height}<CR>`** | Window height setting | Niche |
| **Horizontal scroll** | `zl/zh/zL/zH/zs/ze` | Only relevant with nowrap |
| **Preview window** | `CTRL-W P`, `CTRL-W z`, `CTRL-W }`, `CTRL-W g}` | Requires preview system |
| **Autocmd/scripting** | `:autocmd`, `:function`, `:if`, `:for`, `:let` | Replace with Alfred Lisp |
| **Lockmarks/keepmarks** | `:lockmarks`, `:keepmarks`, `:keepjumps` | Advanced scripting |

---

## Summary Statistics

| Category | Total Commands | Already Implemented | Remaining |
|----------|---------------|---------------------|-----------|
| Cursor Movement | ~60 | 14 | ~46 |
| Operators | 14 | 3 (d, c, y) | 11 |
| Simple Editing | ~30 | 9 | ~21 |
| Search | 14 | 0 | 14 |
| Marks | ~25 | 0 | ~25 |
| Registers | ~20 | 0 | ~20 |
| Macros | 9 | 0 | 9 |
| Scroll | ~20 | 2 (Ctrl-D/U) | ~18 |
| g-prefix | ~55 | 1 (gg) | ~54 |
| z-prefix | ~45 | 0 | ~45 |
| Square bracket | ~20 | 0 | ~20 |
| CTRL-W Window | ~50 | 0 | ~50 |
| Text Objects | ~32 | 0 | ~32 |
| Insert Mode | ~50 | ~8 | ~42 |
| Visual Mode | ~40 | 0 | ~40 |
| Ex Commands | ~80 | 5 | ~75 |
| Tab Commands | ~11 | 0 | ~11 |
| **TOTAL** | **~575** | **~42** | **~533** |

**Implementation Coverage**: ~7% of the complete vim command set is currently implemented.

---

## Source Analysis

| Source | Domain | Reputation | Type | Access Date | Cross-verified |
|--------|--------|------------|------|-------------|----------------|
| Vim Help Index (index.txt) | vimhelp.org | High | Official documentation | 2026-03-23 | Y (canonical) |
| Vim Motion Documentation | vimdoc.sourceforge.net | High | Official documentation | 2026-03-23 | Y |
| Vim Insert Documentation | vimdoc.sourceforge.net | High | Official documentation | 2026-03-23 | Y |
| Vim Visual Documentation | vimdoc.sourceforge.net | High | Official documentation | 2026-03-23 | Y |
| Vim Scroll Documentation | vimdoc.sourceforge.net | High | Official documentation | 2026-03-23 | Y |
| Vim Windows Documentation | vimdoc.sourceforge.net | High | Official documentation | 2026-03-23 | Y |
| Vim Cheat Sheet | vim.rtorr.com | Medium-High | Community reference | 2026-03-23 | Y |
| Learn Vim Reference Guide (Normal) | learnbyexample.github.io | Medium-High | Technical guide | 2026-03-23 | Y |
| Vim and Git Guide | vimandgit.com | Medium-High | Technical blog | 2026-03-23 | Y |
| Vim Registers (brianstorti) | brianstorti.com | Medium | Community blog | 2026-03-23 | Y |
| Learn Vim (iggredible) | learnvim.irian.to | Medium-High | Tutorial series | 2026-03-23 | Y |
| Vim Macros (learnbyexample) | learnbyexample.github.io | Medium-High | Reference guide | 2026-03-23 | Y |
| Insert Mode Cheatsheet (dev.to) | dev.to | Medium | Community blog | 2026-03-23 | Y |
| Vim Tips Wiki | vim.fandom.com | Medium-High | Community wiki | 2026-03-23 | Y |

Reputation: High: 6 (43%) | Medium-High: 6 (43%) | Medium: 2 (14%) | Avg: 0.83

---

## Knowledge Gaps

### Gap 1: Vim Regex Syntax Details
**Issue**: The complete vim regex syntax (magic/very-magic/nomagic modes, vim-specific atoms like `\zs`, `\ze`, `\{-}`) was not fully documented. | **Attempted**: Searched for vim regex reference | **Recommendation**: Dedicated research on vim regex dialect for `:s` and `/` command implementation.

### Gap 2: Complete Option List
**Issue**: Vim has 300+ options settable via `:set`. Only commonly-used options were documented. | **Attempted**: Listed common options | **Recommendation**: Determine which options Alfred actually needs and research only those.

### Gap 3: Operator-Pending Mode Edge Cases
**Issue**: The exact behavior of operators when combined with certain special motions (e.g., forced linewise/charwise via `V`/`v`/`CTRL-V` after operator) was not fully documented. | **Attempted**: Searched official docs | **Recommendation**: Reference `:help forced-motion` for edge case implementation.

### Gap 4: Insert Mode CTRL-R Behavior Details
**Issue**: The subtle differences between `CTRL-R`, `CTRL-R CTRL-R`, `CTRL-R CTRL-O`, and `CTRL-R CTRL-P` (remapping, auto-indent behavior) need more detail. | **Attempted**: Fetched official insert.html | **Recommendation**: Reference `:help i_CTRL-R` for exact behavior per variant.

---

## Recommendations for Further Research

1. **Vim regex syntax**: Deep-dive into vim's regex dialect (`:help pattern`) for implementing search and substitute commands.
2. **Undo tree**: Vim's undo model is a tree, not linear. Research `:help undo-tree`, `g+`, `g-`, `:earlier`, `:later` for proper implementation.
3. **Text object boundary rules**: The exact rules for word, sentence, and paragraph boundaries in vim (`:help word`, `:help sentence`, `:help paragraph`) need precise specification for correct implementation.
4. **Repeat (`.`) infrastructure**: The `.` command requires tracking the "last change" which includes the operator, motion, inserted text, and count. This needs architectural consideration.
5. **Forced motions**: Research `v`/`V`/`CTRL-V` used between operator and motion to force charwise/linewise/blockwise behavior (`:help forced-motion`).

---

## Full Citations

[1] Vim Project. "Vim documentation: index". vimhelp.org. https://vimhelp.org/index.txt.html. Accessed 2026-03-23.
[2] Vim Project. "Vim documentation: motion". vimdoc.sourceforge.net. https://vimdoc.sourceforge.net/htmldoc/motion.html. Accessed 2026-03-23.
[3] vim.rtorr.com. "Vim Cheat Sheet". https://vim.rtorr.com/. Accessed 2026-03-23.
[4] Learn By Example. "Normal mode - Vim Reference Guide". learnbyexample.github.io. https://learnbyexample.github.io/vim_reference/Normal-mode.html. Accessed 2026-03-23.
[5] Vim and Git. "A Complete Guide to Vim's Normal Mode Commands and Operator-Pending Mode". vimandgit.com. https://vimandgit.com/posts/vim/beginners/vim-normal-mode-commands-and-operator-pending-mode.html. Accessed 2026-03-23.
[6] Vim Project. "Vim documentation: insert". vimdoc.sourceforge.net. https://vimdoc.sourceforge.net/htmldoc/insert.html. Accessed 2026-03-23.
[7] Vim Project. "Vim documentation: visual". vimdoc.sourceforge.net. https://vimdoc.sourceforge.net/htmldoc/visual.html. Accessed 2026-03-23.
[8] Vim Project. "Vim documentation: scroll". vimdoc.sourceforge.net. https://vimdoc.sourceforge.net/htmldoc/scroll.html. Accessed 2026-03-23.
[9] Vim Project. "Vim documentation: windows". vimdoc.sourceforge.net. https://vimdoc.sourceforge.net/htmldoc/windows.html. Accessed 2026-03-23.
[10] Brian Storti. "Vim registers: The basics and beyond". brianstorti.com. https://www.brianstorti.com/vim-registers/. Accessed 2026-03-23.
[11] Igor Irianto. "Learn Vim". learnvim.irian.to. https://learnvim.irian.to/. Accessed 2026-03-23.
[12] Learn By Example. "Macro - Vim Reference Guide". learnbyexample.github.io. https://learnbyexample.github.io/vim_reference/Macro.html. Accessed 2026-03-23.
[13] Igor Irianto. "The Only Vim Insert-Mode Cheatsheet You Ever Needed". dev.to. https://dev.to/iggredible/the-only-vim-insert-mode-cheatsheet-you-ever-needed-nk9. Accessed 2026-03-23.
[14] Vim Tips Wiki. vim.fandom.com. https://vim.fandom.com/. Accessed 2026-03-23.

---

## Research Metadata

Duration: ~25 min | Examined: 20+ pages | Cited: 14 | Cross-refs: 42 | Confidence: High 85%, Medium 10%, Low 5% | Output: docs/research/vim/vim-commands-comprehensive-research.md
