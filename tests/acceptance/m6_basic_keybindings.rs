//! M6 Acceptance Tests: Basic Keybinding Plugin
//!
//! What M6 proves: Plugins can intercept input, bind keys, and perform
//! buffer mutations. No hardcoded key handling remains in the kernel.
//!
//! Driving ports exercised:
//!   - PluginRegistry::load_all() — load basic-keybindings plugin
//!   - KeymapRegistry / keymap::resolve() — resolve key events to commands
//!   - CommandRegistry::execute() — execute resolved commands
//!   - EditorState — observe buffer/cursor changes after key processing
//!
//! This milestone is the critical architectural inflection point:
//! after M6, ALL key handling flows through the plugin-defined keymap system.

mod helpers;

// ---------------------------------------------------------------------------
// Walking Skeleton
// ---------------------------------------------------------------------------

/// WS-4: Plugin-defined keybinding inserts a character.
///
/// Given the basic keybinding plugin is loaded
/// And the buffer contains text "Hello"
/// When the user presses the 'a' key
/// Then the character 'a' is inserted into the buffer at the cursor position
#[test]
#[ignore]
fn given_keybinding_plugin_when_key_pressed_then_character_inserted() {
    // Given: basic keybinding plugin loaded, buffer with "Hello", cursor at end
    let (_dir, _file_path) = helpers::create_temp_file("test.txt", "Hello");

    // Load basic-keybindings plugin
    // state.buffer = Buffer::from_str("Hello");
    // state.cursor = Cursor { line: 0, column: 5 };

    // When: user presses 'a'
    // let key = KeyEvent { code: KeyCode::Char('a'), modifiers: Modifiers::NONE };
    // let resolved = keymap::resolve(&state.keymaps, &state.active_keymaps, &KeySequence::from(key));
    // Execute the resolved command (or insert character if no command match)

    // Then: buffer is "Helloa"
    // assert_eq!(state.buffer.content(), "Helloa");
    // assert_eq!(state.cursor.column, 6);

    todo!("Implement when basic-keybindings plugin exists");
}

// ---------------------------------------------------------------------------
// Happy Path
// ---------------------------------------------------------------------------

/// M6-H1: Arrow key moves cursor via plugin-defined binding.
///
/// Given the basic keybinding plugin is loaded
/// When the user presses the down arrow key
/// Then the cursor moves down one line
#[test]
#[ignore]
fn given_keybinding_plugin_when_down_arrow_then_cursor_moves_down() {
    // Given: plugin loaded, file with multiple lines, cursor at (0, 0)
    let content = "Line one\nLine two\nLine three";
    let (_dir, _file_path) = helpers::create_temp_file("test.txt", content);

    // When: user presses Down arrow
    // let key = KeyEvent { code: KeyCode::Down, modifiers: Modifiers::NONE };
    // Process through keymap -> command -> cursor move

    // Then: cursor at (1, 0)
    // assert_eq!(state.cursor.line, 1);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-H2: Backspace deletes character before cursor.
///
/// Given the basic keybinding plugin is loaded
/// And the buffer contains "ab" with cursor after 'b' (column 2)
/// When the user presses backspace
/// Then 'b' is deleted and the buffer contains "a"
#[test]
#[ignore]
fn given_keybinding_plugin_when_backspace_then_previous_char_deleted() {
    // Given: buffer "ab", cursor at column 2
    // state.buffer = Buffer::from_str("ab");
    // state.cursor = Cursor { line: 0, column: 2 };

    // When: backspace
    // let key = KeyEvent { code: KeyCode::Backspace, modifiers: Modifiers::NONE };
    // Process through keymap -> command -> buffer delete

    // Then: buffer is "a", cursor at column 1
    // assert_eq!(state.buffer.content(), "a");
    // assert_eq!(state.cursor.column, 1);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-H3: Ctrl-Q signals editor quit.
///
/// Given the basic keybinding plugin is loaded
/// When the user presses Ctrl-Q
/// Then the editor signals quit
#[test]
#[ignore]
fn given_keybinding_plugin_when_ctrl_q_then_editor_quits() {
    // Given: plugin loaded, editor running

    // When: Ctrl-Q pressed
    // let key = KeyEvent { code: KeyCode::Char('q'), modifiers: Modifiers::CTRL };
    // Process through keymap -> command -> set running = false

    // Then: editor signals quit
    // assert!(!state.running);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-H4: No keybinding plugin means no key handling (proves not hardcoded).
///
/// Given no keybinding plugin is loaded
/// When any key is pressed
/// Then no action occurs (buffer unchanged, cursor unchanged)
#[test]
#[ignore]
fn given_no_keybinding_plugin_when_key_pressed_then_no_action() {
    // Given: no plugin loaded, empty keymap stack
    // state.buffer = Buffer::from_str("Hello");
    // state.cursor = Cursor { line: 0, column: 0 };
    // No plugins loaded, no keymaps defined

    // When: user presses 'a'
    // let key = KeyEvent { code: KeyCode::Char('a'), modifiers: Modifiers::NONE };
    // let resolved = keymap::resolve(&state.keymaps, &state.active_keymaps, &KeySequence::from(key));

    // Then: no command found, buffer unchanged
    // assert!(resolved.is_none() or NoMatch);
    // assert_eq!(state.buffer.content(), "Hello");
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when basic-keybindings plugin exists");
}

// ---------------------------------------------------------------------------
// Error Path
// ---------------------------------------------------------------------------

/// M6-E1: Pressing an unbound key does nothing.
///
/// Given the basic keybinding plugin is loaded
/// When the user presses a key with no binding (e.g., F12)
/// Then nothing happens and the editor remains stable
#[test]
#[ignore]
fn given_keybinding_plugin_when_unbound_key_then_nothing_happens() {
    // Given: plugin loaded
    // state.buffer = Buffer::from_str("Hello");

    // When: F12 pressed (no binding)
    // let key = KeyEvent { code: KeyCode::F(12), modifiers: Modifiers::NONE };
    // let resolved = keymap::resolve(&state.keymaps, &state.active_keymaps, &KeySequence::from(key));

    // Then: no action, buffer unchanged
    // assert_eq!(state.buffer.content(), "Hello");
    // assert!(state.running);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-E2: Backspace at buffer start does nothing.
///
/// Given the basic keybinding plugin is loaded and cursor is at buffer start
/// When backspace is pressed
/// Then nothing happens (no underflow)
#[test]
#[ignore]
fn given_keybinding_plugin_when_backspace_at_start_then_nothing() {
    // Given: cursor at (0, 0), buffer has content
    // state.buffer = Buffer::from_str("Hello");
    // state.cursor = Cursor { line: 0, column: 0 };

    // When: backspace
    // Process through keymap -> command -> (nothing to delete)

    // Then: buffer unchanged
    // assert_eq!(state.buffer.content(), "Hello");
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-E3: Command execution error shows message, editor stable.
///
/// Given a keybinding is mapped to a command that fails
/// When the key is pressed
/// Then an error message is displayed and the editor remains stable
#[test]
#[ignore]
fn given_keybinding_to_failing_command_when_pressed_then_error_message_and_stable() {
    // Given: a keybinding mapped to a command that throws
    // (Set up a plugin that defines a command which calls (error "oops"))

    // When: the key is pressed
    // Process through keymap -> command -> error

    // Then: error message displayed, editor still running
    // assert!(state.message.as_ref().unwrap().contains("oops") || state.message.is_some());
    // assert!(state.running);

    todo!("Implement when basic-keybindings plugin exists");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// M6-EC1: Typing into an empty buffer inserts the character.
///
/// Given the basic keybinding plugin is loaded and the buffer is empty
/// When the user types a character 'x'
/// Then the buffer contains "x"
#[test]
#[ignore]
fn given_keybinding_plugin_and_empty_buffer_when_types_char_then_char_inserted() {
    // Given: empty buffer
    // state.buffer = Buffer::from_str("");
    // state.cursor = Cursor { line: 0, column: 0 };

    // When: type 'x'
    // Process key event for 'x'

    // Then: buffer is "x"
    // assert_eq!(state.buffer.content(), "x");
    // assert_eq!(state.cursor.column, 1);

    todo!("Implement when basic-keybindings plugin exists");
}

/// M6-EC2: Multiple rapid keystrokes all insert in order.
///
/// Given the basic keybinding plugin is loaded and the buffer is empty
/// When the user types "abc" rapidly
/// Then the buffer contains "abc" and the cursor is at column 3
#[test]
#[ignore]
fn given_keybinding_plugin_when_rapid_keystrokes_then_all_insert_in_order() {
    // Given: empty buffer
    // state.buffer = Buffer::from_str("");
    // state.cursor = Cursor { line: 0, column: 0 };

    // When: type 'a', 'b', 'c' in sequence
    // for key_event in helpers::type_string("abc") {
    //     process_key(&mut state, &key_event);
    // }

    // Then: buffer is "abc"
    // assert_eq!(state.buffer.content(), "abc");
    // assert_eq!(state.cursor.column, 3);

    todo!("Implement when basic-keybindings plugin exists");
}
