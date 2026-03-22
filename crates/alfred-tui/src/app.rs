//! App: event loop, key conversion, and key handling for the Alfred editor.
//!
//! This module ties together the event loop, crossterm key event conversion,
//! and key dispatch logic. The event loop is I/O (reads terminal events),
//! but key conversion and key handling are pure functions that are easily tested.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use crossterm::event::{
    self as ct_event, Event, KeyCode as CtKeyCode, KeyEvent as CtKeyEvent, KeyEventKind,
    KeyModifiers as CtKeyModifiers,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use alfred_core::cursor;
use alfred_core::editor_state::EditorState;
use alfred_core::key_event::{KeyCode, KeyEvent, Modifiers};
use alfred_core::viewport;
use alfred_lisp::runtime::LispRuntime;

use crate::renderer;

// ---------------------------------------------------------------------------
// Pure function: convert crossterm KeyEvent to alfred-core KeyEvent
// ---------------------------------------------------------------------------

/// Converts a crossterm KeyEvent into an alfred-core KeyEvent.
///
/// This is a pure mapping function with no side effects. It translates
/// crossterm's key code and modifier representation into alfred-core's
/// domain-independent representation.
pub(crate) fn convert_crossterm_key(ct_key: CtKeyEvent) -> KeyEvent {
    let code = convert_key_code(ct_key.code);
    let modifiers = convert_modifiers(ct_key.modifiers);
    KeyEvent::new(code, modifiers)
}

/// Converts a crossterm KeyCode to an alfred-core KeyCode.
fn convert_key_code(ct_code: CtKeyCode) -> KeyCode {
    match ct_code {
        CtKeyCode::Char(c) => KeyCode::Char(c),
        CtKeyCode::Up => KeyCode::Up,
        CtKeyCode::Down => KeyCode::Down,
        CtKeyCode::Left => KeyCode::Left,
        CtKeyCode::Right => KeyCode::Right,
        CtKeyCode::Enter => KeyCode::Enter,
        CtKeyCode::Esc => KeyCode::Escape,
        CtKeyCode::Backspace => KeyCode::Backspace,
        CtKeyCode::Tab => KeyCode::Tab,
        CtKeyCode::Home => KeyCode::Home,
        CtKeyCode::End => KeyCode::End,
        CtKeyCode::PageUp => KeyCode::PageUp,
        CtKeyCode::PageDown => KeyCode::PageDown,
        CtKeyCode::Delete => KeyCode::Delete,
        // Unmapped keys default to Escape (ignored by handler)
        _ => KeyCode::Escape,
    }
}

/// Converts crossterm KeyModifiers to alfred-core Modifiers.
fn convert_modifiers(ct_mods: CtKeyModifiers) -> Modifiers {
    Modifiers {
        ctrl: ct_mods.contains(CtKeyModifiers::CONTROL),
        alt: ct_mods.contains(CtKeyModifiers::ALT),
        shift: ct_mods.contains(CtKeyModifiers::SHIFT),
    }
}

// ---------------------------------------------------------------------------
// Pure function: handle a key event by updating EditorState
// ---------------------------------------------------------------------------

/// Tracks multi-key input state (e.g., command-line after `:`)
#[derive(Debug, PartialEq)]
pub(crate) enum InputState {
    /// Normal key dispatch
    Normal,
    /// Accumulating a command-line string (entered via `:`)
    Command(String),
}

/// Handles a key event by updating the editor state.
///
/// Returns `(InputState, DeferredAction)` where the DeferredAction tells the
/// caller what to do after dropping the EditorState borrow (eval Lisp, execute
/// a registered command, or nothing).
pub(crate) fn handle_key_event(
    state: &mut EditorState,
    key: KeyEvent,
    input_state: InputState,
) -> (InputState, DeferredAction) {
    // Command-line mode: accumulating input after `:`
    if let InputState::Command(mut cmd) = input_state {
        match key.code {
            KeyCode::Enter => {
                let trimmed = cmd.trim().to_string();
                return execute_colon_command(state, &trimmed);
            }
            KeyCode::Escape => {
                state.message = None;
                return (InputState::Normal, DeferredAction::None);
            }
            KeyCode::Backspace => {
                cmd.pop();
                if cmd.is_empty() {
                    state.message = None;
                    return (InputState::Normal, DeferredAction::None);
                }
                state.message = Some(format!(":{}", cmd));
                return (InputState::Command(cmd), DeferredAction::None);
            }
            KeyCode::Char(c) => {
                cmd.push(c);
                state.message = Some(format!(":{}", cmd));
                return (InputState::Command(cmd), DeferredAction::None);
            }
            _ => {
                return (InputState::Command(cmd), DeferredAction::None);
            }
        }
    }

    // Normal mode
    match key {
        KeyEvent {
            code: KeyCode::Char(':'),
            modifiers: Modifiers { ctrl: false, .. },
        } => {
            state.message = Some(":".to_string());
            return (InputState::Command(String::new()), DeferredAction::None);
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: Modifiers { ctrl: false, .. },
        } => move_cursor_and_adjust_viewport(state, cursor::move_up),
        KeyEvent {
            code: KeyCode::Down,
            modifiers: Modifiers { ctrl: false, .. },
        } => move_cursor_and_adjust_viewport(state, cursor::move_down),
        KeyEvent {
            code: KeyCode::Left,
            modifiers: Modifiers { ctrl: false, .. },
        } => move_cursor_and_adjust_viewport(state, cursor::move_left),
        KeyEvent {
            code: KeyCode::Right,
            modifiers: Modifiers { ctrl: false, .. },
        } => move_cursor_and_adjust_viewport(state, cursor::move_right),
        _ => {}
    }
    (InputState::Normal, DeferredAction::None)
}

/// Action to perform after handle_key_event releases the EditorState borrow.
#[derive(Debug, PartialEq)]
pub(crate) enum DeferredAction {
    /// No action needed
    None,
    /// Evaluate a Lisp expression (from :eval)
    Eval(String),
    /// Execute a registered command by name (from :command-name)
    ExecCommand(String),
}

/// Executes a colon command, returning the new input state and a deferred action.
///
/// Commands that need Lisp evaluation or registered command execution return
/// a DeferredAction so the caller can execute them after dropping the borrow
/// on EditorState (avoiding RefCell double-borrow panics).
fn execute_colon_command(state: &mut EditorState, command: &str) -> (InputState, DeferredAction) {
    match command {
        "q" | "quit" => {
            state.running = false;
            (InputState::Normal, DeferredAction::None)
        }
        cmd if cmd.starts_with("eval ") => {
            let expression = cmd.strip_prefix("eval ").unwrap().to_string();
            (InputState::Normal, DeferredAction::Eval(expression))
        }
        cmd => {
            // Check if it's a registered command — defer execution to avoid borrow conflict
            if alfred_core::command::lookup(&state.commands, cmd).is_some() {
                (InputState::Normal, DeferredAction::ExecCommand(cmd.to_string()))
            } else {
                state.message = Some(format!("Unknown command: {}", cmd));
                (InputState::Normal, DeferredAction::None)
            }
        }
    }
}

/// Applies a cursor movement function and adjusts the viewport to follow.
fn move_cursor_and_adjust_viewport(
    state: &mut EditorState,
    move_fn: fn(
        alfred_core::cursor::Cursor,
        &alfred_core::buffer::Buffer,
    ) -> alfred_core::cursor::Cursor,
) {
    state.cursor = move_fn(state.cursor, &state.buffer);
    state.viewport = viewport::adjust(state.viewport, &state.cursor);
}

/// Evaluates a Lisp expression and sets the result (or error) as the editor message.
///
/// This function borrows `state_rc` only when needed, avoiding conflicts
/// with handle_key_event's borrow. The runtime's bridge closures also
/// borrow `state_rc`, so this must be called after `handle_key_event` returns.
pub(crate) fn eval_and_display(
    state_rc: &Rc<RefCell<EditorState>>,
    runtime: &LispRuntime,
    expression: &str,
) {
    // Clear the command-line text before eval so we can detect if a primitive sets the message
    state_rc.borrow_mut().message = None;

    match runtime.eval(expression) {
        Ok(value) => {
            // If a bridge primitive (like `message`) already set the message during eval,
            // keep it. Otherwise show the eval result.
            let mut state = state_rc.borrow_mut();
            if state.message.is_none() {
                let display = format!("{}", value);
                state.message = Some(display);
            }
        }
        Err(err) => {
            state_rc.borrow_mut().message = Some(format!("Lisp error: {}", err));
        }
    }
}

// ---------------------------------------------------------------------------
// I/O: event loop
// ---------------------------------------------------------------------------

/// Runs the main editor event loop.
///
/// This function is the imperative shell. It:
/// 1. Enters raw mode (via RawModeGuard for cleanup safety)
/// 2. Creates a ratatui Terminal with CrosstermBackend
/// 3. Loops while `state.running`:
///    a. Renders the current frame
///    b. Reads the next crossterm event (blocking)
///    c. Converts crossterm KeyEvent to alfred-core KeyEvent
///    d. Handles the key event (updates state)
///    e. If an eval expression was returned, evaluates it via the Lisp runtime
/// 4. On exit: clears screen, raw mode guard drops (restores terminal)
pub fn run(state_rc: &Rc<RefCell<EditorState>>, runtime: &LispRuntime) -> io::Result<()> {
    let _raw_guard = renderer::RawModeGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut input_state = InputState::Normal;

    loop {
        // Check if still running
        if !state_rc.borrow().running {
            break;
        }

        // Render current frame
        renderer::render_frame(&mut terminal, &state_rc.borrow())?;

        if let Event::Key(ct_key) = ct_event::read()? {
            // Only handle key press events (not release/repeat)
            if ct_key.kind == KeyEventKind::Press {
                let key = convert_crossterm_key(ct_key);

                // Handle the key event (borrow state, then drop before deferred action)
                let deferred = {
                    let mut state = state_rc.borrow_mut();
                    let (new_input_state, action) =
                        handle_key_event(&mut state, key, input_state);
                    input_state = new_input_state;
                    action
                }; // borrow dropped here

                // Execute deferred actions outside the borrow
                match deferred {
                    DeferredAction::Eval(expr) => {
                        eval_and_display(state_rc, runtime, &expr);
                    }
                    DeferredAction::ExecCommand(cmd_name) => {
                        // Clear command-line text, then execute
                        state_rc.borrow_mut().message = None;
                        let result = alfred_core::command::execute(
                            &mut state_rc.borrow_mut(),
                            &cmd_name,
                        );
                        if let Err(e) = result {
                            state_rc.borrow_mut().message =
                                Some(format!("Command error: {}", e));
                        }
                    }
                    DeferredAction::None => {}
                }
            }
        }
    }

    terminal.clear()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use alfred_core::buffer::Buffer;
    use alfred_core::cursor;
    use alfred_core::editor_state;
    use alfred_core::key_event::{KeyCode, KeyEvent};
    use crossterm::event::{
        KeyCode as CtKeyCode, KeyEvent as CtKeyEvent, KeyEventKind, KeyEventState,
        KeyModifiers as CtKeyModifiers,
    };

    /// Helper: call handle_key_event and return just the InputState (ignoring eval).
    /// Used by existing tests that chain key events and don't care about Lisp eval.
    fn handle_key(
        state: &mut alfred_core::editor_state::EditorState,
        key: KeyEvent,
        input_state: super::InputState,
    ) -> super::InputState {
        super::handle_key_event(state, key, input_state).0
    }

    // -----------------------------------------------------------------------
    // Acceptance test: simulate a sequence of key events on EditorState,
    // verifying cursor movement and running flag changes
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_with_multiline_buffer_when_key_events_dispatched_then_cursor_moves_and_quit_stops_running(
    ) {
        // Given: an EditorState with a 3-line buffer
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld!\nBye");

        // Cursor starts at (0, 0), running is true
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
        assert!(state.running);

        // When: press Down arrow
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Down),
            super::InputState::Normal,
        );
        // Then: cursor moves to line 1
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);

        // When: press Right arrow twice
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Right),
            super::InputState::Normal,
        );
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Right),
            super::InputState::Normal,
        );
        // Then: cursor at (1, 2)
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 2);

        // When: press Up arrow
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Up),
            super::InputState::Normal,
        );
        // Then: cursor moves to line 0, column 2
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 2);

        // When: press Left arrow
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Left),
            super::InputState::Normal,
        );
        // Then: cursor at (0, 1)
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 1);

        // Viewport should be adjusted after each key event
        // (cursor is visible within viewport)
        assert!(state.cursor.line >= state.viewport.top_line);
        assert!(state.cursor.line < state.viewport.top_line + state.viewport.height as usize);

        // When: type :q Enter to quit
        let result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('q')), result);
        handle_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);
        // Then: running is false
        assert!(!state.running);
    }

    // -----------------------------------------------------------------------
    // Acceptance test: scrolling works when cursor moves past viewport
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_taller_than_viewport_when_cursor_moves_past_bottom_then_viewport_scrolls() {
        // Given: an EditorState with a small viewport (height=5) and a 10-line buffer
        let mut state = editor_state::new(80, 5);
        let lines: Vec<&str> = vec![
            "Line0", "Line1", "Line2", "Line3", "Line4", "Line5", "Line6", "Line7", "Line8",
            "Line9",
        ];
        state.buffer = Buffer::from_string(&lines.join("\n"));
        assert_eq!(state.viewport.top_line, 0);

        // When: move cursor down 6 times (past the 5-line viewport)
        for _ in 0..6 {
            handle_key(
                &mut state,
                KeyEvent::plain(KeyCode::Down),
                super::InputState::Normal,
            );
        }

        // Then: cursor is at line 6
        assert_eq!(state.cursor.line, 6);

        // And: viewport has scrolled to keep cursor visible
        assert!(state.viewport.top_line > 0);
        assert!(state.cursor.line >= state.viewport.top_line);
        assert!(state.cursor.line < state.viewport.top_line + state.viewport.height as usize);
    }

    // -----------------------------------------------------------------------
    // Unit tests: convert_crossterm_key -- maps crossterm KeyEvent to alfred KeyEvent
    // -----------------------------------------------------------------------

    fn make_crossterm_key(code: CtKeyCode, modifiers: CtKeyModifiers) -> CtKeyEvent {
        CtKeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn given_crossterm_arrow_up_when_converted_then_returns_alfred_up() {
        let ct_event = make_crossterm_key(CtKeyCode::Up, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Up);
        assert!(!result.modifiers.ctrl);
    }

    #[test]
    fn given_crossterm_arrow_down_when_converted_then_returns_alfred_down() {
        let ct_event = make_crossterm_key(CtKeyCode::Down, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Down);
    }

    #[test]
    fn given_crossterm_arrow_left_when_converted_then_returns_alfred_left() {
        let ct_event = make_crossterm_key(CtKeyCode::Left, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Left);
    }

    #[test]
    fn given_crossterm_arrow_right_when_converted_then_returns_alfred_right() {
        let ct_event = make_crossterm_key(CtKeyCode::Right, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Right);
    }

    #[test]
    fn given_crossterm_char_with_ctrl_when_converted_then_returns_alfred_char_with_ctrl() {
        let ct_event = make_crossterm_key(CtKeyCode::Char('q'), CtKeyModifiers::CONTROL);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Char('q'));
        assert!(result.modifiers.ctrl);
        assert!(!result.modifiers.alt);
        assert!(!result.modifiers.shift);
    }

    #[test]
    fn given_crossterm_char_with_alt_when_converted_then_returns_alfred_char_with_alt() {
        let ct_event = make_crossterm_key(CtKeyCode::Char('x'), CtKeyModifiers::ALT);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Char('x'));
        assert!(result.modifiers.alt);
    }

    #[test]
    fn given_crossterm_enter_when_converted_then_returns_alfred_enter() {
        let ct_event = make_crossterm_key(CtKeyCode::Enter, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Enter);
    }

    #[test]
    fn given_crossterm_escape_when_converted_then_returns_alfred_escape() {
        let ct_event = make_crossterm_key(CtKeyCode::Esc, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Escape);
    }

    #[test]
    fn given_crossterm_backspace_when_converted_then_returns_alfred_backspace() {
        let ct_event = make_crossterm_key(CtKeyCode::Backspace, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Backspace);
    }

    #[test]
    fn given_crossterm_tab_when_converted_then_returns_alfred_tab() {
        let ct_event = make_crossterm_key(CtKeyCode::Tab, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Tab);
    }

    #[test]
    fn given_crossterm_home_when_converted_then_returns_alfred_home() {
        let ct_event = make_crossterm_key(CtKeyCode::Home, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Home);
    }

    #[test]
    fn given_crossterm_end_when_converted_then_returns_alfred_end() {
        let ct_event = make_crossterm_key(CtKeyCode::End, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::End);
    }

    #[test]
    fn given_crossterm_pageup_when_converted_then_returns_alfred_pageup() {
        let ct_event = make_crossterm_key(CtKeyCode::PageUp, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::PageUp);
    }

    #[test]
    fn given_crossterm_pagedown_when_converted_then_returns_alfred_pagedown() {
        let ct_event = make_crossterm_key(CtKeyCode::PageDown, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::PageDown);
    }

    #[test]
    fn given_crossterm_delete_when_converted_then_returns_alfred_delete() {
        let ct_event = make_crossterm_key(CtKeyCode::Delete, CtKeyModifiers::NONE);
        let result = super::convert_crossterm_key(ct_event);
        assert_eq!(result.code, KeyCode::Delete);
    }

    // -----------------------------------------------------------------------
    // Unit tests: handle_key_event -- individual key behaviors
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_when_down_arrow_then_cursor_line_increases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("aaa\nbbb\nccc");
        assert_eq!(state.cursor.line, 0);

        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Down),
            super::InputState::Normal,
        );
        assert_eq!(state.cursor.line, 1);
    }

    #[test]
    fn given_editor_when_up_arrow_then_cursor_line_decreases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("aaa\nbbb\nccc");
        state.cursor = cursor::new(2, 0);

        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Up),
            super::InputState::Normal,
        );
        assert_eq!(state.cursor.line, 1);
    }

    #[test]
    fn given_editor_when_right_arrow_then_cursor_column_increases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        assert_eq!(state.cursor.column, 0);

        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Right),
            super::InputState::Normal,
        );
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_editor_when_left_arrow_then_cursor_column_decreases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 3);

        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Left),
            super::InputState::Normal,
        );
        assert_eq!(state.cursor.column, 2);
    }

    #[test]
    fn given_editor_when_colon_q_enter_then_running_becomes_false() {
        let mut state = editor_state::new(80, 24);
        assert!(state.running);

        // `:` enters command mode
        let (input_state, _) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        assert!(matches!(input_state, super::InputState::Command(_)));
        assert_eq!(state.message, Some(":".to_string()));

        // Type `q`
        let (input_state, _) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Char('q')), input_state);
        assert!(matches!(input_state, super::InputState::Command(_)));
        assert_eq!(state.message, Some(":q".to_string()));

        // Press Enter to execute
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), input_state);
        assert!(!state.running);
    }

    #[test]
    fn given_editor_in_command_mode_when_escape_then_command_cancelled() {
        let mut state = editor_state::new(80, 24);

        // Enter command mode
        let result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        assert!(matches!(result, super::InputState::Command(_)));

        // Type some chars
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('x')), result);

        // Escape cancels
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Escape), result);
        assert_eq!(result, super::InputState::Normal);
        assert!(state.running);
        assert_eq!(state.message, None);
    }

    #[test]
    fn given_editor_when_unknown_command_then_shows_error_message() {
        let mut state = editor_state::new(80, 24);

        // :foo Enter
        let result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('f')), result);
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('o')), result);
        let result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('o')), result);
        handle_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        assert!(state.running); // Did NOT quit
        assert_eq!(state.message, Some("Unknown command: foo".to_string()));
    }

    #[test]
    fn given_editor_when_quit_command_then_also_accepts_full_word() {
        let mut state = editor_state::new(80, 24);

        // :quit Enter
        let mut result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "quit".chars() {
            result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        handle_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);
        assert!(!state.running);
    }

    #[test]
    fn given_editor_when_unhandled_key_then_state_unchanged() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        let cursor_before = state.cursor;
        let running_before = state.running;

        // Press 'a' -- no insert in M1, should be ignored
        handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char('a')),
            super::InputState::Normal,
        );
        assert_eq!(state.cursor, cursor_before);
        assert_eq!(state.running, running_before);
    }

    #[test]
    fn given_editor_when_arrow_key_then_viewport_adjusted() {
        let mut state = editor_state::new(80, 3);
        state.buffer = Buffer::from_string("L0\nL1\nL2\nL3\nL4\nL5");
        assert_eq!(state.viewport.top_line, 0);

        // Move cursor past viewport bottom
        for _ in 0..4 {
            handle_key(
                &mut state,
                KeyEvent::plain(KeyCode::Down),
                super::InputState::Normal,
            );
        }
        // Viewport should have scrolled
        assert!(state.viewport.top_line > 0);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (02-04): eval command via :eval prefix
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_with_bridge_when_eval_message_command_then_state_message_changes() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: an editor state wrapped in Rc<RefCell> (for bridge sharing)
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));

        // And: a Lisp runtime with core primitives registered
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());

        // When: simulate typing `:eval (message "hi")` and pressing Enter
        let deferred = {
            let mut state = state_rc.borrow_mut();
            let mut result = handle_key(
                &mut state,
                KeyEvent::plain(KeyCode::Char(':')),
                super::InputState::Normal,
            );
            for c in "eval (message \"hi\")".chars() {
                result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
            }
            let (_, action) =
                super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);
            action
        };

        if let super::DeferredAction::Eval(expr) = deferred {
            super::eval_and_display(&state_rc, &runtime, &expr);
        }

        // Then: the message has been set to "hi" by the Lisp (message ...) primitive
        let state = state_rc.borrow();
        assert_eq!(state.message, Some("hi".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests (02-04): eval command parsing and error handling
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_when_eval_command_entered_then_returns_eval_expression() {
        let mut state = editor_state::new(80, 24);

        // Type `:eval (+ 1 2)` and press Enter
        let mut result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "eval (+ 1 2)".chars() {
            result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        let (input_state, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        // Then: returns the expression to eval
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::Eval("(+ 1 2)".to_string()));
    }

    #[test]
    fn given_editor_when_lisp_eval_error_then_message_shows_error_not_crash() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: runtime with bridge
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());

        // When: evaluate invalid Lisp expression
        let deferred = {
            let mut state = state_rc.borrow_mut();
            let mut result = handle_key(
                &mut state,
                KeyEvent::plain(KeyCode::Char(':')),
                super::InputState::Normal,
            );
            for c in "eval (+ 1".chars() {
                result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
            }
            let (_, action) =
                super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);
            action
        };

        if let super::DeferredAction::Eval(expr) = deferred {
            super::eval_and_display(&state_rc, &runtime, &expr);
        }

        // Then: message contains an error, editor still running
        let state = state_rc.borrow();
        assert!(state.message.is_some());
        let msg = state.message.as_ref().unwrap();
        assert!(
            msg.contains("error") || msg.contains("Error"),
            "Expected error message, got: {}",
            msg
        );
        assert!(state.running); // editor did not crash
    }

    #[test]
    fn given_editor_when_q_command_then_still_quits_after_lisp_integration() {
        let mut state = editor_state::new(80, 24);
        assert!(state.running);

        // Type `:q` and press Enter (should still work)
        let mut result = handle_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        result = handle_key(&mut state, KeyEvent::plain(KeyCode::Char('q')), result);
        let (input_state, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        // Then: quit works, no deferred action
        assert!(!state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
    }
}
