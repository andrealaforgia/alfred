//! Alfred Lisp -- Lisp interpreter integration for the Alfred text editor.
//!
//! Stub crate. Will be implemented in Milestone 2.

/// Placeholder indicating the Lisp subsystem is available.
pub fn available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lisp_stub_reports_unavailable() {
        assert!(!available());
    }
}
