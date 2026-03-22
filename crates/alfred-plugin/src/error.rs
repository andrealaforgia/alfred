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
    /// Plugin with this name is already loaded.
    AlreadyLoaded { name: String },
    /// Plugin not found in registry.
    NotFound { name: String },
    /// Plugin init.lisp evaluation failed.
    InitError { name: String, reason: String },
    /// Circular dependency detected among plugins.
    CircularDependency { cycle: Vec<String> },
    /// Plugin declares a dependency that is not available.
    MissingDependency { plugin: String, dependency: String },
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
            PluginError::AlreadyLoaded { name } => {
                write!(f, "plugin already loaded: {}", name)
            }
            PluginError::NotFound { name } => {
                write!(f, "plugin not found: {}", name)
            }
            PluginError::InitError { name, reason } => {
                write!(f, "init error for plugin {}: {}", name, reason)
            }
            PluginError::CircularDependency { cycle } => {
                write!(f, "circular dependency: {}", cycle.join(" -> "))
            }
            PluginError::MissingDependency { plugin, dependency } => {
                write!(
                    f,
                    "plugin '{}' depends on '{}' which is not available",
                    plugin, dependency
                )
            }
        }
    }
}

impl std::error::Error for PluginError {}
