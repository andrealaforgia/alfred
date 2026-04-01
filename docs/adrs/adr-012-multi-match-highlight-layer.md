# ADR-012: Separate Match Highlight Layer on EditorState

## Status

Proposed

## Context

The regex wizard (and future search features) need to highlight multiple match ranges in the buffer simultaneously. Alfred's renderer already supports two per-line coloring mechanisms:

1. **`line_styles`**: `HashMap<usize, Vec<(usize, usize, ThemeColor)>>` -- per-character foreground color segments. Used by syntax highlighting (tree-sitter) to colorize tokens. Cleared and rebuilt on buffer changes by the syntax highlighting plugin.

2. **`line_backgrounds`**: `HashMap<usize, (ThemeColor, ThemeColor)>` -- full-line (fg, bg) pair. Used by browse-mode for cursor-bar highlighting. Only one color pair per line (no range granularity).

The regex wizard needs per-range background highlighting across multiple lines, with potentially many ranges per line. The question: where should match highlight data live on `EditorState`?

**Requirements**:
- Multiple highlight ranges per line (a regex can match several times on one line)
- Background color (not foreground) so syntax highlighting text colors remain visible
- Independent lifecycle from syntax highlighting (match highlights change on pattern edit, syntax highlights change on buffer edit)
- Clearable in bulk (close wizard -> clear all highlights)

## Decision

Add a new field `match_highlights: HashMap<usize, Vec<(usize, usize, ThemeColor)>>` to `EditorState`, separate from both `line_styles` and `line_backgrounds`.

The renderer merges `match_highlights` as a background-color overlay on top of existing styling. Rendering priority:
1. `line_styles` (foreground colors from syntax highlighting)
2. `line_backgrounds` (full-line background)
3. `match_highlights` (per-range background, highest visual priority)

Match highlights set background color only, preserving whatever foreground color `line_styles` already applied to that character position.

## Alternatives Considered

### Alternative 1: Extend `line_styles` to Carry Match Highlights

**Description**: Reuse the existing `line_styles` HashMap. When the regex wizard finds matches, append highlight segments to `line_styles` alongside syntax highlighting segments. Use a convention (e.g., a specific color or a new flag) to distinguish match highlights from syntax highlights.

**Evaluation**:
- (+) No new field on `EditorState` -- zero data model change
- (+) Renderer already processes `line_styles` segments
- (-) **Ownership conflict**: `line_styles` is cleared and rebuilt by the syntax highlighting plugin on every buffer change. If match highlights are mixed in, they are destroyed whenever syntax re-highlights. The wizard would need to re-insert highlights after every syntax rebuild, creating tight coupling between two unrelated subsystems.
- (-) **Semantic confusion**: `line_styles` segments are foreground colors. Match highlights need background colors. Mixing foreground and background semantics in the same data structure requires the renderer to inspect each segment and apply it differently based on an out-of-band flag.
- (-) **Ordering complexity**: `line_styles` segments must be sorted by position for the renderer's cursor sweep. Inserting match highlights into this sorted order on every pattern change is O(n log n) per line.
- (-) **Cleanup**: clearing match highlights requires iterating all lines and filtering segments by type, rather than a simple `clear()`.

**Rejection rationale**: The ownership conflict is the primary disqualifier. Syntax highlighting and match highlighting have independent lifecycles: syntax changes on buffer edit, matches change on pattern edit. Mixing them in one structure creates coupling that the functional-core architecture explicitly avoids.

### Alternative 2: Extend `line_backgrounds` to Support Per-Range Highlighting

**Description**: Change `line_backgrounds` from `HashMap<usize, (ThemeColor, ThemeColor)>` (one pair per line) to `HashMap<usize, Vec<(usize, usize, ThemeColor, ThemeColor)>>` (multiple ranges per line). Regex match highlights and cursor-bar highlights would share this structure.

**Evaluation**:
- (+) Consolidates all background coloring into one system
- (+) No new field -- evolves an existing field
- (-) **Breaking change**: `line_backgrounds` is used by browse-mode for full-line cursor highlighting. Changing its type from a single pair to a Vec of ranges breaks the existing API and all code that reads/writes it.
- (-) **Semantic mismatch**: browse-mode sets one background per line (full width). Match highlights need column-range backgrounds. These are fundamentally different: "this entire line is highlighted" vs "these 3 character ranges on this line are highlighted."
- (-) **API complexity**: the existing `set-line-background` bridge primitive takes (line, fg, bg). A range-based API would need (line, start, end, fg, bg), which is a different primitive anyway.
- (-) **Renderer complication**: the current renderer checks `line_backgrounds` first and applies a full-line style. Changing this to range-based backgrounds requires merging ranges with `line_styles`, which is the same complexity as a new field but with higher regression risk.

**Rejection rationale**: Breaking the existing `line_backgrounds` contract and its consumers for a semantically different use case is unjustified. Full-line backgrounds and column-range highlights warrant separate data structures.

### Alternative 3: Store Highlights in the Plugin (Lisp State) Instead of EditorState

**Description**: Keep match positions as Lisp lists in the plugin. On each render cycle, the plugin writes highlights to `line_styles` or `line_backgrounds` just before rendering.

**Evaluation**:
- (+) No Rust data model changes at all
- (+) Plugin has full control over highlight lifecycle
- (-) **No pre-render hook**: Alfred's render loop reads `EditorState` directly. There is no "before render" callback where Lisp can inject styles. The plugin would need to write styles after every pattern change AND after every buffer edit (because line numbers shift). This creates fragile timing dependencies.
- (-) **Performance**: converting a Lisp list of thousands of match positions into `set-line-style` calls (one bridge call per range) would be extremely slow. A 500-match result requires 500 bridge invocations.
- (-) **Still conflicts with `line_styles`**: even if written just-in-time, these highlights mix with syntax highlighting in the same structure (same problems as Alternative 1).

**Rejection rationale**: Without a pre-render hook, the timing is unreliable. The per-range bridge call overhead makes this impractical for buffers with many matches.

## Consequences

### Positive
- Clean ownership: `line_styles` owned by syntax highlighting, `match_highlights` owned by search/wizard features, `line_backgrounds` owned by cursor-bar features. No cross-contamination.
- Simple lifecycle: `clear-match-highlights` is a single `HashMap::clear()` call
- `regex-find-all` writes directly to `match_highlights` in one operation (no per-range bridge calls from Lisp)
- Renderer change is additive: existing `line_styles` and `line_backgrounds` logic untouched
- Future reuse: `match_highlights` can serve incremental search (`/` command), find-and-replace, and any feature that needs transient range highlighting

### Negative
- Third highlighting-related field on `EditorState` (alongside `line_styles` and `line_backgrounds`) -- developers must understand which to use
- Renderer must process one more data source per visible line (negligible cost: one HashMap lookup per line)
- 3-4 new bridge primitives to maintain (`regex-find-all`, `regex-valid?`, `set-match-highlight`, `clear-match-highlights`)

### Neutral
- The type `HashMap<usize, Vec<(usize, usize, ThemeColor)>>` is identical to `line_styles`. A type alias could reduce duplication, but this is a crafter decision (internal structure)
- `match_highlights` applies background color; `line_styles` applies foreground color. The renderer merges them by composing foreground from `line_styles` with background from `match_highlights`
