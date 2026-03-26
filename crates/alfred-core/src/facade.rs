//! Facade: controlled API layer over EditorState.
//!
//! This module provides high-level free functions that operate on EditorState,
//! hiding the internal submodule structure (buffer, cursor, viewport, etc.).
//! Callers use `facade::buffer_content(state)` instead of
//! `buffer::content(&state.buffer)`, reducing structural coupling.

use crate::buffer;
use crate::cursor::{self, Cursor};
use crate::editor_state::EditorState;
use crate::key_event::KeyEvent;
use crate::viewport;

// ---------------------------------------------------------------------------
// Buffer queries
// ---------------------------------------------------------------------------

/// Returns the full text content of the editor's buffer.
pub fn buffer_content(state: &EditorState) -> String {
    buffer::content(&state.buffer)
}

/// Returns the number of lines in the editor's buffer.
pub fn buffer_line_count(state: &EditorState) -> usize {
    buffer::line_count(&state.buffer)
}

/// Returns the text of a specific line (with trailing newline), or None if out of range.
pub fn buffer_get_line(state: &EditorState, index: usize) -> Option<&str> {
    buffer::get_line(&state.buffer, index)
}

// ---------------------------------------------------------------------------
// Cursor queries and construction
// ---------------------------------------------------------------------------

/// Returns the current cursor position as (line, column).
pub fn cursor_position(state: &EditorState) -> (usize, usize) {
    (state.cursor.line, state.cursor.column)
}

/// Creates a new Cursor at the given line and column.
pub fn cursor_new(line: usize, column: usize) -> Cursor {
    cursor::new(line, column)
}

/// Returns a cursor clamped to the buffer's bounds.
pub fn cursor_ensure_within_bounds(cur: Cursor, state: &EditorState) -> Cursor {
    cursor::ensure_within_bounds(cur, &state.buffer)
}

// ---------------------------------------------------------------------------
// Viewport
// ---------------------------------------------------------------------------

/// Adjusts the viewport so the cursor is visible, returning the new viewport.
pub fn viewport_adjust(state: &EditorState) -> crate::viewport::Viewport {
    viewport::adjust(state.viewport, &state.cursor)
}

// ---------------------------------------------------------------------------
// Key resolution
// ---------------------------------------------------------------------------

/// Resolves a key event to a command name using the active keymaps.
pub fn resolve_key(state: &EditorState, key: KeyEvent) -> Option<String> {
    crate::editor_state::resolve_key(state, key)
}
