//! Plugin registry -- tracks loaded plugins and manages their lifecycle.
//!
//! Each loaded plugin tracks the commands and hooks it registered,
//! enabling clean removal of all plugin resources on unload.

use std::collections::HashMap;

use alfred_core::command::CommandRegistry;
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
    /// Command names registered by this plugin.
    pub registered_commands: Vec<String>,
    /// Hook names registered by this plugin.
    pub registered_hooks: Vec<String>,
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
        registered_commands: Vec::new(),
        registered_hooks: Vec::new(),
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

/// Remove a plugin from the registry, cleaning up its commands and hooks.
///
/// All commands tracked for this plugin are removed from the given
/// `CommandRegistry`. Hook tracking is cleared from the plugin entry.
/// Returns `Err(PluginError::NotFound)` if no plugin with that name is loaded.
pub fn unload_plugin_with_cleanup(
    registry: &mut PluginRegistry,
    name: &str,
    commands: &mut CommandRegistry,
) -> Result<(), PluginError> {
    match registry.plugins.remove(name) {
        Some(plugin) => {
            for cmd_name in &plugin.registered_commands {
                alfred_core::command::remove(commands, cmd_name);
            }
            Ok(())
        }
        None => Err(PluginError::NotFound {
            name: name.to_string(),
        }),
    }
}

/// Track a command as belonging to a plugin.
///
/// Records `cmd_name` in the plugin's `registered_commands` list.
/// If the plugin is not found, this is a no-op.
pub fn track_command(registry: &mut PluginRegistry, plugin_name: &str, cmd_name: &str) {
    if let Some(plugin) = registry.plugins.get_mut(plugin_name) {
        plugin.registered_commands.push(cmd_name.to_string());
    }
}

/// Track a hook as belonging to a plugin.
///
/// Records `hook_name` in the plugin's `registered_hooks` list.
/// If the plugin is not found, this is a no-op.
pub fn track_hook(registry: &mut PluginRegistry, plugin_name: &str, hook_name: &str) {
    if let Some(plugin) = registry.plugins.get_mut(plugin_name) {
        plugin.registered_hooks.push(hook_name.to_string());
    }
}

/// Returns the commands tracked for a plugin, or `None` if plugin not found.
pub fn plugin_commands(registry: &PluginRegistry, plugin_name: &str) -> Option<Vec<String>> {
    registry
        .plugins
        .get(plugin_name)
        .map(|p| p.registered_commands.clone())
}

/// Returns the hooks tracked for a plugin, or `None` if plugin not found.
pub fn plugin_hooks(registry: &PluginRegistry, plugin_name: &str) -> Option<Vec<String>> {
    registry
        .plugins
        .get(plugin_name)
        .map(|p| p.registered_hooks.clone())
}

/// List all currently loaded plugins.
pub fn list_plugins(registry: &PluginRegistry) -> Vec<&PluginMetadata> {
    registry.plugins.values().map(|lp| &lp.metadata).collect()
}
