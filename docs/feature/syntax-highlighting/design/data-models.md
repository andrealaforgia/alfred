# Syntax Highlighting -- Data Models

**Feature**: syntax-highlighting
**Date**: 2026-03-24

---

## 1. SyntaxHighlighter

Top-level struct managing tree-sitter parsing state.

**Conceptual fields**:

| Field | Type (conceptual) | Purpose |
|-------|-------------------|---------|
| parser | tree_sitter::Parser | Reusable parser instance |
| current_tree | Option&lt;tree_sitter::Tree&gt; | Most recent parse tree (None before first parse) |
| current_language | Option&lt;LanguageId&gt; | Which language is currently set on the parser |
| language_configs | HashMap&lt;LanguageId, LanguageConfig&gt; | Registered language configurations |
| buffer_version | u64 | Last parsed buffer version (for staleness detection) |

**Key behaviors**:
- Parse: given text, produce/update tree
- Edit: given edit descriptor, notify tree of change before re-parse
- Highlight: given line range and theme, produce styled segments
- Set language: switch parser to a different grammar

---

## 2. LanguageConfig

Configuration for a single supported language.

| Field | Type (conceptual) | Purpose |
|-------|-------------------|---------|
| id | LanguageId (enum or string) | Language identifier (e.g., "rust", "python") |
| extensions | Vec&lt;String&gt; | File extensions (e.g., [".rs"], [".py", ".pyi"]) |
| grammar | tree_sitter::Language | Compiled grammar from grammar crate |
| highlight_query | String | SCM highlight query text (loaded at build time) |

---

## 3. LanguageId

Identifies a supported language. Can be an enum or string -- crafter decides.

**If enum**: `Rust`, `Python`, `JavaScript`
**If string**: `"rust"`, `"python"`, `"javascript"`

The enum approach provides compile-time exhaustiveness checking. The string approach is more extensible. Both are valid -- the crafter chooses based on how language addition should work.

---

## 4. HighlightRange (output of highlight query)

A single highlight capture for a region of text.

| Field | Type | Purpose |
|-------|------|---------|
| line | usize | Buffer line number (0-indexed) |
| start_col | usize | Start column (byte offset within line) |
| end_col | usize | End column (byte offset within line, exclusive) |
| capture_name | String | Tree-sitter capture name (e.g., "keyword", "string") |

This is the intermediate format before theme color resolution. The theme color is looked up at render time by mapping `capture_name` to `"syntax-{capture_name}"` and reading from `EditorState.theme`.

---

## 5. EditNotification

Describes a buffer edit for tree-sitter's incremental parsing.

| Field | Type | Purpose |
|-------|------|---------|
| start_byte | usize | Byte offset where edit begins |
| old_end_byte | usize | Byte offset where old text ended |
| new_end_byte | usize | Byte offset where new text ends |
| start_position | (usize, usize) | (row, column) where edit begins |
| old_end_position | (usize, usize) | (row, column) where old text ended |
| new_end_position | (usize, usize) | (row, column) where new text ends |

Maps directly to tree-sitter's `InputEdit` struct. Computed from the buffer operation parameters (line, column, inserted/deleted text length).

---

## 6. Highlight Group -> Theme Slot Mapping

Convention-based mapping. No explicit data structure needed.

| Tree-sitter Capture | Theme Slot Name | Typical Color (Catppuccin) |
|---------------------|----------------|---------------------------|
| `@keyword` | `syntax-keyword` | mauve (#c678dd) |
| `@function` | `syntax-function` | blue (#89b4fa) |
| `@function.method` | `syntax-function` | blue (#89b4fa) |
| `@string` | `syntax-string` | green (#a6e3a1) |
| `@comment` | `syntax-comment` | overlay0 (#6c7086) |
| `@type` | `syntax-type` | yellow (#f9e2ae) |
| `@variable` | `syntax-variable` | text (#cdd6f4) |
| `@variable.parameter` | `syntax-variable` | text (#cdd6f4) |
| `@operator` | `syntax-operator` | sky (#89dceb) |
| `@number` | `syntax-number` | peach (#fab387) |
| `@punctuation` | `syntax-punctuation` | overlay2 (#9399b2) |
| `@punctuation.bracket` | `syntax-punctuation` | overlay2 (#9399b2) |
| `@constant` | `syntax-constant` | peach (#fab387) |
| `@property` | `syntax-property` | lavender (#b4befe) |
| `@attribute` | `syntax-attribute` | yellow (#f9e2ae) |

**Hierarchical resolution**: `@function.method` falls back to `@function` if `syntax-function.method` is not in the theme. The crafter decides whether to implement this fallback or flatten all sub-captures to their parent.

---

## 7. Buffer Text Access (new addition to alfred-core)

Tree-sitter's parser can accept a callback for text access:

```
FnMut(byte_offset: usize, position: Point) -> &[u8]
```

This avoids materializing the entire rope as a contiguous `String`. The Buffer needs to expose a way for `alfred-syntax` to create this callback. Conceptually:

- **Input**: byte offset
- **Output**: byte slice starting at that offset (can be a single rope chunk)

This is a pure data accessor. It does not require tree-sitter types in `alfred-core`. The `alfred-syntax` crate wraps it into the closure tree-sitter expects.

**Alternative**: For simplicity in Step 02, the crafter may use `buffer::content()` (returns full String) and optimize to chunk-based access in Step 04 when performance matters. Both approaches are valid.
