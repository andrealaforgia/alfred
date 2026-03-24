# Syntax Highlighting -- Walking Skeleton Roadmap

**Feature**: syntax-highlighting
**Date**: 2026-03-24
**Estimated production files**: 6-8 (new crate modules + minor changes to existing crates)
**Steps**: 6
**Step ratio**: 6/7 = 0.86 (well within 2.5 limit)
**Paradigm**: Functional core / imperative shell

---

## Step 01: Crate scaffold with tree-sitter dependency

**Description**: Create `alfred-syntax` crate with tree-sitter and one grammar (Rust). Verify compilation and basic parse of a hardcoded string.

**Acceptance criteria**:
- `alfred-syntax` crate exists in workspace with tree-sitter dependency
- Rust grammar compiles and links successfully
- A Rust source string parses into a tree-sitter tree without error

**Architectural constraints**:
- `alfred-syntax` depends on `alfred-core` and `tree-sitter`
- `alfred-core` gains NO new dependencies
- Crate follows functional-core style: data in, data out

---

## Step 02: Parse buffer and produce highlight ranges

**Description**: Given buffer text and a language, parse with tree-sitter, run highlight query, return `(line, start_col, end_col, capture_name)` ranges for a specified line range.

**Acceptance criteria**:
- Rust source buffer parsed produces correct highlight captures
- `fn` keyword in Rust source identified as `@keyword` capture
- String literals identified as `@string` capture
- Querying a line range returns only captures within that range
- Unrecognized file extension returns empty highlights (no error)

**Architectural constraints**:
- Buffer text accessed through `alfred-core` Buffer API (no direct rope access from syntax crate)
- Highlight queries loaded from bundled SCM files
- Output is pure data: Vec of highlight range descriptors

---

## Step 03: Integrate with renderer via line_styles

**Description**: Wire `alfred-syntax` into `alfred-tui` event loop. On file open, parse buffer. Before render, query highlights for visible lines and write to `EditorState.line_styles`.

**Acceptance criteria**:
- Opening a `.rs` file displays syntax-colored text
- Keywords, strings, comments, types render with distinct colors
- Opening a non-code file (`.txt`) renders without highlighting
- Highlight colors use hardcoded defaults initially (theme integration in step 05)

**Architectural constraints**:
- `alfred-tui` depends on `alfred-syntax`
- `alfred-bin` creates highlighter and passes to TUI
- Highlighter writes to `line_styles` through existing `add_line_style` / `clear_line_styles`
- Existing `build_styled_line` renders highlights without modification

---

## Step 04: Incremental re-parsing on buffer edits

**Description**: On buffer edit (insert/delete), send tree-sitter edit notification and re-parse incrementally. Only the changed region is re-parsed.

**Acceptance criteria**:
- Inserting text updates highlighting within the same render frame
- Deleting text updates highlighting correctly
- Adding a block comment (`/* ... */`) recolors enclosed lines
- Editing a 10K-line file re-parses in <5ms
- Highlight query for visible lines completes in <1ms

**Architectural constraints**:
- `alfred-core` Buffer exposes byte-offset text access for tree-sitter parse callback
- Edit notifications computed from buffer operation parameters (line, column, text length)
- Tree is updated in-place via `tree.edit()` before re-parse

---

## Step 05: Lisp theme integration for highlight colors

**Description**: Map highlight capture names to theme color slots. Default-theme plugin defines syntax colors. Colors configurable via `(set-theme-color)`.

**Acceptance criteria**:
- `(set-theme-color "syntax-keyword" "#c678dd")` changes keyword color
- Default-theme plugin provides a complete set of syntax colors
- Unset theme slots render text with terminal default color
- Theme changes take effect on next render frame

**Architectural constraints**:
- Capture name `@keyword` maps to theme slot `"syntax-keyword"` (convention-based)
- No new Lisp primitives needed -- uses existing `set-theme-color`
- Color resolution happens in `alfred-syntax` (reads from EditorState.theme)

---

## Step 06: Python and JavaScript grammar support

**Description**: Add Python and JavaScript grammars with highlight queries. Language detected from file extension.

**Acceptance criteria**:
- `.py` files highlighted with Python grammar
- `.js` files highlighted with JavaScript grammar
- Python keywords, strings, comments render with correct colors
- JavaScript keywords, strings, comments render with correct colors
- Each grammar adds <500KB to binary size

**Architectural constraints**:
- Grammar crates added as Cargo dependencies (compiled-in)
- Highlight queries bundled per language in `queries/{lang}/highlights.scm`
- Language detection is extension-based lookup, not content sniffing

---

## Step Dependency Graph

```
Step 01 (scaffold)
  |
  v
Step 02 (parse + highlight ranges)
  |
  v
Step 03 (renderer integration)
  |
  v
Step 04 (incremental re-parse)
  |
  v
Step 05 (theme integration)
  |
  v
Step 06 (Python + JS grammars)
```

Steps are sequential. Each builds on the previous.

---

## Quality Gates

Each step must pass before the next begins:

- [ ] All acceptance criteria met
- [ ] `alfred-core` has zero tree-sitter dependency
- [ ] `alfred-syntax` tests pass (parse correctness, highlight accuracy)
- [ ] Existing tests pass (no regressions in rainbow-csv, rendering, etc.)
- [ ] Binary compiles and runs with new crate
