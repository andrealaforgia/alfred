//! Command handlers organized by category.
//!
//! Each sub-module contains named handler functions with the signature
//! `fn(&mut EditorState) -> Result<(), AlfredError>`.
//! These are registered in the BUILTIN_COMMANDS table for data-driven dispatch.

pub mod cursor;
pub mod editing;
pub mod insert_mode;
pub mod navigation;
pub mod search;
pub mod undo_redo;
pub mod visual;
pub mod yank_paste;
