//! CommandRegistry: named command storage and execution.
//!
//! Commands are registered by name (String) and can be looked up and executed.
//! CommandHandler is an enum supporting Native function pointers and Dynamic closures.
//! This module has no I/O dependencies.

use std::collections::HashMap;
use std::rc::Rc;

use crate::editor_state::EditorState;
use crate::error::Result;

/// Type alias for a dynamic command closure (e.g., Lisp-defined commands).
pub type DynCommandFn = dyn Fn(&mut EditorState) -> Result<()>;

/// A command handler that can operate on EditorState.
///
/// Supports both native function pointers (Rust-defined commands) and
/// dynamic closures (Lisp-defined commands registered via `define-command`).
pub enum CommandHandler {
    /// A native Rust function that mutates editor state.
    Native(fn(&mut EditorState) -> Result<()>),
    /// A dynamic closure (e.g., Lisp callback) that mutates editor state.
    Dynamic(Rc<DynCommandFn>),
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

/// Removes a command handler by name.
///
/// If no command with the given name exists, this is a no-op.
pub fn remove(registry: &mut CommandRegistry, name: &str) {
    registry.commands.remove(name);
}

/// Executes a named command against the given editor state.
///
/// Looks up the command by name in the state's command registry,
/// then invokes the handler. Returns an error if the command is not found.
/// For Native handlers, the fn pointer is copied; for Dynamic handlers,
/// the Rc is cloned -- both release the borrow on state.commands before invocation.
pub fn execute(state: &mut EditorState, name: &str) -> Result<()> {
    let handler = state.commands.commands.get(name).ok_or_else(|| {
        crate::error::AlfredError::CommandNotFound {
            name: name.to_string(),
        }
    })?;
    // Clone/copy the handler to release the borrow on state before calling it
    match handler {
        CommandHandler::Native(f) => {
            let f = *f;
            f(state)
        }
        CommandHandler::Dynamic(f) => {
            let f = Rc::clone(f);
            f(state)
        }
    }
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
    fn given_registered_command_when_removed_then_lookup_returns_none() {
        let mut registry = CommandRegistry::new();
        register(
            &mut registry,
            "ephemeral".to_string(),
            CommandHandler::Native(|_state| Ok(())),
        );
        assert!(lookup(&registry, "ephemeral").is_some());

        remove(&mut registry, "ephemeral");

        assert!(
            lookup(&registry, "ephemeral").is_none(),
            "command should be gone after remove"
        );
    }

    #[test]
    fn given_unknown_command_when_executed_then_returns_error() {
        let mut state = editor_state::new(80, 24);
        let result = execute(&mut state, "unknown");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unit test: Dynamic command handler execution (step 03-05)
    // -----------------------------------------------------------------------

    #[test]
    fn given_dynamic_command_when_executed_then_mutates_state() {
        use std::rc::Rc;
        let mut state = editor_state::new(80, 24);
        register(
            &mut state.commands,
            "dynamic-cmd".to_string(),
            CommandHandler::Dynamic(Rc::new(|s| {
                s.message = Some("dynamic executed".to_string());
                Ok(())
            })),
        );

        let result = execute(&mut state, "dynamic-cmd");
        assert!(result.is_ok());
        assert_eq!(state.message, Some("dynamic executed".to_string()));
    }
}
