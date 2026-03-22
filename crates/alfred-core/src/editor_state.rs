//! EditorState: the top-level aggregation of all editor state.
//!
//! EditorState is the single mutable container passed through the event loop.
//! It aggregates buffer, cursor, viewport, command registry, mode, keymaps,
//! hook registry, message, and running flag.
//! This module has no I/O dependencies -- EditorState is pure state.

use crate::buffer::Buffer;
use crate::command::CommandRegistry;
use crate::cursor::Cursor;
use crate::hook::HookRegistry;
use crate::viewport::Viewport;

/// Stub type for keymaps. Full implementation in M6.
pub type Keymap = String;

/// The editor mode, determining how key events are interpreted.
///
/// In M1 only `Normal` mode exists. `Insert` will be added in M7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Normal (command) mode -- the default mode.
    Normal,
}

impl std::fmt::Display for EditorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorMode::Normal => write!(f, "normal"),
        }
    }
}

/// The top-level editor state, aggregating all subsystems.
///
/// This is the single mutable container passed through the event loop.
/// It owns the buffer, cursor, viewport, command registry, mode,
/// active keymaps, hook registry, an optional status message, and the running flag.
pub struct EditorState {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub viewport: Viewport,
    pub commands: CommandRegistry,
    pub mode: EditorMode,
    pub active_keymaps: Vec<Keymap>,
    pub hooks: HookRegistry,
    pub message: Option<String>,
    pub running: bool,
}

/// Creates a new EditorState with default initialization.
///
/// - Buffer is empty (from empty string).
/// - Cursor is at (0, 0).
/// - Viewport fits the given terminal width and height.
/// - Command registry is empty.
/// - Mode is "normal".
/// - Active keymaps are empty, hook registry is empty.
/// - Message is None.
/// - Running is true.
pub fn new(width: u16, height: u16) -> EditorState {
    EditorState {
        buffer: Buffer::from_string(""),
        cursor: crate::cursor::new(0, 0),
        viewport: crate::viewport::new(0, height, width),
        commands: CommandRegistry::new(),
        mode: EditorMode::Normal,
        active_keymaps: Vec::new(),
        hooks: HookRegistry::new(),
        message: None,
        running: true,
    }
}

#[cfg(test)]
mod tests {
    use crate::command;
    use crate::editor_state;

    // -----------------------------------------------------------------------
    // Acceptance test: register a command, execute it, verify state change
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_state_when_command_registered_and_executed_then_state_changes() {
        // Given: an EditorState with default initialization
        let mut state = editor_state::new(80, 24);

        // And: cursor starts at (0, 0), running is true, message is None
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
        assert!(state.running);
        assert!(state.message.is_none());

        // And: a command is registered that sets a message
        command::register(
            &mut state.commands,
            "greet".to_string(),
            command::CommandHandler::Native(|state| {
                state.message = Some("Hello from command!".to_string());
                Ok(())
            }),
        );

        // When: the command is executed
        let result = command::execute(&mut state, "greet");

        // Then: execution succeeds
        assert!(result.is_ok());

        // And: the state has been mutated by the command
        assert_eq!(state.message, Some("Hello from command!".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests: EditorState initialization
    // -----------------------------------------------------------------------

    #[test]
    fn given_new_editor_state_then_cursor_at_origin() {
        let state = editor_state::new(80, 24);
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn given_new_editor_state_then_viewport_matches_terminal_size() {
        let state = editor_state::new(80, 24);
        assert_eq!(state.viewport.width, 80);
        assert_eq!(state.viewport.height, 24);
        assert_eq!(state.viewport.top_line, 0);
    }

    #[test]
    fn given_new_editor_state_then_running_is_true() {
        let state = editor_state::new(80, 24);
        assert!(state.running);
    }

    #[test]
    fn given_new_editor_state_then_message_is_none() {
        let state = editor_state::new(80, 24);
        assert!(state.message.is_none());
    }

    #[test]
    fn given_new_editor_state_then_mode_is_normal() {
        let state = editor_state::new(80, 24);
        assert_eq!(state.mode, crate::editor_state::EditorMode::Normal);
    }

    #[test]
    fn given_new_editor_state_then_command_registry_is_empty() {
        let state = editor_state::new(80, 24);
        assert!(command::lookup(&state.commands, "anything").is_none());
    }

    #[test]
    fn given_new_editor_state_then_active_keymaps_is_empty() {
        let state = editor_state::new(80, 24);
        assert!(state.active_keymaps.is_empty());
    }
}
