//! App: event loop, key conversion, and key handling for the Alfred editor.
//!
//! This module ties together the event loop, crossterm key event conversion,
//! and key dispatch logic. The event loop is I/O (reads terminal events),
//! but key conversion and key handling are pure functions that are easily tested.

use std::io;

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

use crate::renderer;

// ---------------------------------------------------------------------------
// Pure function: convert crossterm KeyEvent to alfred-core KeyEvent
// ---------------------------------------------------------------------------

/// Converts a crossterm KeyEvent into an alfred-core KeyEvent.
///
/// This is a pure mapping function with no side effects. It translates
/// crossterm's key code and modifier representation into alfred-core's
/// domain-independent representation.
pub fn convert_crossterm_key(ct_key: CtKeyEvent) -> KeyEvent {
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

/// Handles a key event by updating the editor state.
///
/// This function dispatches on the key event to perform cursor movement
/// or quit the editor. For M1, keybindings are hardcoded:
/// - Arrow keys: cursor movement (Up, Down, Left, Right)
/// - Ctrl-Q: quit (set running = false)
/// - All other keys: ignored (read-only in M1)
///
/// After handling cursor movement, the viewport is adjusted to ensure
/// the cursor remains visible.
pub fn handle_key_event(state: &mut EditorState, key: KeyEvent) {
    match key {
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: Modifiers { ctrl: true, .. },
        } => {
            state.running = false;
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: Modifiers { ctrl: false, .. },
        } => {
            state.cursor = cursor::move_up(state.cursor, &state.buffer);
            state.viewport = viewport::adjust(state.viewport, &state.cursor);
        }
        KeyEvent {
            code: KeyCode::Down,
            modifiers: Modifiers { ctrl: false, .. },
        } => {
            state.cursor = cursor::move_down(state.cursor, &state.buffer);
            state.viewport = viewport::adjust(state.viewport, &state.cursor);
        }
        KeyEvent {
            code: KeyCode::Left,
            modifiers: Modifiers { ctrl: false, .. },
        } => {
            state.cursor = cursor::move_left(state.cursor, &state.buffer);
            state.viewport = viewport::adjust(state.viewport, &state.cursor);
        }
        KeyEvent {
            code: KeyCode::Right,
            modifiers: Modifiers { ctrl: false, .. },
        } => {
            state.cursor = cursor::move_right(state.cursor, &state.buffer);
            state.viewport = viewport::adjust(state.viewport, &state.cursor);
        }
        // M1: all other keys are ignored (buffer is read-only)
        _ => {}
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
/// 4. On exit: clears screen, raw mode guard drops (restores terminal)
pub fn run(state: &mut EditorState) -> io::Result<()> {
    let _raw_guard = renderer::RawModeGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    while state.running {
        renderer::render_frame(&mut terminal, state)?;

        if let Event::Key(ct_key) = ct_event::read()? {
            // Only handle key press events (not release/repeat)
            if ct_key.kind == KeyEventKind::Press {
                let key = convert_crossterm_key(ct_key);
                handle_key_event(state, key);
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
    use alfred_core::key_event::{KeyCode, KeyEvent, Modifiers};
    use crossterm::event::{
        KeyCode as CtKeyCode, KeyEvent as CtKeyEvent, KeyEventKind, KeyEventState,
        KeyModifiers as CtKeyModifiers,
    };

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
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Down));
        // Then: cursor moves to line 1
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);

        // When: press Right arrow twice
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Right));
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Right));
        // Then: cursor at (1, 2)
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 2);

        // When: press Up arrow
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Up));
        // Then: cursor moves to line 0, column 2
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 2);

        // When: press Left arrow
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Left));
        // Then: cursor at (0, 1)
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 1);

        // Viewport should be adjusted after each key event
        // (cursor is visible within viewport)
        assert!(state.cursor.line >= state.viewport.top_line);
        assert!(
            state.cursor.line < state.viewport.top_line + state.viewport.height as usize
        );

        // When: press Ctrl-Q
        super::handle_key_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('q'), Modifiers::ctrl()),
        );
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
            "Line0", "Line1", "Line2", "Line3", "Line4", "Line5", "Line6", "Line7",
            "Line8", "Line9",
        ];
        state.buffer = Buffer::from_string(&lines.join("\n"));
        assert_eq!(state.viewport.top_line, 0);

        // When: move cursor down 6 times (past the 5-line viewport)
        for _ in 0..6 {
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Down));
        }

        // Then: cursor is at line 6
        assert_eq!(state.cursor.line, 6);

        // And: viewport has scrolled to keep cursor visible
        assert!(state.viewport.top_line > 0);
        assert!(state.cursor.line >= state.viewport.top_line);
        assert!(
            state.cursor.line < state.viewport.top_line + state.viewport.height as usize
        );
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

        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Down));
        assert_eq!(state.cursor.line, 1);
    }

    #[test]
    fn given_editor_when_up_arrow_then_cursor_line_decreases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("aaa\nbbb\nccc");
        state.cursor = cursor::new(2, 0);

        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Up));
        assert_eq!(state.cursor.line, 1);
    }

    #[test]
    fn given_editor_when_right_arrow_then_cursor_column_increases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        assert_eq!(state.cursor.column, 0);

        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Right));
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_editor_when_left_arrow_then_cursor_column_decreases() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 3);

        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Left));
        assert_eq!(state.cursor.column, 2);
    }

    #[test]
    fn given_editor_when_ctrl_q_then_running_becomes_false() {
        let mut state = editor_state::new(80, 24);
        assert!(state.running);

        super::handle_key_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('q'), Modifiers::ctrl()),
        );
        assert!(!state.running);
    }

    #[test]
    fn given_editor_when_unhandled_key_then_state_unchanged() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        let cursor_before = state.cursor;
        let running_before = state.running;

        // Press 'a' -- no insert in M1, should be ignored
        super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Char('a')));
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
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Down));
        }
        // Viewport should have scrolled
        assert!(state.viewport.top_line > 0);
    }
}
