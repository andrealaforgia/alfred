//! Visual mode command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and manages visual mode selection operations.

use crate::editor_state::{
    self, advance_cursor_by_one, collect_lines_content, selection_range, EditorState, MODE_INSERT,
    MODE_NORMAL, MODE_VISUAL,
};
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn enter_visual_mode(s: &mut EditorState) -> Result<()> {
    s.selection_start = Some(s.cursor);
    s.visual_line_mode = false;
    s.mode = MODE_VISUAL.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_VISUAL)];
    Ok(())
}

pub fn enter_visual_line_mode(s: &mut EditorState) -> Result<()> {
    s.selection_start = Some(s.cursor);
    s.visual_line_mode = true;
    s.mode = MODE_VISUAL.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_VISUAL)];
    Ok(())
}

pub fn exit_visual_mode(s: &mut EditorState) -> Result<()> {
    s.selection_start = None;
    s.visual_line_mode = false;
    s.mode = MODE_NORMAL.to_string();
    s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
    Ok(())
}

pub fn visual_delete(s: &mut EditorState) -> Result<()> {
    if let Some(anchor) = s.selection_start {
        let (from, to) = selection_range(anchor, s.cursor);
        let reg = s.pending_register.take();
        editor_state::push_undo(s);
        if s.visual_line_mode {
            // Line-wise: delete entire lines from min_line to max_line
            let min_line = from.line;
            let max_line = to.line;
            let yanked = collect_lines_content(&s.buffer, min_line, max_line);
            editor_state::set_register(s, reg, yanked, true);
            let mut buf = s.buffer.clone();
            for _ in min_line..=max_line {
                buf = buffer::delete_line(&buf, min_line);
            }
            s.buffer = buf;
            s.cursor = cursor::ensure_within_bounds(cursor::new(min_line, 0), &s.buffer);
        } else {
            // Character-wise: inclusive selection, extend to by one char
            let to_exclusive = advance_cursor_by_one(to, &s.buffer);
            let text = buffer::get_text_range(
                &s.buffer,
                from.line,
                from.column,
                to_exclusive.line,
                to_exclusive.column,
            );
            editor_state::set_register(s, reg, text, false);
            s.buffer = buffer::delete_char_range(
                &s.buffer,
                from.line,
                from.column,
                to_exclusive.line,
                to_exclusive.column,
            );
            s.cursor = cursor::ensure_within_bounds(from, &s.buffer);
        }
        s.selection_start = None;
        s.visual_line_mode = false;
        s.mode = MODE_NORMAL.to_string();
        s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}

pub fn visual_yank(s: &mut EditorState) -> Result<()> {
    if let Some(anchor) = s.selection_start {
        let (from, to) = selection_range(anchor, s.cursor);
        let reg = s.pending_register.take();
        if s.visual_line_mode {
            // Line-wise: yank entire lines
            let min_line = from.line;
            let max_line = to.line;
            let yanked = collect_lines_content(&s.buffer, min_line, max_line);
            editor_state::set_register(s, reg, yanked, true);
            s.cursor = cursor::new(min_line, 0);
        } else {
            // Character-wise: inclusive selection
            let to_exclusive = advance_cursor_by_one(to, &s.buffer);
            let text = buffer::get_text_range(
                &s.buffer,
                from.line,
                from.column,
                to_exclusive.line,
                to_exclusive.column,
            );
            editor_state::set_register(s, reg, text, false);
            s.cursor = from;
        }
        s.selection_start = None;
        s.visual_line_mode = false;
        s.mode = MODE_NORMAL.to_string();
        s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
        s.message = Some("yanked".to_string());
    }
    Ok(())
}

pub fn visual_change(s: &mut EditorState) -> Result<()> {
    if let Some(anchor) = s.selection_start {
        let (from, to) = selection_range(anchor, s.cursor);
        let reg = s.pending_register.take();
        editor_state::push_undo(s);
        if s.visual_line_mode {
            // Line-wise: delete line contents but leave an empty line, enter insert
            let min_line = from.line;
            let max_line = to.line;
            let yanked = collect_lines_content(&s.buffer, min_line, max_line);
            editor_state::set_register(s, reg, yanked, true);
            // Delete lines from max down to min+1, keeping min_line
            let mut buf = s.buffer.clone();
            for _ in (min_line + 1)..=max_line {
                buf = buffer::delete_line(&buf, min_line + 1);
            }
            // Clear the remaining line's content (replace with empty)
            let line_content = buffer::get_line_content(&buf, min_line);
            if !line_content.is_empty() {
                buf = buffer::delete_char_range(&buf, min_line, 0, min_line, line_content.len());
            }
            s.buffer = buf;
            s.cursor = cursor::new(min_line, 0);
        } else {
            // Character-wise: inclusive selection
            let to_exclusive = advance_cursor_by_one(to, &s.buffer);
            let text = buffer::get_text_range(
                &s.buffer,
                from.line,
                from.column,
                to_exclusive.line,
                to_exclusive.column,
            );
            editor_state::set_register(s, reg, text, false);
            s.buffer = buffer::delete_char_range(
                &s.buffer,
                from.line,
                from.column,
                to_exclusive.line,
                to_exclusive.column,
            );
            s.cursor = cursor::ensure_within_bounds(from, &s.buffer);
        }
        s.selection_start = None;
        s.visual_line_mode = false;
        s.mode = MODE_INSERT.to_string();
        s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}
