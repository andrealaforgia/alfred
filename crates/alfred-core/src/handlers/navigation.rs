//! Jump list and change list navigation command handlers.
//!
//! Each function has the signature `fn(&mut EditorState) -> Result<()>`
//! and delegates to the core jump/change list logic in editor_state.

use crate::editor_state::{self, EditorState};
use crate::error::Result;

pub fn jump_back(s: &mut EditorState) -> Result<()> {
    editor_state::jump_back(s);
    Ok(())
}

pub fn jump_forward(s: &mut EditorState) -> Result<()> {
    editor_state::jump_forward(s);
    Ok(())
}

pub fn change_list_back(s: &mut EditorState) -> Result<()> {
    editor_state::change_list_back(s);
    Ok(())
}

pub fn change_list_forward(s: &mut EditorState) -> Result<()> {
    editor_state::change_list_forward(s);
    Ok(())
}
