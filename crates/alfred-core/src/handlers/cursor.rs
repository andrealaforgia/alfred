//! Cursor movement command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and performs a single cursor movement, adjusting the viewport afterward.

use crate::editor_state::EditorState;
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn cursor_up(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_up(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_down(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_down(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_left(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_left(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_right(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_right(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_line_start(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_line_start(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_line_end(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_line_end(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_first_non_blank(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_first_non_blank(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_document_start(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_document_start(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_document_end(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_to_document_end(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_word_forward(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_word_forward(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_word_backward(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_word_backward(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_word_end(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::move_word_end(s.cursor, &s.buffer);
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn cursor_screen_top(s: &mut EditorState) -> Result<()> {
    s.cursor = cursor::new(s.viewport.top_line, 0);
    Ok(())
}

pub fn cursor_screen_middle(s: &mut EditorState) -> Result<()> {
    let middle_line = s.viewport.top_line + (s.viewport.height as usize) / 2;
    let last_line = buffer::line_count(&s.buffer).saturating_sub(1);
    s.cursor = cursor::new(middle_line.min(last_line), 0);
    Ok(())
}

pub fn cursor_screen_bottom(s: &mut EditorState) -> Result<()> {
    let screen_bottom = s.viewport.top_line + s.viewport.height as usize - 1;
    let last_line = buffer::line_count(&s.buffer).saturating_sub(1);
    s.cursor = cursor::new(screen_bottom.min(last_line), 0);
    Ok(())
}

pub fn scroll_half_page_down(s: &mut EditorState) -> Result<()> {
    let half_page = (s.viewport.height as usize) / 2;
    let last_line = buffer::line_count(&s.buffer).saturating_sub(1);
    let new_cursor_line = (s.cursor.line + half_page).min(last_line);
    let new_top_line = (s.viewport.top_line + half_page).min(last_line);
    s.cursor = cursor::new(new_cursor_line, 0);
    s.viewport = viewport::Viewport {
        top_line: new_top_line,
        ..s.viewport
    };
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}

pub fn scroll_half_page_up(s: &mut EditorState) -> Result<()> {
    let half_page = (s.viewport.height as usize) / 2;
    let new_cursor_line = s.cursor.line.saturating_sub(half_page);
    let new_top_line = s.viewport.top_line.saturating_sub(half_page);
    s.cursor = cursor::new(new_cursor_line, 0);
    s.viewport = viewport::Viewport {
        top_line: new_top_line,
        ..s.viewport
    };
    s.viewport = viewport::adjust(s.viewport, &s.cursor);
    Ok(())
}
