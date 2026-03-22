//! Alfred TUI -- terminal UI shell for the Alfred text editor.
//!
//! This crate is the imperative shell. It handles terminal IO via
//! crossterm and rendering via ratatui, delegating all domain logic
//! to alfred-core.

pub mod app;
pub mod renderer;

/// Returns the crate version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
