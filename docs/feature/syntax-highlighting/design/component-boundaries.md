# Syntax Highlighting -- Component Boundaries

**Feature**: syntax-highlighting
**Date**: 2026-03-24

---

## Crate Responsibility Matrix

| Crate | Syntax Highlighting Role | Changes Required |
|-------|------------------------|------------------|
| `alfred-syntax` (NEW) | All tree-sitter interaction: parsing, queries, language detection, segment production | New crate |
| `alfred-core` | Provides Buffer text access + ThemeColor type + line_styles storage | Minor: add byte-offset text accessor on Buffer |
| `alfred-tui` | Orchestrates: notifies highlighter on edit, queries before render, writes line_styles | Add dependency on alfred-syntax, add highlight calls in event loop |
| `alfred-lisp` | No direct changes. Existing `set-theme-color` used for syntax colors | None |
| `alfred-plugin` | No changes. Default-theme plugin updated to include syntax colors | None (plugin Lisp file updated) |
| `alfred-bin` | Creates SyntaxHighlighter, passes to TUI | Add dependency on alfred-syntax, wiring code |

---

## alfred-syntax -- Internal Structure

```
crates/alfred-syntax/
  Cargo.toml
  src/
    lib.rs              -- Public API: SyntaxHighlighter, highlight_lines, language detection
    highlighter.rs      -- SyntaxHighlighter struct, parse/re-parse/query orchestration
    language.rs         -- Language registry: extension -> grammar + query mapping
    query.rs            -- Highlight query execution, capture name -> theme slot mapping
  queries/
    rust/
      highlights.scm    -- Tree-sitter highlight query for Rust
    python/
      highlights.scm    -- Tree-sitter highlight query for Python
    javascript/
      highlights.scm    -- Tree-sitter highlight query for JavaScript
```

**Note**: The crafter determines exact module decomposition. The above is a logical breakdown, not a mandate.

---

## Boundary: alfred-syntax <-> alfred-core

**Direction**: alfred-syntax depends on alfred-core (inward dependency).

**What alfred-syntax reads from alfred-core**:
- `Buffer` -- text content via `get_line()`, `line_count()`, `content()`, `version()`
- `Buffer` -- byte-offset text access (new method needed for tree-sitter parse callback)
- `ThemeColor` -- to produce typed color values for line_styles

**What alfred-syntax does NOT access**:
- `EditorState` -- the syntax crate produces highlight data but does not write it to state directly. The TUI orchestrator writes segments to `line_styles`
- `Cursor`, `Viewport`, `CommandRegistry`, etc. -- irrelevant to highlighting

---

## Boundary: alfred-tui <-> alfred-syntax

**Direction**: alfred-tui depends on alfred-syntax.

**TUI's role** (orchestrator):
1. Holds the `SyntaxHighlighter` instance (not on EditorState)
2. On file open: calls `highlighter.set_language(filename)` and `highlighter.parse(buffer_text)`
3. On buffer edit: calls `highlighter.edit(edit_info)` then `highlighter.parse(buffer_text)`
4. Before render: calls `highlighter.highlight_lines(line_range, theme)` -> receives segments
5. Writes segments to `EditorState.line_styles` via `add_line_style()`

**Why the TUI orchestrates**: The highlighter needs both the buffer text (from core) and the parsed tree (internal state). The TUI already has access to both EditorState and the event loop. Placing orchestration here keeps alfred-syntax free of EditorState coupling.

---

## Boundary: alfred-lisp <-> Theme System

**No new Lisp primitives needed**. The existing theme primitives suffice:

- `(set-theme-color "syntax-keyword" "#c678dd")` -- sets keyword highlight color
- `(get-theme-color "syntax-keyword")` -- reads current color
- `(define-theme "my-theme" ...)` -- can include syntax colors in theme definitions

The default-theme plugin (`plugins/default-theme/init.lisp`) will be updated to include default syntax highlight colors for all capture groups.

---

## Boundary: alfred-bin <-> alfred-syntax

**alfred-bin** is the composition root. It:
1. Creates a `SyntaxHighlighter` with all compiled grammars registered
2. Passes it to the TUI event loop (ownership transfer or shared reference)

This follows the existing pattern: alfred-bin creates LispRuntime, creates EditorState, wires everything, then calls `alfred_tui::app::run()`.
