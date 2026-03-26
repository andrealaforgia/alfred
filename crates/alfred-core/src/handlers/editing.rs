//! Basic editing command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and performs a single editing operation (delete, indent, toggle-case, etc.).

use crate::editor_state::{self, EditorState, MODE_INSERT};
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn delete_backward(s: &mut EditorState) -> Result<()> {
    if s.cursor.line == 0 && s.cursor.column == 0 {
        return Ok(());
    }
    editor_state::push_undo(s);
    s.cursor = cursor::move_left(s.cursor, &s.buffer);
    s.buffer = buffer::delete_at(&s.buffer, s.cursor.line, s.cursor.column);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn delete_char_at_cursor(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::delete_at(&s.buffer, s.cursor.line, s.cursor.column);
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn delete_line(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::delete_line(&s.buffer, s.cursor.line);
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn delete_to_end(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::delete_to_line_end(&s.buffer, s.cursor.line, s.cursor.column);
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn delete_char_before(s: &mut EditorState) -> Result<()> {
    if s.cursor.column == 0 {
        return Ok(());
    }
    editor_state::push_undo(s);
    let new_col = s.cursor.column - 1;
    s.buffer = buffer::delete_at(&s.buffer, s.cursor.line, new_col);
    s.cursor = cursor::new(s.cursor.line, new_col);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn join_lines(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::join_lines(&s.buffer, s.cursor.line);
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn indent_line(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::indent_line(&s.buffer, s.cursor.line, "    ");
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn unindent_line(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::unindent_line(&s.buffer, s.cursor.line, 4);
    s.cursor = cursor::ensure_within_bounds(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn toggle_case(s: &mut EditorState) -> Result<()> {
    let line_content = buffer::get_line_content(&s.buffer, s.cursor.line);
    if line_content.is_empty() {
        return Ok(());
    }
    if s.cursor.column >= line_content.len() {
        return Ok(());
    }
    editor_state::push_undo(s);
    s.buffer = buffer::toggle_case_at(&s.buffer, s.cursor.line, s.cursor.column);
    // Advance cursor right (within line, like vim ~)
    let new_line_content = buffer::get_line_content(&s.buffer, s.cursor.line);
    let max_col = new_line_content.len().saturating_sub(1);
    if s.cursor.column < max_col {
        s.cursor = cursor::new(s.cursor.line, s.cursor.column + 1);
    }
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn increment_number(s: &mut EditorState) -> Result<()> {
    if let Some((start, end, value)) =
        buffer::find_number_at_cursor(&s.buffer, s.cursor.line, s.cursor.column)
    {
        editor_state::push_undo(s);
        let new_value = value.saturating_add(1);
        s.buffer = buffer::replace_number_in_line(&s.buffer, s.cursor.line, start, end, new_value);
        // Position cursor on the last digit of the new number
        let new_num_str = new_value.to_string();
        let new_end = start + new_num_str.len();
        s.cursor = cursor::new(s.cursor.line, new_end.saturating_sub(1));
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}

pub fn decrement_number(s: &mut EditorState) -> Result<()> {
    if let Some((start, end, value)) =
        buffer::find_number_at_cursor(&s.buffer, s.cursor.line, s.cursor.column)
    {
        editor_state::push_undo(s);
        let new_value = value.saturating_sub(1);
        s.buffer = buffer::replace_number_in_line(&s.buffer, s.cursor.line, start, end, new_value);
        // Position cursor on the last digit of the new number
        let new_num_str = new_value.to_string();
        let new_end = start + new_num_str.len();
        s.cursor = cursor::new(s.cursor.line, new_end.saturating_sub(1));
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}

pub fn substitute_line(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::replace_line(&s.buffer, s.cursor.line, "");
    s.cursor = cursor::new(s.cursor.line, 0);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn substitute_char(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    let line_content = buffer::get_line_content(&s.buffer, s.cursor.line);
    if s.cursor.column < line_content.len() {
        s.buffer = buffer::delete_at(&s.buffer, s.cursor.line, s.cursor.column);
    }
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn replace_char_at_cursor(_s: &mut EditorState) -> Result<()> {
    // This is a no-op placeholder -- actual replace is handled by PendingReplace
    // in the TUI event loop which calls buffer::replace_char_at directly.
    Ok(())
}

pub fn change_line(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::replace_line(&s.buffer, s.cursor.line, "");
    s.cursor = cursor::new(s.cursor.line, 0);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn change_to_end(s: &mut EditorState) -> Result<()> {
    editor_state::push_undo(s);
    s.buffer = buffer::delete_to_line_end(&s.buffer, s.cursor.line, s.cursor.column);
    s.mode = MODE_INSERT.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}
