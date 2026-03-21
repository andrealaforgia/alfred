//! Alfred Lisp -- Lisp interpreter integration for the Alfred text editor.
//!
//! Wraps the `rust_lisp` interpreter, providing a clean eval API
//! for Alfred's plugin system.

pub mod runtime;

/// Indicates the Lisp subsystem is available.
pub fn available() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lisp_subsystem_reports_available() {
        assert!(available());
    }
}
