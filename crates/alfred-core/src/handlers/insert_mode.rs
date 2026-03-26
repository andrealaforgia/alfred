//! Insert mode entry command handlers (I, a, A, o, O).
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and positions the cursor then switches to insert mode.

use crate::editor_state::{self, EditorState, MODE_INSERT};
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn insert_at_line_start(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_first_non_blank(s.cursor, &s.buffer);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn insert_after_cursor(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_right_on_line(s.cursor, &s.buffer);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn insert_at_line_end(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_line_end_for_insert(s.cursor, &s.buffer);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn open_line_below(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    let current_line = s.cursor.line;
    let line_len = buffer::get_line(&s.buffer, current_line)
        .map(|l| l.trim_end_matches('\n').len())
        .unwrap_or(0);
    s.buffer = buffer::insert_at(&s.buffer, current_line, line_len, "\n");
    s.cursor = cursor::new(current_line + 1, 0);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn open_line_above(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    let current_line = s.cursor.line;
    s.buffer = buffer::insert_at(&s.buffer, current_line, 0, "\n");
    s.cursor = cursor::new(current_line, 0);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}
