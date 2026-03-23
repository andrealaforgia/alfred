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
use crate::theme::Theme;
use crate::viewport::Viewport;

/// A keymap maps key events to command names.
pub type Keymap = HashMap<KeyEvent, String>;

/// Known mode name constants.
pub const MODE_NORMAL: &str = "normal";
pub const MODE_INSERT: &str = "insert";

/// A snapshot of buffer and cursor state for undo/redo.
///
/// Rope cloning is O(1) due to structural sharing, making
/// whole-buffer snapshots cheap.
#[derive(Debug, Clone)]
pub struct UndoSnapshot {
    pub buffer: Buffer,
    pub cursor: Cursor,
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
    pub mode: String,
    pub keymaps: HashMap<String, Keymap>,
    pub active_keymaps: Vec<String>,
    pub hooks: HookRegistry,
    pub message: Option<String>,
    pub running: bool,
    pub yank_register: Option<String>,
    pub undo_stack: Vec<UndoSnapshot>,
    pub redo_stack: Vec<UndoSnapshot>,
    pub theme: Theme,
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
///
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
    // Insert mode variant commands (vim I, a, A, o, O)
    crate::command::register(
        &mut state.commands,
        "insert-at-line-start".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_first_non_blank(s.cursor, &s.buffer);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "insert-after-cursor".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_right_on_line(s.cursor, &s.buffer);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "insert-at-line-end".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::move_to_line_end_for_insert(s.cursor, &s.buffer);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "open-line-below".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let current_line = s.cursor.line;
            let line_len = crate::buffer::get_line(&s.buffer, current_line)
                .map(|l| l.trim_end_matches('\n').len())
                .unwrap_or(0);
            s.buffer = crate::buffer::insert_at(&s.buffer, current_line, line_len, "\n");
            s.cursor = crate::cursor::new(current_line + 1, 0);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "open-line-above".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let current_line = s.cursor.line;
            s.buffer = crate::buffer::insert_at(&s.buffer, current_line, 0, "\n");
            s.cursor = crate::cursor::new(current_line, 0);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    // --- 09-03: Join, yank, paste, change, undo, redo commands ---
    crate::command::register(
        &mut state.commands,
        "join-lines".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            s.buffer = crate::buffer::join_lines(&s.buffer, s.cursor.line);
            s.cursor = crate::cursor::ensure_within_bounds(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "yank-line".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let content = crate::buffer::get_line_content(&s.buffer, s.cursor.line);
            s.yank_register = Some(content);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "paste-below".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(ref text) = s.yank_register.clone() {
                push_undo(s);
                let current_line = s.cursor.line;
                let line_len = crate::buffer::get_line(&s.buffer, current_line)
                    .map(|l| l.trim_end_matches('\n').len())
                    .unwrap_or(0);
                // Insert a newline at end of current line, then the yanked text
                s.buffer = crate::buffer::insert_at(&s.buffer, current_line, line_len, "\n");
                s.buffer = crate::buffer::insert_at(&s.buffer, current_line + 1, 0, text);
                s.cursor = crate::cursor::new(current_line + 1, 0);
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "change-line".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            s.buffer = crate::buffer::replace_line(&s.buffer, s.cursor.line, "");
            s.cursor = crate::cursor::new(s.cursor.line, 0);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "change-to-end".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            s.buffer = crate::buffer::delete_to_line_end(&s.buffer, s.cursor.line, s.cursor.column);
            s.mode = MODE_INSERT.to_string();
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "undo".to_string(),
        crate::command::CommandHandler::Native(|s| {
            undo(s);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "redo".to_string(),
        crate::command::CommandHandler::Native(|s| {
            redo(s);
            Ok(())
        }),
    );
    // --- 09-04: Screen-relative cursor and half-page scroll commands ---
    crate::command::register(
        &mut state.commands,
        "cursor-screen-top".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.cursor = crate::cursor::new(s.viewport.top_line, 0);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-screen-middle".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let middle_line = s.viewport.top_line + (s.viewport.height as usize) / 2;
            let last_line = crate::buffer::line_count(&s.buffer).saturating_sub(1);
            s.cursor = crate::cursor::new(middle_line.min(last_line), 0);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "cursor-screen-bottom".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let screen_bottom = s.viewport.top_line + s.viewport.height as usize - 1;
            let last_line = crate::buffer::line_count(&s.buffer).saturating_sub(1);
            s.cursor = crate::cursor::new(screen_bottom.min(last_line), 0);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "scroll-half-page-down".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let half_page = (s.viewport.height as usize) / 2;
            let last_line = crate::buffer::line_count(&s.buffer).saturating_sub(1);
            let new_cursor_line = (s.cursor.line + half_page).min(last_line);
            let new_top_line = (s.viewport.top_line + half_page).min(last_line);
            s.cursor = crate::cursor::new(new_cursor_line, 0);
            s.viewport = crate::viewport::Viewport {
                top_line: new_top_line,
                ..s.viewport
            };
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "scroll-half-page-up".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let half_page = (s.viewport.height as usize) / 2;
            let new_cursor_line = s.cursor.line.saturating_sub(half_page);
            let new_top_line = s.viewport.top_line.saturating_sub(half_page);
            s.cursor = crate::cursor::new(new_cursor_line, 0);
            s.viewport = crate::viewport::Viewport {
                top_line: new_top_line,
                ..s.viewport
            };
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
}

/// Saves a snapshot of the current buffer and cursor onto the undo stack.
///
/// Clears the redo stack (any redo history is lost when a new edit is made).
/// Call this before any buffer mutation to enable undo.
pub fn push_undo(state: &mut EditorState) {
    state.undo_stack.push(UndoSnapshot {
        buffer: state.buffer.clone(),
        cursor: state.cursor,
    });
    state.redo_stack.clear();
}

/// Undoes the last change by popping the undo stack.
///
/// Pushes the current state onto the redo stack before restoring.
/// If the undo stack is empty, the state is unchanged.
pub fn undo(state: &mut EditorState) {
    if let Some(snapshot) = state.undo_stack.pop() {
        state.redo_stack.push(UndoSnapshot {
            buffer: state.buffer.clone(),
            cursor: state.cursor,
        });
        state.buffer = snapshot.buffer;
        state.cursor = snapshot.cursor;
        state.viewport = crate::viewport::adjust(state.viewport, &state.cursor);
    }
}

/// Redoes the last undone change by popping the redo stack.
///
/// Pushes the current state onto the undo stack before restoring.
/// If the redo stack is empty, the state is unchanged.
pub fn redo(state: &mut EditorState) {
    if let Some(snapshot) = state.redo_stack.pop() {
        state.undo_stack.push(UndoSnapshot {
            buffer: state.buffer.clone(),
            cursor: state.cursor,
        });
        state.buffer = snapshot.buffer;
        state.cursor = snapshot.cursor;
        state.viewport = crate::viewport::adjust(state.viewport, &state.cursor);
    }
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
        yank_register: None,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        theme: crate::theme::new_theme(),
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
    // Unit test: EditorState initialization (all default properties)
    // -----------------------------------------------------------------------

    #[test]
    fn given_new_editor_state_then_all_defaults_are_correct() {
        let state = editor_state::new(80, 24);

        // Cursor at origin
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);

        // Viewport matches terminal size
        assert_eq!(state.viewport.width, 80);
        assert_eq!(state.viewport.height, 24);
        assert_eq!(state.viewport.top_line, 0);

        // Running flag
        assert!(state.running);

        // No message
        assert!(state.message.is_none());

        // Normal mode
        assert_eq!(state.mode, "normal");

        // Empty command registry
        assert!(command::lookup(&state.commands, "anything").is_none());

        // Empty active keymaps
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

    // -----------------------------------------------------------------------
    // Acceptance test (09-02): open-line-below inserts new line and enters insert mode
    // -----------------------------------------------------------------------

    #[test]
    fn given_normal_mode_when_open_line_below_then_new_line_inserted_and_mode_is_insert() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("First\nSecond\nThird");
        state.cursor = crate::cursor::new(0, 2); // cursor on "First", column 2
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        // When: open-line-below is executed
        let result = command::execute(&mut state, "open-line-below");

        // Then: execution succeeds
        assert!(result.is_ok());

        // And: a new empty line is inserted below current line
        assert_eq!(buffer::content(&state.buffer), "First\n\nSecond\nThird");

        // And: cursor is on the new line at column 0
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);

        // And: mode is insert
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-02): insert mode variant commands
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_with_leading_whitespace_when_insert_at_line_start_then_cursor_at_first_non_blank_and_insert_mode(
    ) {
        let mut state = editor_state::new(80, 24);
        state.buffer = crate::buffer::Buffer::from_string("   hello world");
        state.cursor = crate::cursor::new(0, 8); // cursor in middle
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "insert-at-line-start");
        assert!(result.is_ok());
        assert_eq!(state.cursor.column, 3); // first non-blank
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_cursor_in_middle_of_line_when_insert_after_cursor_then_cursor_moves_right_and_insert_mode(
    ) {
        let mut state = editor_state::new(80, 24);
        state.buffer = crate::buffer::Buffer::from_string("hello");
        state.cursor = crate::cursor::new(0, 2); // cursor at 'l'
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "insert-after-cursor");
        assert!(result.is_ok());
        assert_eq!(state.cursor.column, 3); // moved right by 1
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_cursor_at_end_of_line_when_insert_after_cursor_then_cursor_stays_at_end_and_insert_mode(
    ) {
        let mut state = editor_state::new(80, 24);
        state.buffer = crate::buffer::Buffer::from_string("hi\nworld");
        state.cursor = crate::cursor::new(0, 2); // cursor at end of "hi" (line_length)
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "insert-after-cursor");
        assert!(result.is_ok());
        // Should NOT wrap to next line; should stay at end of current line
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 2);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_cursor_anywhere_when_insert_at_line_end_then_cursor_at_end_of_line_and_insert_mode() {
        let mut state = editor_state::new(80, 24);
        state.buffer = crate::buffer::Buffer::from_string("hello world");
        state.cursor = crate::cursor::new(0, 2); // cursor at 'l'
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "insert-at-line-end");
        assert!(result.is_ok());
        // "hello world" has 11 chars, insert position is at column 11 (past last char)
        assert_eq!(state.cursor.column, 11);
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_normal_mode_when_open_line_above_then_new_line_inserted_above_and_mode_is_insert() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("First\nSecond\nThird");
        state.cursor = crate::cursor::new(1, 3); // cursor on "Second"
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "open-line-above");
        assert!(result.is_ok());

        // New empty line inserted above "Second"
        assert_eq!(buffer::content(&state.buffer), "First\n\nSecond\nThird");

        // Cursor is on the new empty line (line 1) at column 0
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);

        // Mode is insert
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_first_line_when_open_line_above_then_new_line_at_top_and_cursor_on_it() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Only line");
        state.cursor = crate::cursor::new(0, 4);
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "open-line-above");
        assert!(result.is_ok());

        // New empty line inserted above, original line pushed down
        assert_eq!(buffer::content(&state.buffer), "\nOnly line");

        // Cursor is on the new empty line (line 0) at column 0
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (09-03): yank line then paste below duplicates the line
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_when_yank_line_and_paste_below_then_line_is_duplicated() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("First\nSecond\nThird");
        state.cursor = crate::cursor::new(1, 0); // cursor on "Second"
        editor_state::register_builtin_commands(&mut state);

        // When: yank the current line, then paste below
        let result = command::execute(&mut state, "yank-line");
        assert!(result.is_ok());
        let result = command::execute(&mut state, "paste-below");
        assert!(result.is_ok());

        // Then: "Second" is duplicated below
        assert_eq!(
            buffer::content(&state.buffer),
            "First\nSecond\nSecond\nThird"
        );

        // And: cursor is on the pasted line
        assert_eq!(state.cursor.line, 2);
        assert_eq!(state.cursor.column, 0);
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-03): join-lines, change-line, change-to-end, undo, redo
    // Test Budget: 7 behaviors x 2 = 14 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_join_lines_then_current_and_next_merged_with_space() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello\nWorld\nEnd");
        state.cursor = crate::cursor::new(0, 3);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "join-lines");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello World\nEnd");
    }

    #[test]
    fn given_last_line_when_join_lines_then_buffer_unchanged() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Only");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "join-lines");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Only");
    }

    #[test]
    fn given_line_with_text_when_change_line_then_line_cleared_and_insert_mode() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello\nWorld");
        state.cursor = crate::cursor::new(0, 3);
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "change-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "\nWorld");
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_cursor_in_middle_when_change_to_end_then_text_after_cursor_deleted_and_insert_mode() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello World\nSecond");
        state.cursor = crate::cursor::new(0, 5);
        state.mode = editor_state::MODE_NORMAL.to_string();
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "change-to-end");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello\nSecond");
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 5);
        assert_eq!(state.mode, editor_state::MODE_INSERT);
    }

    #[test]
    fn given_buffer_modified_when_undo_then_buffer_restored_to_previous_state() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Original");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Modify: join-lines pushes undo before mutation
        state.buffer = buffer::Buffer::from_string("Hello\nWorld");
        state.cursor = crate::cursor::new(0, 0);
        let result = command::execute(&mut state, "join-lines");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello World");

        // When: undo
        let result = command::execute(&mut state, "undo");
        assert!(result.is_ok());

        // Then: buffer is restored
        assert_eq!(buffer::content(&state.buffer), "Hello\nWorld");
    }

    #[test]
    fn given_undone_change_when_redo_then_change_reapplied() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello\nWorld");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Mutate: join lines
        let result = command::execute(&mut state, "join-lines");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello World");

        // Undo
        let result = command::execute(&mut state, "undo");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello\nWorld");

        // When: redo
        let result = command::execute(&mut state, "redo");
        assert!(result.is_ok());

        // Then: change reapplied
        assert_eq!(buffer::content(&state.buffer), "Hello World");
    }

    #[test]
    fn given_no_history_when_undo_or_redo_then_buffer_unchanged() {
        use crate::buffer;

        // (command_name, label)
        let cases: Vec<(&str, &str)> =
            vec![("undo", "no undo history"), ("redo", "no redo history")];

        for (command_name, label) in &cases {
            let mut state = editor_state::new(80, 24);
            state.buffer = buffer::Buffer::from_string("Unchanged");
            state.cursor = crate::cursor::new(0, 0);
            editor_state::register_builtin_commands(&mut state);

            let result = command::execute(&mut state, command_name);
            assert!(result.is_ok(), "{}: should succeed", label);
            assert_eq!(
                buffer::content(&state.buffer),
                "Unchanged",
                "{}: buffer should be unchanged",
                label
            );
        }
    }

    #[test]
    fn given_paste_without_yank_when_paste_below_then_buffer_unchanged() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "paste-below");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello");
    }

    // -----------------------------------------------------------------------
    // Acceptance test (09-04): H moves cursor to screen top, M to middle, L to bottom
    // -----------------------------------------------------------------------

    #[test]
    fn given_scrolled_viewport_when_h_m_l_then_cursor_moves_to_screen_top_middle_bottom() {
        let mut state = editor_state::new(80, 24);
        // Create a 50-line buffer
        let lines: Vec<&str> = (0..50).map(|_| "line content").collect();
        state.buffer = crate::buffer::Buffer::from_string(&lines.join("\n"));
        // Viewport showing lines 10..33 (top_line=10, height=24)
        state.viewport = crate::viewport::new(10, 24, 80);
        state.cursor = crate::cursor::new(20, 5); // cursor somewhere in middle
        editor_state::register_builtin_commands(&mut state);

        // When: cursor-screen-top (H)
        let result = command::execute(&mut state, "cursor-screen-top");
        assert!(result.is_ok());
        assert_eq!(state.cursor.line, 10); // top of viewport
        assert_eq!(state.cursor.column, 0);

        // When: cursor-screen-middle (M)
        let result = command::execute(&mut state, "cursor-screen-middle");
        assert!(result.is_ok());
        assert_eq!(state.cursor.line, 22); // 10 + 24/2 = 22
        assert_eq!(state.cursor.column, 0);

        // When: cursor-screen-bottom (L)
        let result = command::execute(&mut state, "cursor-screen-bottom");
        assert!(result.is_ok());
        assert_eq!(state.cursor.line, 33); // 10 + 24 - 1 = 33
        assert_eq!(state.cursor.column, 0);
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-04): screen-relative cursor and half-page scroll
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_viewport_near_end_when_cursor_screen_bottom_then_clamped_to_last_line() {
        let mut state = editor_state::new(80, 24);
        // Buffer with only 15 lines, viewport at top
        let lines: Vec<&str> = (0..15).map(|_| "text").collect();
        state.buffer = crate::buffer::Buffer::from_string(&lines.join("\n"));
        state.viewport = crate::viewport::new(0, 24, 80);
        state.cursor = crate::cursor::new(5, 0);
        editor_state::register_builtin_commands(&mut state);

        // L should clamp to last line (14), not viewport bottom (23)
        let result = command::execute(&mut state, "cursor-screen-bottom");
        assert!(result.is_ok());
        assert_eq!(state.cursor.line, 14);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn scroll_half_page_moves_cursor_and_viewport() {
        // (top_line, cursor_line, cursor_col, command, expected_cursor_line, expected_top_line, label)
        let cases: Vec<(usize, usize, usize, &str, usize, usize, &str)> = vec![
            (
                0,
                5,
                3,
                "scroll-half-page-down",
                17,
                12,
                "down moves cursor and viewport by half page",
            ),
            (
                20,
                30,
                2,
                "scroll-half-page-up",
                18,
                8,
                "up moves cursor and viewport by half page",
            ),
        ];

        for (
            top_line,
            cursor_line,
            cursor_col,
            command_name,
            expected_cursor,
            expected_top,
            label,
        ) in &cases
        {
            let mut state = editor_state::new(80, 24);
            let lines: Vec<&str> = (0..50).map(|_| "content").collect();
            state.buffer = crate::buffer::Buffer::from_string(&lines.join("\n"));
            state.viewport = crate::viewport::new(*top_line, 24, 80);
            state.cursor = crate::cursor::new(*cursor_line, *cursor_col);
            editor_state::register_builtin_commands(&mut state);

            let result = command::execute(&mut state, command_name);
            assert!(result.is_ok(), "{}: should succeed", label);
            assert_eq!(
                state.cursor.line, *expected_cursor,
                "{}: cursor line",
                label
            );
            assert_eq!(
                state.viewport.top_line, *expected_top,
                "{}: viewport top_line",
                label
            );
        }
    }

    #[test]
    fn scroll_half_page_clamps_at_buffer_boundaries() {
        // (num_lines, top_line, cursor_line, command, expected_cursor, expected_top, label)
        let cases: Vec<(usize, usize, usize, &str, usize, usize, &str)> = vec![
            (
                20,
                5,
                15,
                "scroll-half-page-down",
                19,
                17,
                "down clamps to last line",
            ),
            (
                50,
                3,
                5,
                "scroll-half-page-up",
                0,
                0,
                "up clamps to first line",
            ),
        ];

        for (
            num_lines,
            top_line,
            cursor_line,
            command_name,
            expected_cursor,
            expected_top,
            label,
        ) in &cases
        {
            let mut state = editor_state::new(80, 24);
            let lines: Vec<&str> = (0..*num_lines).map(|_| "text").collect();
            state.buffer = crate::buffer::Buffer::from_string(&lines.join("\n"));
            state.viewport = crate::viewport::new(*top_line, 24, 80);
            state.cursor = crate::cursor::new(*cursor_line, 0);
            editor_state::register_builtin_commands(&mut state);

            let result = command::execute(&mut state, command_name);
            assert!(result.is_ok(), "{}: should succeed", label);
            assert_eq!(
                state.cursor.line, *expected_cursor,
                "{}: cursor line",
                label
            );
            assert_eq!(
                state.viewport.top_line, *expected_top,
                "{}: viewport top_line",
                label
            );
        }
    }
}
