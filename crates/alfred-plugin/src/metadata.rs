//! Plugin metadata types.

use std::path::PathBuf;

/// Metadata extracted from a plugin's init.lisp header comments.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub source_path: PathBuf,
}
