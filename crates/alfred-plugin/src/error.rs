//! Plugin error types.

use std::path::PathBuf;

/// Errors that can occur during plugin discovery and loading.
#[derive(Debug)]
pub enum PluginError {
    /// Plugin directory does not contain init.lisp.
    MissingInitFile { path: PathBuf },
    /// Failed to read init.lisp.
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to parse metadata from init.lisp.
    ParseError { path: PathBuf, reason: String },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::MissingInitFile { path } => {
                write!(f, "missing init.lisp in {}", path.display())
            }
            PluginError::ReadError { path, source } => {
                write!(f, "failed to read {}: {}", path.display(), source)
            }
            PluginError::ParseError { path, reason } => {
                write!(f, "parse error in {}: {}", path.display(), reason)
            }
        }
    }
}

impl std::error::Error for PluginError {}
