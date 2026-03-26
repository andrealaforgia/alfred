//! Search and navigation command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and performs search, character find, or bracket matching operations.

use crate::command;
use crate::editor_state::{self, EditorState};
use crate::error::Result;
use crate::{buffer, cursor, viewport};

pub fn search_next(s: &mut EditorState) -> Result<()> {
    if let Some(ref pattern) = s.search_pattern.clone() {
        let found = if s.search_forward {
            buffer::find_forward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
        } else {
            buffer::find_backward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
        };
        match found {
            Some((line, col)) => {
                s.cursor = cursor::new(line, col);
                s.viewport = viewport::adjust(s.viewport, &s.cursor);
                s.message = None;
            }
            None => {
                s.message = Some(format!("Pattern not found: {}", pattern));
            }
        }
    } else {
        s.message = Some("No previous search pattern".to_string());
    }
    Ok(())
}

pub fn search_prev(s: &mut EditorState) -> Result<()> {
    if let Some(ref pattern) = s.search_pattern.clone() {
        // search-prev is the opposite direction of the last search
        let found = if s.search_forward {
            buffer::find_backward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
        } else {
            buffer::find_forward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
        };
        match found {
            Some((line, col)) => {
                s.cursor = cursor::new(line, col);
                s.viewport = viewport::adjust(s.viewport, &s.cursor);
                s.message = None;
            }
            None => {
                s.message = Some(format!("Pattern not found: {}", pattern));
            }
        }
    } else {
        s.message = Some("No previous search pattern".to_string());
    }
    Ok(())
}

pub fn repeat_char_find(s: &mut EditorState) -> Result<()> {
    if let Some((kind, ch)) = s.last_char_find {
        if let Some(new_cursor) = editor_state::execute_char_find(s.cursor, &s.buffer, kind, ch) {
            s.cursor = new_cursor;
            s.viewport = viewport::adjust(s.viewport, &s.cursor);
        }
    }
    Ok(())
}

pub fn reverse_char_find(s: &mut EditorState) -> Result<()> {
    if let Some((kind, ch)) = s.last_char_find {
        let reversed_kind = editor_state::reverse_char_find_kind(kind);
        if let Some(new_cursor) =
            editor_state::execute_char_find(s.cursor, &s.buffer, reversed_kind, ch)
        {
            s.cursor = new_cursor;
            s.viewport = viewport::adjust(s.viewport, &s.cursor);
        }
    }
    Ok(())
}

pub fn repeat_last_change(s: &mut EditorState) -> Result<()> {
    if let Some(cmd_name) = s.last_edit_command.clone() {
        command::execute(s, &cmd_name)?;
    }
    Ok(())
}

pub fn match_bracket(s: &mut EditorState) -> Result<()> {
    if let Some(new_cursor) = cursor::find_matching_bracket(s.cursor, &s.buffer) {
        s.cursor = new_cursor;
        s.viewport = viewport::adjust(s.viewport, &s.cursor);
    }
    Ok(())
}
