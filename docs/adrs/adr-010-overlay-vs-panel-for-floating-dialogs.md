# ADR-010: Overlay as Separate System vs. Panel Extension

## Status

Proposed

## Context

Alfred needs a floating centered dialog for file search (and potentially future use cases: command palette, goto-line, find-and-replace). The existing panel system supports edge-docked regions (top, bottom, left, right) with line-based content, priority ordering, and per-line styling.

The key question: should floating dialogs be implemented as an extension to the panel system or as a separate overlay system?

**Quality attribute priorities**:
- Simplicity: minimal Rust surface area, easy for Lisp plugins to use
- Extensibility: other plugins should be able to create overlays without Rust changes
- Rendering correctness: overlay must draw on top of all other content, centered
- Input isolation: when overlay is open, keys must route to overlay keymap only

**Constraints**:
- Plugin-first: Rust provides generic primitives, Lisp implements behavior
- Functional-core: overlay state is data, rendering is a pure transformation
- Single-process TUI: ratatui immediate-mode rendering (later widgets overwrite earlier ones)

## Decision

Implement the overlay as a **separate system** from panels, with its own data model in `alfred-core/overlay.rs` and its own rendering pass in `alfred-tui/renderer.rs`.

The overlay is a single instance on `EditorState` (not a registry like panels). It has its own data model optimized for the overlay use case: input field, items list with cursor, title, dimensions, scroll offset. Rendering draws it last (on top of everything) as a bordered, centered rectangle.

## Alternatives Considered

### Alternative 1: Extend PanelPosition with Floating Variant

**Description**: Add `PanelPosition::Floating { width, height }` to the existing `PanelPosition` enum. Floating panels are centered and rendered last.

**Evaluation**:
- (+) Single system to learn -- plugins use the same panel API
- (+) Reuses existing panel infrastructure (lines, styles, cursor, content)
- (-) Panels use a `HashMap<usize, String>` lines model. An overlay needs a dedicated input field (separate from result lines) -- this semantic distinction does not exist in panels
- (-) Panel rendering iterates edges (top, bottom, left, right). Floating requires a separate centered-rect computation that shares no logic with edge docking
- (-) Panel cursor position is relative to panel lines. Overlay cursor position must be at the input field end -- different semantics
- (-) Adds complexity to every `panels_at()` call (must filter out floating panels from edge layouts)
- (-) The `PanelRegistry` stores a flat `Vec<Panel>`. Floating panels in this vec complicate iteration for edge-docked layout math

**Rejection rationale**: The panel model is optimized for edge-docked regions. Floating overlay semantics (centered positioning, input+results composite layout, cursor at input field) diverge enough that forcing them into the panel model creates accidental complexity. The two systems share no rendering logic.

### Alternative 2: Generic Dialog System with Multiple Overlay Types

**Description**: Build a full dialog system supporting multiple simultaneous overlays with z-ordering, configurable positioning (centered, top-right, bottom-left), and multiple input fields.

**Evaluation**:
- (+) Handles future use cases (multi-field dialogs, stacked modals)
- (+) Maximum flexibility
- (-) Over-engineering: no identified use case requires multiple simultaneous overlays in a TUI editor
- (-) Z-ordering and positioning configuration adds API surface without near-term value
- (-) Multi-field input routing requires complex focus management
- (-) Violates simplest-solution principle

**Rejection rationale**: YAGNI. TUI editors (vim, helix, kakoune) use single-overlay patterns. A single overlay instance covers file search, command palette, goto-line, and find-and-replace (all are open-one-close-before-next). If concurrent overlays are ever needed, the single-overlay model can be promoted to a stack.

### Alternative 3: Render Search Results in the Message Line / Command Line

**Description**: Use the existing message line (bottom row) as the search input and display results inline below the status bar.

**Evaluation**:
- (+) Zero new Rust infrastructure -- uses existing message primitive
- (+) Familiar pattern (vim's `/` search uses the command line)
- (-) Message line is 1 row -- cannot show results alongside the query
- (-) Would need to repurpose multiple bottom rows, conflicting with status bar panel
- (-) No visual distinction between overlay and editor content
- (-) User explicitly requested a centered floating dialog with ~65-char width

**Rejection rationale**: The message line is too constrained for a results list. The user's design requirement specifically calls for a floating overlay, not a command-line interaction.

## Consequences

### Positive
- Clean separation: overlay model is purpose-built for its use case (input field + items + cursor + dimensions)
- No impact on existing panel system -- zero risk of regression
- Simple Lisp API: `open-overlay`, `close-overlay`, `overlay-set-items`, etc.
- Renderer addition is isolated: one new block at the end of `render_frame`
- Extensible without Rust changes: any Lisp plugin can use the same overlay primitives for different purposes

### Negative
- Two "layer" systems in the codebase (panels and overlay) -- developers must know which to use
- Overlay primitives add ~10 new Lisp bridge functions to maintain
- Single overlay instance means plugins must coordinate (only one overlay at a time)

### Neutral
- ADR-009 (file search in sidebar panel) is superseded for the project-wide search use case. The `/` current-directory filter in browse-mode remains unchanged.
- The overlay system does not replace panels for docked regions -- both systems coexist
