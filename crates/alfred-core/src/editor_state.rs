//! EditorState: the top-level aggregation of all editor state.
//!
//! EditorState is the single mutable container passed through the event loop.
//! It aggregates buffer, cursor, viewport, command registry, mode, keymaps,
//! hook registry, message, and running flag.
//! This module has no I/O dependencies -- EditorState is pure state.

use std::collections::HashMap;

use crate::buffer::Buffer;
use crate::command::CommandRegistry;
use crate::cursor::Cursor;
use crate::hook::HookRegistry;
use crate::key_event::KeyEvent;
use crate::viewport::Viewport;

/// A keymap maps key events to command names.
pub type Keymap = HashMap<KeyEvent, String>;

/// Known mode name constants.
pub const MODE_NORMAL: &str = "normal";
pub const MODE_INSERT: &str = "insert";

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
    pub mode: String,
    pub keymaps: HashMap<String, Keymap>,
    pub active_keymaps: Vec<String>,
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
/// - Keymaps registry is empty, active keymaps are empty, hook registry is empty.
/// - Message is None.
/// - Running is true.
/// Resolves a key event by looking it up in the active keymaps.
///
/// Iterates through active keymaps in order, returning the command name
/// from the first keymap that contains a binding for the given key.
/// Returns None if no keymap contains the key.
pub fn resolve_key(state: &EditorState, key: KeyEvent) -> Option<String> {
    for keymap_name in &state.active_keymaps {
        if let Some(keymap) = state.keymaps.get(keymap_name) {
            if let Some(command_name) = keymap.get(&key) {
                return Some(command_name.clone());
            }
        }
    }
    None
}

/// Registers built-in native commands for cursor movement and mode switching.
///
/// These commands are the minimal set needed for keymap-based dispatch:
/// - "cursor-up", "cursor-down", "cursor-left", "cursor-right"
/// - "enter-command-mode" is handled specially by the event loop (not a command)
pub fn register_builtin_commands(state: &mut EditorState) {
    crate::command::register(
        &mut state.commands,
        "cursor-up".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_up(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-down".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_down(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-left".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_left(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-right".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_right(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "delete-backward".to_string(),
        crate::command::CommandHandler::Native(|s| {
            // Delete the character before the cursor (backspace behavior).
            // If cursor is at beginning of buffer, do nothing.
            if s.cursor.line == 0 && s.cursor.column == 0 {
                return Ok(());
            }
            // Move cursor left, then delete character at new position
            s.cursor = crate::cursor::move_left(s.cursor, &s.buffer);
            s.buffer = crate::buffer::delete_at(&s.buffer, s.cursor.line, s.cursor.column);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "delete-char-at-cursor".to_string(),
        crate::command::CommandHandler::Native(|s| {
            // Delete the character at the cursor position (forward delete, vim 'x').
            // If cursor is at end of buffer, do nothing (delete_at handles this).
            s.buffer = crate::buffer::delete_at(&s.buffer, s.cursor.line, s.cursor.column);
            s.cursor = crate::cursor::ensure_within_bounds(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "delete-line".to_string(),
        crate::command::CommandHandler::Native(|s| {
            // Delete the entire current line (vim 'dd' / 'd').
            s.buffer = crate::buffer::delete_line(&s.buffer, s.cursor.line);
            s.cursor = crate::cursor::ensure_within_bounds(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-line-start".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_line_start(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-line-end".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_line_end(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-first-non-blank".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_first_non_blank(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-document-start".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_document_start(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-document-end".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_document_end(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-word-forward".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_word_forward(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-word-backward".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_word_backward(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-word-end".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_word_end(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
}

pub fn new(width: u16, height: u16) -> EditorState {
    EditorState {
        buffer: Buffer::from_string(""),
        cursor: crate::cursor::new(0, 0),
        viewport: crate::viewport::new(0, height, width),
        commands: CommandRegistry::new(),
        mode: MODE_NORMAL.to_string(),
        keymaps: HashMap::new(),
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
        assert_eq!(state.mode, "normal");
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

    // -----------------------------------------------------------------------
    // Unit tests (06-02): resolve_key keymap lookup
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_keymap_with_binding_when_resolve_key_then_returns_command_name() {
        use crate::key_event::{KeyCode, KeyEvent};

        let mut state = editor_state::new(80, 24);
        let mut keymap = crate::editor_state::Keymap::new();
        keymap.insert(KeyEvent::plain(KeyCode::Up), "cursor-up".to_string());
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());

        let result = editor_state::resolve_key(&state, KeyEvent::plain(KeyCode::Up));
        assert_eq!(result, Some("cursor-up".to_string()));
    }

    #[test]
    fn given_keymap_when_unbound_key_then_returns_none() {
        use crate::key_event::{KeyCode, KeyEvent};

        let mut state = editor_state::new(80, 24);
        let keymap = crate::editor_state::Keymap::new(); // empty keymap
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());

        let result = editor_state::resolve_key(&state, KeyEvent::plain(KeyCode::Tab));
        assert_eq!(result, None);
    }

    #[test]
    fn given_multiple_active_keymaps_when_key_in_first_then_first_wins() {
        use crate::key_event::{KeyCode, KeyEvent};

        let mut state = editor_state::new(80, 24);

        // First keymap: Up -> "custom-up"
        let mut keymap1 = crate::editor_state::Keymap::new();
        keymap1.insert(KeyEvent::plain(KeyCode::Up), "custom-up".to_string());
        state.keymaps.insert("overlay".to_string(), keymap1);

        // Second keymap: Up -> "cursor-up"
        let mut keymap2 = crate::editor_state::Keymap::new();
        keymap2.insert(KeyEvent::plain(KeyCode::Up), "cursor-up".to_string());
        state.keymaps.insert("global".to_string(), keymap2);

        // Active keymaps: overlay checked first, then global
        state.active_keymaps.push("overlay".to_string());
        state.active_keymaps.push("global".to_string());

        let result = editor_state::resolve_key(&state, KeyEvent::plain(KeyCode::Up));
        assert_eq!(result, Some("custom-up".to_string()));
    }

    #[test]
    fn given_no_active_keymaps_when_resolve_key_then_returns_none() {
        use crate::key_event::{KeyCode, KeyEvent};

        let state = editor_state::new(80, 24);
        // No keymaps, no active keymaps
        let result = editor_state::resolve_key(&state, KeyEvent::plain(KeyCode::Up));
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // Unit tests (07-02): delete-char-at-cursor and delete-line commands
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_with_text_when_delete_char_at_cursor_executed_then_char_at_cursor_removed() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello");
        state.cursor = crate::cursor::new(0, 1); // cursor at 'e'
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "delete-char-at-cursor");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hllo");
        // Cursor stays at same position after forward-delete
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_cursor_at_end_of_buffer_when_delete_char_at_cursor_executed_then_buffer_unchanged() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hi");
        state.cursor = crate::cursor::new(0, 2); // cursor past last char
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "delete-char-at-cursor");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hi");
    }

    #[test]
    fn given_multiline_buffer_when_delete_line_executed_then_current_line_removed() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("First\nSecond\nThird");
        state.cursor = crate::cursor::new(1, 3); // cursor on "Second"
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "delete-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "First\nThird");
    }

    #[test]
    fn given_single_line_buffer_when_delete_line_executed_then_buffer_becomes_empty() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Only line");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "delete-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "");
    }
}
