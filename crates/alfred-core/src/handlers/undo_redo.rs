//! Undo and redo command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and delegates to the core undo/redo logic in editor_state.

use crate::editor_state::{self, EditorState};
use crate::error::Result;

pub fn undo(s: &mut EditorState) -> Result<()> {
    editor_state::undo(s);
    Ok(())
}

pub fn redo(s: &mut EditorState) -> Result<()> {
    editor_state::redo(s);
    Ok(())
}
