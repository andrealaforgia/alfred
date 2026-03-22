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

/// Resolve the load order for a set of plugins via topological sort.
///
/// Plugins are sorted so that each plugin's dependencies appear before it.
/// Independent plugins (no dependencies) maintain their input order.
/// Returns an error if a circular or missing dependency is detected.
pub fn resolve_load_order(plugins: &[PluginMetadata]) -> Result<Vec<&PluginMetadata>, PluginError> {
    use std::collections::{HashMap, VecDeque};

    // Build index: name -> position in input slice
    let name_to_idx: HashMap<&str, usize> = plugins
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();

    // Validate all dependencies exist
    for plugin in plugins {
        for dep in &plugin.dependencies {
            if !name_to_idx.contains_key(dep.as_str()) {
                return Err(PluginError::MissingDependency {
                    plugin: plugin.name.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }

    // Build in-degree counts and adjacency list (dependency -> dependents)
    let n = plugins.len();
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, plugin) in plugins.iter().enumerate() {
        in_degree[i] = plugin.dependencies.len();
        for dep in &plugin.dependencies {
            let dep_idx = name_to_idx[dep.as_str()];
            dependents[dep_idx].push(i);
        }
    }

    // Kahn's algorithm: start with in-degree 0 nodes, in input order
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut result: Vec<&PluginMetadata> = Vec::with_capacity(n);

    while let Some(idx) = queue.pop_front() {
        result.push(&plugins[idx]);
        // Sort dependents by their original index to maintain discovery order
        let mut deps = dependents[idx].clone();
        deps.sort();
        for dependent_idx in deps {
            in_degree[dependent_idx] -= 1;
            if in_degree[dependent_idx] == 0 {
                queue.push_back(dependent_idx);
            }
        }
    }

    if result.len() != n {
        // Remaining nodes with in_degree > 0 form the cycle
        let cycle: Vec<String> = (0..n)
            .filter(|&i| in_degree[i] > 0)
            .map(|i| plugins[i].name.clone())
            .collect();
        return Err(PluginError::CircularDependency { cycle });
    }

    Ok(result)
}
