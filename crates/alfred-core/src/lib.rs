//! Alfred Core -- pure domain layer for the Alfred text editor.
//!
//! This crate contains buffer management, cursor logic, viewport tracking,
//! and all domain types. It has zero runtime dependencies on IO crates
//! (alfred-tui, alfred-lisp, alfred-plugin).

pub mod buffer;
pub mod command;
pub mod cursor;
pub mod editor_state;
pub mod error;
pub mod hook;
pub mod key_event;
pub mod text_object;
pub mod theme;
pub mod viewport;

/// Returns the crate version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
