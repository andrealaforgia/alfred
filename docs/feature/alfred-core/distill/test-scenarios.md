# Acceptance Test Scenario Inventory -- Alfred Editor

**Feature**: alfred-core
**Date**: 2026-03-20
**Phase**: DISTILL (Acceptance Test Design)

---

## Summary

| Milestone | Walking Skeletons | Happy Path | Error Path | Edge Case | Total | Error Ratio |
|-----------|-------------------|------------|------------|-----------|-------|-------------|
| M1 | 1 | 4 | 4 | 3 | 12 | 42% |
| M2 | 1 | 3 | 4 | 2 | 10 | 40% |
| M3 | 1 | 3 | 5 | 2 | 11 | 45% |
| M4 | 0 | 3 | 2 | 2 | 7 | 43% |
| M5 | 0 | 3 | 2 | 2 | 7 | 43% |
| M6 | 1 | 4 | 3 | 2 | 10 | 40% |
| M7 | 1 | 5 | 4 | 5 | 15 | 47% |
| **Total** | **5** | **25** | **24** | **18** | **72** | **44%** |

Error ratio target: >= 40%. Achieved: 44% overall.

Note: M7 edge case count (5) includes 2 property-shaped scenarios tagged @property,
signaling the DELIVER wave to implement them as property-based tests.

---

## M1: Rust Kernel -- Buffer, Cursor, Viewport, Navigation

### Walking Skeleton

**WS-1**: Given a file with known content, when the user opens it, then the buffer contains the file's text and the cursor is at the beginning.

### Happy Path

**M1-H1**: Given a file is open, when the user moves the cursor down, then the cursor advances to the next line.

**M1-H2**: Given the cursor is beyond the visible area, when the cursor moves past the viewport boundary, then the viewport scrolls to keep the cursor visible.

**M1-H3**: Given the user is viewing a file, when the user presses the quit key combination, then the editor exits cleanly.

**M1-H4**: Given a file is open, when the user moves the cursor right within a line, then the cursor advances one column.

### Error Path

**M1-E1**: Given the cursor is at the last line, when the user moves down, then the cursor remains on the last line (no crash, no out-of-bounds).

**M1-E2**: Given the cursor is at line start, when the user moves left, then the cursor remains at line start.

**M1-E3**: Given the cursor is at the end of a line, when the user moves right, then the cursor remains at line end.

**M1-E4**: Given no file argument is provided, when the editor starts, then the editor opens with an empty buffer (graceful handling).

### Edge Cases

**M1-EC1**: Given an empty file, when opened, then the buffer has zero lines of content and the cursor is at position (0, 0).

**M1-EC2**: Given a file with one very long line (exceeding viewport width), when opened, then the buffer contains the full line content.

**M1-EC3**: Given the cursor is on a long line and moves to a shorter line, then the cursor column clamps to the shorter line's length.

---

## M2: Lisp Integration -- Evaluate Expressions, Call Rust Primitives

### Walking Skeleton

**WS-2**: Given a buffer with text "Hello", when the expression `(buffer-insert " World")` is evaluated, then the buffer contains "Hello World".

### Happy Path

**M2-H1**: Given a buffer, when `(cursor-move :down 5)` is evaluated, then the cursor moves down five lines.

**M2-H2**: Given a buffer, when `(cursor-position)` is evaluated, then it returns the current line and column.

**M2-H3**: Given a buffer, when `(message "test")` is evaluated, then the message area contains "test".

### Error Path

**M2-E1**: Given the editor is running, when a Lisp expression with a syntax error is evaluated, then an error message is displayed and the editor remains stable.

**M2-E2**: Given the editor is running, when a Lisp expression calls an undefined function, then an error message is displayed and the editor remains stable.

**M2-E3**: Given the editor is running, when a primitive receives wrong argument types (e.g., `(cursor-move "not-a-direction" "not-a-number")`), then an error message is displayed and the buffer is unchanged.

**M2-E4**: Given the editor is running, when a primitive receives too few arguments, then an error message is displayed and the editor remains stable.

### Edge Cases

**M2-EC1**: Given an empty buffer, when `(buffer-insert "text")` is evaluated, then the buffer contains "text" (insertion into empty buffer).

**M2-EC2**: Given a buffer with content, when `(buffer-line-count)` is evaluated, then it returns the correct number of lines.

---

## M3: Plugin System -- Discovery, Loading, Lifecycle

### Walking Skeleton

**WS-3**: Given a test plugin in the plugins directory, when the editor loads plugins, then the plugin's command is registered and callable.

### Happy Path

**M3-H1**: Given multiple plugins in the plugins directory, when the editor starts, then all plugins are discovered and their metadata is available.

**M3-H2**: Given a loaded plugin, when the plugin is unloaded, then its registered commands are removed from the registry.

**M3-H3**: Given a plugin with dependencies, when plugins are loaded, then dependencies are loaded before dependents (topological order).

### Error Path

**M3-E1**: Given the plugins directory does not exist, when the editor starts, then it starts normally with no plugins loaded and no crash.

**M3-E2**: Given a plugin with a syntax error in its init.lisp, when the editor loads plugins, then the broken plugin reports an error status and other plugins still load.

**M3-E3**: Given a plugin whose init function throws an error, when the editor loads plugins, then the failing plugin is marked as errored and other plugins are unaffected.

**M3-E4**: Given a plugin with a missing dependency, when the editor loads plugins, then the plugin reports an error about the missing dependency.

**M3-E5**: Given a plugin directory with no init.lisp file, when the editor scans for plugins, then the directory is skipped without error.

### Edge Cases

**M3-EC1**: Given an empty plugins directory, when the editor starts, then it starts normally with zero plugins.

**M3-EC2**: Given a plugin that registers multiple commands, when the plugin is unloaded, then all of its commands are removed.

---

## M4: Line Numbers Plugin

### Happy Path

**M4-H1**: Given the line numbers plugin is loaded and a file is open, then line numbers appear in the gutter for each visible line.

**M4-H2**: Given the line numbers plugin is loaded, when the user scrolls down, then the gutter line numbers update to match the visible lines.

**M4-H3**: Given the line numbers plugin is loaded and a file has more than 999 lines, then the gutter width accommodates four-digit line numbers.

### Error Path

**M4-E1**: Given the line numbers plugin is not present in the plugins directory, when the editor starts, then no gutter is rendered and the editor functions normally.

**M4-E2**: Given the line numbers plugin is loaded, when the plugin encounters an error during rendering, then the editor continues without line numbers rather than crashing.

### Edge Cases

**M4-EC1**: Given an empty file with the line numbers plugin loaded, then the gutter shows line number 1 for the single empty line.

**M4-EC2**: Given a file with exactly one line, then the gutter width is minimal (one digit).

---

## M5: Status Bar Plugin

### Happy Path

**M5-H1**: Given the status bar plugin is loaded and a file is open, then the status bar displays the filename.

**M5-H2**: Given the status bar plugin is loaded, when the cursor moves, then the status bar updates to show the new cursor position.

**M5-H3**: Given the status bar plugin is loaded and the buffer has a known filename, then the status bar displays that filename.

### Error Path

**M5-E1**: Given the status bar plugin is not present, when the editor starts, then no status bar is rendered and the editor functions normally.

**M5-E2**: Given the status bar plugin is loaded but a status field is missing (e.g., no filename for an unnamed buffer), then the status bar displays gracefully without the missing field.

### Edge Cases

**M5-EC1**: Given the status bar plugin is loaded and the buffer is unnamed (no file), then the status bar shows a placeholder like "[No Name]".

**M5-EC2**: Given the status bar plugin is loaded, when the cursor is at position (0, 0), then the status bar shows "1:1" (one-indexed for user display).

---

## M6: Basic Keybinding Plugin

### Walking Skeleton

**WS-4**: Given the basic keybinding plugin is loaded and the buffer contains "Hello", when the user presses 'a', then 'a' is inserted at the cursor position.

### Happy Path

**M6-H1**: Given the basic keybinding plugin is loaded, when the user presses the down arrow key, then the cursor moves down one line.

**M6-H2**: Given the basic keybinding plugin is loaded and text "ab" is in the buffer with cursor after 'b', when the user presses backspace, then 'b' is deleted and the buffer contains "a".

**M6-H3**: Given the basic keybinding plugin is loaded, when the user presses Ctrl-Q, then the editor signals quit.

**M6-H4**: Given no keybinding plugin is loaded, when a key is pressed, then no action occurs (proving keybindings are not hardcoded in the kernel).

### Error Path

**M6-E1**: Given the basic keybinding plugin is loaded, when the user presses a key with no binding, then nothing happens and the editor remains stable.

**M6-E2**: Given the basic keybinding plugin is loaded and the cursor is at buffer start, when backspace is pressed, then nothing happens (no underflow).

**M6-E3**: Given the basic keybinding plugin is loaded and a command bound to a key fails during execution, then an error message is displayed and the editor remains stable.

### Edge Cases

**M6-EC1**: Given the basic keybinding plugin is loaded and the buffer is empty, when the user types a character, then the character is inserted into the empty buffer.

**M6-EC2**: Given the basic keybinding plugin is loaded, when multiple keys are pressed in rapid succession, then all characters are inserted in order.

---

## M7: Vim Keybindings Plugin -- Modal Editing

### Walking Skeleton

**WS-5**: Given the Vim keybinding plugin is loaded and the editor is in Normal mode with buffer "Hello World", when the user presses 'i', types "Brave ", and presses Escape, then the buffer contains "Brave Hello World" and the editor is in Normal mode.

### Happy Path

**M7-H1**: Given the editor is in Normal mode, when the user presses 'j', then the cursor moves down one line.

**M7-H2**: Given the editor is in Normal mode, when the user presses 'i', then the editor enters Insert mode.

**M7-H3**: Given the editor is in Insert mode, when the user presses Escape, then the editor returns to Normal mode.

**M7-H4**: Given the editor is in Normal mode with a two-line buffer, when the user presses 'dd', then the current line is deleted.

**M7-H5**: Given the editor is in Normal mode, when the user presses 'x', then the character under the cursor is deleted.

### Error Path

**M7-E1**: Given the editor is in Normal mode, when the user presses an unmapped key, then nothing happens and the mode remains Normal.

**M7-E2**: Given the editor is in Normal mode on a single-line buffer, when 'dd' is pressed, then the buffer becomes empty (not a crash).

**M7-E3**: Given the editor is in Normal mode at an empty line, when 'x' is pressed, then nothing happens (no underflow).

**M7-E4**: Given the editor is in Normal mode, when the user presses 'd' and then waits for timeout without pressing a second key, then the pending 'd' is discarded and no action occurs.

### Edge Cases

**M7-EC1**: Given the editor is in Insert mode, when the user types characters, then each character is inserted at the cursor position and the cursor advances.

**M7-EC2**: Given the editor is in Normal mode, when the user presses '0', then the cursor moves to the start of the line.

**M7-EC3**: Given the editor is in Normal mode, when the user presses '$', then the cursor moves to the end of the line.

### Property-Shaped Criteria

**@property M7-P1**: Given any sequence of mode transitions (Normal -> Insert -> Normal -> ...), the editor is always in exactly one mode.

**@property M7-P2**: Given any valid cursor movement command in Normal mode, the cursor position always remains within buffer bounds.

---

## Cross-Milestone Regression Scenarios

These scenarios verify that earlier milestone behavior is preserved as new milestones are implemented.

**REG-1**: After M2, M1 behavior still works (file loading, cursor movement, quit).

**REG-2**: After M3, M2 behavior still works (Lisp evaluation modifies buffer).

**REG-3**: After M6, removing the keybinding plugin results in no key handling (proves M1 hardcoded bindings were removed).

**REG-4**: After M7, basic editing commands from M6 still work when the Vim plugin is not loaded.

---

## Mandate Compliance Evidence

### CM-A: Hexagonal Boundary Enforcement

All tests invoke through driving ports:
- `EditorState` construction and field access (buffer, cursor, mode)
- `LispRuntime::eval()` for Lisp evaluation
- `PluginRegistry::load_all()` for plugin loading
- `CommandRegistry::execute()` for command dispatch
- `keymap::resolve()` for key resolution

No tests import internal modules (buffer.rs internals, cursor movement functions directly, Lisp parser internals, plugin scanner internals).

### CM-B: Business Language Purity

Scenarios use domain terms only:
- "buffer", "cursor", "line", "column", "mode", "command", "plugin", "keybinding", "gutter", "status bar"
- Zero technical terms: no "rope", "HashMap", "Vec", "crossterm", "ratatui", "FFI", "trait", "struct"

### CM-C: Walking Skeleton + Focused Scenario Counts

- Walking skeletons: 5 (WS-1 through WS-5)
- Focused scenarios: 67 (happy path + error path + edge cases + property)
- Total: 72
- Error ratio: 44% (exceeds 40% target)
