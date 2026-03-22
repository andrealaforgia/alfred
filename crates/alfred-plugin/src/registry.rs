//! Plugin registry -- tracks loaded plugins and manages their lifecycle.

use std::collections::HashMap;

use alfred_lisp::runtime::LispRuntime;

use crate::error::PluginError;
use crate::metadata::PluginMetadata;

/// Status of a loaded plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginStatus {
    /// Plugin loaded and init.lisp evaluated successfully.
    Active,
}

/// A plugin that has been loaded into the registry.
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub metadata: PluginMetadata,
    pub status: PluginStatus,
}

/// Registry tracking all loaded plugins by name.
pub struct PluginRegistry {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        PluginRegistry {
            plugins: HashMap::new(),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a plugin: evaluate its init.lisp in the Lisp runtime and track it.
///
/// Returns `Err(PluginError::AlreadyLoaded)` if a plugin with the same name
/// is already registered. Returns `Err(PluginError::InitError)` if the
/// init.lisp evaluation fails.
pub fn load_plugin(
    registry: &mut PluginRegistry,
    metadata: PluginMetadata,
    runtime: &LispRuntime,
) -> Result<(), PluginError> {
    if registry.plugins.contains_key(&metadata.name) {
        return Err(PluginError::AlreadyLoaded {
            name: metadata.name.clone(),
        });
    }

    runtime
        .eval_file(&metadata.source_path)
        .map_err(|e| PluginError::InitError {
            name: metadata.name.clone(),
            reason: e.to_string(),
        })?;

    let loaded = LoadedPlugin {
        metadata: metadata.clone(),
        status: PluginStatus::Active,
    };
    registry.plugins.insert(metadata.name, loaded);

    Ok(())
}

/// Remove a plugin from the registry by name.
///
/// Returns `Err(PluginError::NotFound)` if no plugin with that name is loaded.
pub fn unload_plugin(registry: &mut PluginRegistry, name: &str) -> Result<(), PluginError> {
    match registry.plugins.remove(name) {
        Some(_) => Ok(()),
        None => Err(PluginError::NotFound {
            name: name.to_string(),
        }),
    }
}

/// List all currently loaded plugins.
pub fn list_plugins(registry: &PluginRegistry) -> Vec<&PluginMetadata> {
    registry.plugins.values().map(|lp| &lp.metadata).collect()
}
