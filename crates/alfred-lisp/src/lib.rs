//! Alfred Lisp -- Lisp interpreter integration for the Alfred text editor.
//!
//! Wraps the `rust_lisp` interpreter, providing a clean eval API
//! for Alfred's plugin system.

pub mod bridge;
pub mod runtime;

/// Indicates the Lisp subsystem is available.
pub fn available() -> bool {
    true
}
