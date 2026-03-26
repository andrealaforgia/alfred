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

/// Returns the text content of a specific line (trailing newline stripped).
pub fn buffer_get_line_content(state: &EditorState, index: usize) -> String {
    buffer::get_line_content(&state.buffer, index)
}

/// Returns the buffer's filename, or None if the buffer has no file path.
pub fn buffer_filename(state: &EditorState) -> Option<&str> {
    state.buffer.filename()
}

/// Returns whether the buffer has been modified since last save.
pub fn buffer_is_modified(state: &EditorState) -> bool {
    state.buffer.is_modified()
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
// Mode
// ---------------------------------------------------------------------------

/// Returns the current editor mode name (e.g., "normal", "insert").
pub fn current_mode(state: &EditorState) -> &str {
    &state.mode
}

// ---------------------------------------------------------------------------
// Viewport
// ---------------------------------------------------------------------------

/// Returns the first visible line number (0-indexed).
pub fn viewport_top_line(state: &EditorState) -> usize {
    state.viewport.top_line
}

/// Returns the number of visible lines in the viewport.
pub fn viewport_height(state: &EditorState) -> u16 {
    state.viewport.height
}

/// Adjusts the viewport so the cursor is visible, returning the new viewport.
pub fn viewport_adjust(state: &EditorState) -> crate::viewport::Viewport {
    viewport::adjust(state.viewport, &state.cursor)
}

// ---------------------------------------------------------------------------
// Tab width
// ---------------------------------------------------------------------------

/// Returns the current tab width (number of spaces per Tab).
pub fn tab_width(state: &EditorState) -> usize {
    state.tab_width
}

// ---------------------------------------------------------------------------
// Cursor shape
// ---------------------------------------------------------------------------

/// Returns the cursor shape name for a given mode, or None if not configured.
pub fn cursor_shape<'a>(state: &'a EditorState, mode: &str) -> Option<&'a str> {
    state.cursor_shapes.get(mode).map(|s| s.as_str())
}

// ---------------------------------------------------------------------------
// Undo and jump list
// ---------------------------------------------------------------------------

/// Pushes the current buffer and cursor state onto the undo stack.
pub fn push_undo(state: &mut EditorState) {
    crate::editor_state::push_undo(state)
}

/// Pushes the current cursor position onto the jump list.
pub fn push_jump(state: &mut EditorState) {
    crate::editor_state::push_jump(state)
}

// ---------------------------------------------------------------------------
// Key resolution
// ---------------------------------------------------------------------------

/// Resolves a key event to a command name using the active keymaps.
pub fn resolve_key(state: &EditorState, key: KeyEvent) -> Option<String> {
    crate::editor_state::resolve_key(state, key)
}
