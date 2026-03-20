//! Alfred Core -- pure domain layer for the Alfred text editor.
//!
//! This crate contains buffer management, cursor logic, viewport tracking,
//! and all domain types. It has zero runtime dependencies on IO crates
//! (alfred-tui, alfred-lisp, alfred-plugin).

/// Returns the crate version string.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }
}
