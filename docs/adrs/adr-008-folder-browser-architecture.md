# ADR-008: Folder Browser Architecture

## Status

Accepted

## Context

Alfred needs a folder browser feature activated when the user runs `alfred .` or `alfred <directory>`. The feature introduces a new `browse` mode with vim-style navigation, directory traversal, and file opening. It must integrate with the existing mode system (normal/insert/visual), keymap system, command registry, panel system (status bar), and rendering pipeline.

Three architectural approaches were considered:

**Quality attribute priorities** (from requirements):
- Maintainability: HIGH -- must fit cleanly into existing 6-crate architecture
- Testability: HIGH -- pure domain logic must be testable without filesystem
- Time-to-market: MEDIUM -- single developer, ~1-2 weeks delivery
- Performance: MEDIUM -- fast directory listing, but no async/streaming needed

**Constraints**:
- Single developer + AI pair programming
- No new external dependencies preferred
- Functional-core / imperative-shell architecture (CLAUDE.md)
- Existing crate boundaries: alfred-core (pure), alfred-tui (IO shell), alfred-bin (composition root)

## Decision

**Approach A: Core-only** -- Add BrowserState and browser commands to alfred-core, render in alfred-tui, keybindings in a Lisp plugin. No new crate.

In the context of adding a folder browser to an existing 6-crate editor, facing the need to integrate with the mode system, command registry, keymap system, and rendering pipeline, we decided for the core-only approach to achieve minimal coupling and maximal reuse of existing infrastructure, accepting that all browser logic shares the alfred-core module namespace.

## Alternatives Considered

### Alternative B: New Crate (alfred-browser)

**Description**: Create a new `alfred-browser` crate containing BrowserState, DirEntry types, and browser command functions. alfred-core would not depend on alfred-browser; instead, alfred-tui and alfred-bin would depend on both.

**Evaluation**:
- (+) Clean module boundary for browser-specific code
- (+) Could be compiled independently
- (-) BrowserState needs to live on EditorState (alfred-core), creating a circular dependency or requiring a trait/generic on EditorState
- (-) Browser commands need to mutate EditorState (mode, buffer, keymaps), requiring alfred-browser to depend on alfred-core anyway
- (-) Additional crate for ~200-400 lines of domain code is over-engineering
- (-) Increases compile time and workspace complexity for no isolation benefit
- (-) The browser is a mode of the editor, not a standalone capability

**Rejection rationale**: The browser is not independently deployable or independently testable without EditorState. A separate crate creates coupling problems (EditorState ownership) without providing meaningful modularity gains. The crate boundary would be artificial.

### Alternative C: Lisp Plugin Only

**Description**: Implement the entire folder browser as a Lisp plugin, adding new Lisp primitives for `(read-dir path)`, `(entry-type entry)`, and `(open-file path)`, with the browser state and rendering managed in Lisp.

**Evaluation**:
- (+) Aligns with plugin-first philosophy (ADR-002)
- (+) Maximum extensibility -- users could customize browser behavior
- (-) The Lisp runtime lacks rendering primitives for list-based UIs (no scrollable list widget)
- (-) Would require significant new bridge primitives: directory listing, entry classification, binary detection, symlink resolution, list rendering with cursor
- (-) Performance concern: Lisp loop over 1000+ entries for rendering on every keypress
- (-) Current Lisp bridge is designed for configuration and commands, not complex UI state management
- (-) Existing Lisp primitives operate on buffer/cursor/panels -- browser needs different rendering model

**Rejection rationale**: The Lisp runtime is designed for configuration, keymaps, and simple stateless commands. A complex stateful UI component with filesystem IO, custom rendering, and scroll management exceeds what the current Lisp bridge can efficiently support. This would require building a general-purpose list UI framework in Lisp primitives, which is a larger effort than the feature itself. Future: once Alfred has a general list-selection UI primitive, the browser could be partially migrated to Lisp.

## Consequences

### Positive
- Zero new dependencies -- uses only std::fs and existing workspace crates
- Browser commands are pure functions in alfred-core, highly testable
- Follows existing patterns exactly: mode constant, keymap, commands, panel, rendering branch
- Lisp plugin for keybindings maintains plugin-first philosophy for configuration
- Minimal diff: ~1 new module in alfred-core, ~1 rendering branch in alfred-tui, ~1 classification block in alfred-bin, ~1 new plugin

### Negative
- alfred-core grows by one module (browser) -- acceptable given it already has 12 modules
- Browser rendering logic in alfred-tui requires a conditional branch in render_frame -- adds complexity to the renderer
- The `gg` (jump to first) key sequence needs special handling: either a PendingG input state or simplification to single-`g` in browse mode (crafter decides)

### Neutral
- BrowserState as `Option<BrowserState>` on EditorState: None when not browsing, Some when browsing. This is the same pattern used for other optional state (selection_start, macro_recording, etc.)
- Future enhancements (file search, preview, gitignore filtering) can be added to the browser module without architectural changes
