//! Unified error type for the Alfred core domain.
//!
//! All fallible operations in alfred-core return `Result<T, AlfredError>`.
//! Errors are values -- no panics in domain logic.

use std::path::PathBuf;

/// Unified error type for alfred-core operations.
#[derive(Debug, thiserror::Error)]
pub enum AlfredError {
    /// Failed to read a file from disk.
    #[error("failed to read file '{path}': {source}")]
    FileReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Command not found in the registry.
    #[error("command not found: '{name}'")]
    CommandNotFound { name: String },
}

/// Convenience alias used throughout alfred-core.
pub type Result<T> = std::result::Result<T, AlfredError>;
