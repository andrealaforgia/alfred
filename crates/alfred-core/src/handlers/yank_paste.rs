//! Yank and paste command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and handles register-aware yank/paste operations.

use crate::editor_state::{self, EditorState};
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn yank_line(s: &mut EditorState) -> Result<()> {
    let content = buffer::get_line_content(&s.buffer, s.cursor.line);
    let reg = s.pending_register.take();
    editor_state::set_register(s, reg, content, true);
    Ok(())
}

pub fn paste_below(s: &mut EditorState) -> Result<()> {
    let reg = s.pending_register.take();
    if let Some((text, linewise)) = editor_state::get_yank_content(s, reg) {
        editor_state::push_undo(s);
        if linewise {
            // Line-wise paste: insert on a new line below
            let current_line = s.cursor.line;
            let line_len = buffer::get_line(&s.buffer, current_line)
                .map(|l| l.trim_end_matches('\n').len())
                .unwrap_or(0);
            s.buffer = buffer::insert_at(&s.buffer, current_line, line_len, "\n");
            s.buffer = buffer::insert_at(&s.buffer, current_line + 1, 0, &text);
            s.cursor = cursor::new(current_line + 1, 0);
        } else {
            // Character-wise paste: insert after cursor position
            let col = s.cursor.column + 1;
            s.buffer = buffer::insert_at(&s.buffer, s.cursor.line, col, &text);
            // Cursor moves to end of pasted text - 1 (on last pasted char)
            let end_col = col + text.len().saturating_sub(1);
            s.cursor = cursor::new(s.cursor.line, end_col);
        }
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}

pub fn paste_before(s: &mut EditorState) -> Result<()> {
    let reg = s.pending_register.take();
    if let Some((text, linewise)) = editor_state::get_yank_content(s, reg) {
        editor_state::push_undo(s);
        if linewise {
            // Line-wise paste: insert on a new line above
            let current_line = s.cursor.line;
            s.buffer = buffer::insert_at(&s.buffer, current_line, 0, &format!("{}\n", text));
            s.cursor = cursor::new(current_line, 0);
        } else {
            // Character-wise paste: insert before cursor position
            let col = s.cursor.column;
            s.buffer = buffer::insert_at(&s.buffer, s.cursor.line, col, &text);
            // Cursor moves to end of pasted text - 1 (on last pasted char)
            let end_col = col + text.len().saturating_sub(1);
            s.cursor = cursor::new(s.cursor.line, end_col);
        }
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}
