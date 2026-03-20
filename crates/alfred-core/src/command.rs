//! CommandRegistry: named command storage and execution.
//!
//! Commands are registered by name (String) and can be looked up and executed.
//! CommandHandler is an enum supporting Native function pointers.
//! This module has no I/O dependencies.

use std::collections::HashMap;

use crate::editor_state::EditorState;
use crate::error::Result;

/// A command handler that can operate on EditorState.
///
/// Currently only supports Native function pointers. The Lisp variant
/// will be added in M2.
pub enum CommandHandler {
    /// A native Rust function that mutates editor state.
    Native(fn(&mut EditorState) -> Result<()>),
}

/// Registry mapping command names to their handlers.
pub struct CommandRegistry {
    commands: HashMap<String, CommandHandler>,
}

impl CommandRegistry {
    /// Creates an empty command registry.
    pub fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
        }
    }

    /// Returns the native function pointer for the named command, if it exists.
    ///
    /// This extracts a `Copy` value, allowing callers to release the borrow
    /// on the registry before invoking the function pointer.
    pub(crate) fn lookup_native_fn(
        &self,
        name: &str,
    ) -> Option<fn(&mut EditorState) -> Result<()>> {
        self.commands.get(name).map(|CommandHandler::Native(f)| *f)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registers a command handler under the given name.
///
/// If a command with the same name already exists, it is replaced.
pub fn register(registry: &mut CommandRegistry, name: String, handler: CommandHandler) {
    registry.commands.insert(name, handler);
}

/// Looks up a command handler by name.
///
/// Returns a reference to the handler if found, or None.
pub fn lookup<'a>(registry: &'a CommandRegistry, name: &str) -> Option<&'a CommandHandler> {
    registry.commands.get(name)
}

/// Executes a named command against the given editor state.
///
/// Looks up the command by name in the state's command registry,
/// then invokes the handler. Returns an error if the command is not found.
pub fn execute(state: &mut EditorState, name: &str) -> Result<()> {
    // Extract the Copy function pointer first to release the borrow on state.commands
    let handler_fn = state.commands.lookup_native_fn(name).ok_or_else(|| {
        crate::error::AlfredError::CommandNotFound {
            name: name.to_string(),
        }
    })?;
    handler_fn(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_state;

    // -----------------------------------------------------------------------
    // Unit tests: CommandRegistry register and lookup
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_registry_when_lookup_then_returns_none() {
        let registry = CommandRegistry::new();
        assert!(lookup(&registry, "nonexistent").is_none());
    }

    #[test]
    fn given_registered_command_when_lookup_by_name_then_returns_some() {
        let mut registry = CommandRegistry::new();
        register(
            &mut registry,
            "test-cmd".to_string(),
            CommandHandler::Native(|_state| Ok(())),
        );
        assert!(lookup(&registry, "test-cmd").is_some());
    }

    #[test]
    fn given_registered_command_when_lookup_wrong_name_then_returns_none() {
        let mut registry = CommandRegistry::new();
        register(
            &mut registry,
            "test-cmd".to_string(),
            CommandHandler::Native(|_state| Ok(())),
        );
        assert!(lookup(&registry, "other-cmd").is_none());
    }

    // -----------------------------------------------------------------------
    // Unit tests: CommandRegistry execute
    // -----------------------------------------------------------------------

    #[test]
    fn given_registered_command_when_executed_then_mutates_state() {
        let mut state = editor_state::new(80, 24);
        register(
            &mut state.commands,
            "set-msg".to_string(),
            CommandHandler::Native(|s| {
                s.message = Some("executed".to_string());
                Ok(())
            }),
        );

        let result = execute(&mut state, "set-msg");
        assert!(result.is_ok());
        assert_eq!(state.message, Some("executed".to_string()));
    }

    #[test]
    fn given_unknown_command_when_executed_then_returns_error() {
        let mut state = editor_state::new(80, 24);
        let result = execute(&mut state, "unknown");
        assert!(result.is_err());
    }
}
