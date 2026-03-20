//! Alfred Plugin -- plugin system for the Alfred text editor.
//!
//! Stub crate. Will be implemented in Milestone 3.

/// Placeholder indicating the plugin system is available.
pub fn available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_stub_reports_unavailable() {
        assert!(!available());
    }
}
