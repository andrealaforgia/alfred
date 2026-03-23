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

use alfred_core::editor_state::EditorState;
use alfred_core::key_event::{KeyCode, KeyEvent, Modifiers};
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

    // Normal mode: resolve key through active keymaps
    match alfred_core::editor_state::resolve_key(state, key) {
        Some(ref cmd) if cmd == "enter-command-mode" => {
            state.message = Some(":".to_string());
            (InputState::Command(String::new()), DeferredAction::None)
        }
        Some(cmd) => (InputState::Normal, DeferredAction::ExecCommand(cmd)),
        None => {
            // Self-insert: only in insert mode with active keymaps.
            // Handles printable characters and Enter (newline).
            if state.mode == alfred_core::editor_state::MODE_INSERT
                && !state.active_keymaps.is_empty()
            {
                match key.code {
                    KeyCode::Char(c) => {
                        alfred_core::editor_state::push_undo(state);
                        let line = state.cursor.line;
                        let col = state.cursor.column;
                        state.buffer = alfred_core::buffer::insert_at(
                            &state.buffer,
                            line,
                            col,
                            &c.to_string(),
                        );
                        state.cursor = alfred_core::cursor::move_right(state.cursor, &state.buffer);
                        state.viewport =
                            alfred_core::viewport::adjust(state.viewport, &state.cursor);
                    }
                    KeyCode::Enter => {
                        alfred_core::editor_state::push_undo(state);
                        let line = state.cursor.line;
                        let col = state.cursor.column;
                        state.buffer =
                            alfred_core::buffer::insert_at(&state.buffer, line, col, "\n");
                        // Move cursor to beginning of new line
                        state.cursor = alfred_core::cursor::new(line + 1, 0);
                        state.viewport =
                            alfred_core::viewport::adjust(state.viewport, &state.cursor);
                    }
                    _ => {}
                }
            }
            (InputState::Normal, DeferredAction::None)
        }
    }
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
    /// Save the current buffer to a file path (None = use buffer's file_path)
    SaveBuffer(Option<String>),
    /// Open a file into the buffer
    OpenFile(String),
    /// Save the current buffer then quit (from :wq)
    SaveAndQuit,
}

/// Executes a colon command, returning the new input state and a deferred action.
///
/// Commands that need Lisp evaluation or registered command execution return
/// a DeferredAction so the caller can execute them after dropping the borrow
/// on EditorState (avoiding RefCell double-borrow panics).
fn execute_colon_command(state: &mut EditorState, command: &str) -> (InputState, DeferredAction) {
    match command {
        "q" | "quit" => {
            if state.buffer.is_modified() {
                state.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                (InputState::Normal, DeferredAction::None)
            } else {
                state.running = false;
                (InputState::Normal, DeferredAction::None)
            }
        }
        "q!" => {
            state.running = false;
            (InputState::Normal, DeferredAction::None)
        }
        "wq" => (InputState::Normal, DeferredAction::SaveAndQuit),
        "w" => {
            // Save to the buffer's existing file path
            (InputState::Normal, DeferredAction::SaveBuffer(None))
        }
        cmd if cmd.starts_with("w ") => {
            let path = cmd.strip_prefix("w ").unwrap().trim().to_string();
            (InputState::Normal, DeferredAction::SaveBuffer(Some(path)))
        }
        cmd if cmd.starts_with("e ") => {
            let path = cmd.strip_prefix("e ").unwrap().trim().to_string();
            (InputState::Normal, DeferredAction::OpenFile(path))
        }
        cmd if cmd.starts_with("eval ") => {
            let expression = cmd.strip_prefix("eval ").unwrap().to_string();
            (InputState::Normal, DeferredAction::Eval(expression))
        }
        cmd => {
            // Check if it's a registered command — defer execution to avoid borrow conflict
            if alfred_core::command::lookup(&state.commands, cmd).is_some() {
                (
                    InputState::Normal,
                    DeferredAction::ExecCommand(cmd.to_string()),
                )
            } else {
                state.message = Some(format!("Unknown command: {}", cmd));
                (InputState::Normal, DeferredAction::None)
            }
        }
    }
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
// Pure function: compute gutter content from hook dispatch
// ---------------------------------------------------------------------------

/// Computes gutter content by dispatching the "render-gutter" hook.
///
/// If no hook is registered (no line-numbers plugin), returns (0, empty vec).
/// Otherwise, dispatches the hook with visible line range info and returns
/// (gutter_width, formatted_lines).
///
/// The gutter_width is calculated as: number of digits in line_count + 1 (for padding).
pub(crate) fn compute_gutter_content(state: &EditorState) -> (u16, Vec<String>) {
    let top_line = state.viewport.top_line;
    let height = state.viewport.height as usize;
    let line_count = alfred_core::buffer::line_count(&state.buffer);

    // Check if any hooks are registered for "render-gutter"
    let start_line_1indexed = top_line + 1;
    let end_line_1indexed = (top_line + height).min(line_count);

    let args = vec![
        start_line_1indexed.to_string(),
        end_line_1indexed.to_string(),
        line_count.to_string(),
    ];

    let results = alfred_core::hook::dispatch_hook(&state.hooks, "render-gutter", &args);

    if results.is_empty() {
        // No hook registered -- no gutter
        return (0, Vec::new());
    }

    // Calculate gutter width: digits in line_count + 1 for padding
    let digits = if line_count == 0 {
        1
    } else {
        (line_count as f64).log10().floor() as u16 + 1
    };
    let gutter_width = digits + 1;

    // Build formatted line numbers for visible rows
    let gutter_lines: Vec<String> = (0..height)
        .map(|row| {
            let buffer_line = top_line + row;
            if buffer_line < line_count {
                let line_num = buffer_line + 1; // 1-indexed
                format!("{:>width$} ", line_num, width = digits as usize)
            } else {
                " ".repeat(gutter_width as usize)
            }
        })
        .collect();

    (gutter_width, gutter_lines)
}

// ---------------------------------------------------------------------------
// Pure function: compute status bar content from hook dispatch
// ---------------------------------------------------------------------------

/// Computes status bar content by checking if the "render-status" hook has callbacks.
///
/// If no hook is registered, returns None (no status bar rendered).
/// Otherwise, builds a formatted status string from EditorState fields:
/// ` filename.txt  Ln 1, Col 0  [+]  NORMAL `
///
/// - Filename: buffer filename or "[No Name]" if unnamed
/// - Position: 1-indexed line, 0-indexed column
/// - Modified: "[+]" if buffer modified, omitted if clean
/// - Mode: current mode name uppercased
pub(crate) fn compute_status_content(state: &EditorState) -> Option<String> {
    let results = alfred_core::hook::dispatch_hook(&state.hooks, "render-status", &[]);

    if results.is_empty() {
        return None;
    }

    let filename = state.buffer.filename().unwrap_or("[No Name]");

    let line = state.cursor.line + 1; // 1-indexed for display
    let col = state.cursor.column;

    let modified = if state.buffer.is_modified() {
        "  [+]"
    } else {
        ""
    };

    let mode = state.mode.to_string().to_uppercase();

    Some(format!(
        " {}  Ln {}, Col {}{}  {} ",
        filename, line, col, modified, mode
    ))
}

// ---------------------------------------------------------------------------
// I/O: event loop
// ---------------------------------------------------------------------------

/// Runs the main editor event loop.
///
/// This function is the imperative shell. It:
/// 1. Enters raw mode and alternate screen (via TerminalGuard for cleanup safety)
/// 2. Creates a ratatui Terminal with CrosstermBackend
/// 3. Loops while `state.running`:
///    a. Renders the current frame
///    b. Reads the next crossterm event (blocking)
///    c. Converts crossterm KeyEvent to alfred-core KeyEvent
///    d. Handles the key event (updates state)
///    e. If an eval expression was returned, evaluates it via the Lisp runtime
/// 4. On exit: clears screen, terminal guard drops (leaves alternate screen, disables raw mode)
pub fn run(state_rc: &Rc<RefCell<EditorState>>, runtime: &LispRuntime) -> io::Result<()> {
    let _terminal_guard = renderer::TerminalGuard::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut input_state = InputState::Normal;

    loop {
        // Check if still running
        if !state_rc.borrow().running {
            break;
        }

        // Compute gutter content by dispatching "render-gutter" hook
        let (gutter_width, gutter_lines) = {
            let state = state_rc.borrow();
            compute_gutter_content(&state)
        };

        // Compute status bar content by dispatching "render-status" hook
        let status_content = {
            let state = state_rc.borrow();
            compute_status_content(&state)
        };

        // Update viewport dimensions to match actual available area.
        // Terminal height minus reserved rows (status bar + message line).
        {
            let mut state = state_rc.borrow_mut();
            state.viewport.gutter_width = gutter_width;
            let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
            let reserved_rows = {
                let mut r: u16 = 0;
                if status_content.is_some() {
                    r += 1; // status bar
                }
                if state.message.is_some() {
                    r += 1; // message line
                }
                r
            };
            state.viewport.height = term_height.saturating_sub(reserved_rows);
            state.viewport.width = term_width;
        }
        renderer::render_frame(
            &mut terminal,
            &state_rc.borrow(),
            &gutter_lines,
            status_content.as_deref(),
        )?;

        // Set terminal cursor shape based on current mode
        renderer::apply_cursor_shape(&state_rc.borrow())?;

        // Poll with timeout to avoid blocking forever if terminal doesn't deliver events
        if !ct_event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(ct_key) = ct_event::read()? {
            // Handle key press events. Accept both Press and unknown kinds
            // (some terminals don't support enhanced keyboard protocol).
            if ct_key.kind != KeyEventKind::Release {
                let key = convert_crossterm_key(ct_key);

                // Handle the key event (borrow state, then drop before deferred action)
                let deferred = {
                    let mut state = state_rc.borrow_mut();
                    let (new_input_state, action) = handle_key_event(&mut state, key, input_state);
                    input_state = new_input_state;
                    action
                }; // borrow dropped here

                // Execute deferred actions outside the borrow
                match deferred {
                    DeferredAction::Eval(expr) => {
                        eval_and_display(state_rc, runtime, &expr);
                    }
                    DeferredAction::ExecCommand(cmd_name) => {
                        // Extract handler from registry, dropping the borrow BEFORE calling.
                        let handler = {
                            let state = state_rc.borrow();
                            state.commands.extract_handler(&cmd_name)
                        }; // borrow dropped

                        state_rc.borrow_mut().message = None;

                        let result = match handler {
                            Some(alfred_core::command::ClonedHandler::Native(f)) => {
                                // Native handlers are plain fn pointers — safe to call with borrow
                                f(&mut state_rc.borrow_mut())
                            }
                            Some(alfred_core::command::ClonedHandler::Dynamic(f)) => {
                                // Dynamic (Lisp) handlers capture their own Rc<RefCell<EditorState>>
                                // and call borrow_mut() internally. We must NOT hold a borrow here.
                                // Pass a temporary EditorState that the closure ignores.
                                let mut dummy = alfred_core::editor_state::new(1, 1);
                                f(&mut dummy)
                            }
                            None => {
                                state_rc.borrow_mut().message =
                                    Some(format!("Unknown command: {}", cmd_name));
                                Ok(())
                            }
                        };
                        if let Err(e) = result {
                            state_rc.borrow_mut().message = Some(format!("Command error: {}", e));
                        }
                    }
                    DeferredAction::SaveBuffer(opt_path) => {
                        let mut state = state_rc.borrow_mut();
                        let save_path = match opt_path {
                            Some(ref p) => Some(std::path::PathBuf::from(p)),
                            None => state.buffer.file_path().map(|p| p.to_path_buf()),
                        };
                        match save_path {
                            Some(path) => {
                                match alfred_core::buffer::save_to_file(&state.buffer, &path) {
                                    Ok(saved_buffer) => {
                                        let byte_count =
                                            alfred_core::buffer::content(&saved_buffer).len();
                                        state.buffer = saved_buffer;
                                        state.message = Some(format!(
                                            "\"{}\" written, {} bytes",
                                            path.display(),
                                            byte_count
                                        ));
                                    }
                                    Err(e) => {
                                        state.message = Some(format!("{}", e));
                                    }
                                }
                            }
                            None => {
                                state.message = Some("No file name".to_string());
                            }
                        }
                    }
                    DeferredAction::OpenFile(ref path_str) => {
                        let path = std::path::Path::new(path_str);
                        match alfred_core::buffer::Buffer::from_file(path) {
                            Ok(new_buffer) => {
                                let mut state = state_rc.borrow_mut();
                                let filename =
                                    new_buffer.filename().unwrap_or(path_str).to_string();
                                state.buffer = new_buffer;
                                state.cursor = alfred_core::cursor::new(0, 0);
                                state.viewport.top_line = 0;
                                state.message = Some(format!("\"{}\"", filename));
                            }
                            Err(e) => {
                                state_rc.borrow_mut().message = Some(format!("{}", e));
                            }
                        }
                    }
                    DeferredAction::SaveAndQuit => {
                        let mut state = state_rc.borrow_mut();
                        let save_path = state.buffer.file_path().map(|p| p.to_path_buf());
                        match save_path {
                            Some(path) => {
                                match alfred_core::buffer::save_to_file(&state.buffer, &path) {
                                    Ok(saved_buffer) => {
                                        let byte_count =
                                            alfred_core::buffer::content(&saved_buffer).len();
                                        state.buffer = saved_buffer;
                                        state.message = Some(format!(
                                            "\"{}\" written, {} bytes",
                                            path.display(),
                                            byte_count
                                        ));
                                        state.running = false;
                                    }
                                    Err(e) => {
                                        state.message = Some(format!("{}", e));
                                    }
                                }
                            }
                            None => {
                                state.message = Some("No file name".to_string());
                            }
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

    /// Helper: set up standard keymaps with arrow keys and colon binding,
    /// plus register built-in native commands.
    /// Used by tests that rely on keymap-based key dispatch (06-02+).
    fn setup_standard_keymaps(state: &mut alfred_core::editor_state::EditorState) {
        use alfred_core::editor_state::Keymap;
        let mut keymap = Keymap::new();
        keymap.insert(KeyEvent::plain(KeyCode::Up), "cursor-up".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Down), "cursor-down".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Left), "cursor-left".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Right), "cursor-right".to_string());
        keymap.insert(
            KeyEvent::plain(KeyCode::Char(':')),
            "enter-command-mode".to_string(),
        );
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());
        alfred_core::editor_state::register_builtin_commands(state);
    }

    /// Helper: dispatch a key event through keymap lookup and execute any
    /// deferred command. Returns the new InputState.
    /// This replaces handle_key for tests that need full dispatch (cursor movement etc).
    fn dispatch_key(
        state: &mut alfred_core::editor_state::EditorState,
        key: KeyEvent,
        input_state: super::InputState,
    ) -> super::InputState {
        let (new_input_state, action) = super::handle_key_event(state, key, input_state);
        if let super::DeferredAction::ExecCommand(ref cmd_name) = action {
            let _ = alfred_core::command::execute(state, cmd_name);
        }
        new_input_state
    }

    // -----------------------------------------------------------------------
    // Acceptance test: simulate a sequence of key events on EditorState,
    // verifying cursor movement and running flag changes
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_with_multiline_buffer_when_key_events_dispatched_then_cursor_moves_and_quit_stops_running(
    ) {
        // Given: an EditorState with a 3-line buffer and standard keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld!\nBye");
        setup_standard_keymaps(&mut state);

        // Cursor starts at (0, 0), running is true
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
        assert!(state.running);

        // When: press Down arrow
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Down),
            super::InputState::Normal,
        );
        // Then: cursor moves to line 1
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);

        // When: press Right arrow twice
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Right),
            super::InputState::Normal,
        );
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Right),
            super::InputState::Normal,
        );
        // Then: cursor at (1, 2)
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 2);

        // When: press Up arrow
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Up),
            super::InputState::Normal,
        );
        // Then: cursor moves to line 0, column 2
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 2);

        // When: press Left arrow
        dispatch_key(
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
        let result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('q')), result);
        dispatch_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);
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
        setup_standard_keymaps(&mut state);
        assert_eq!(state.viewport.top_line, 0);

        // When: move cursor down 6 times (past the 5-line viewport)
        for _ in 0..6 {
            dispatch_key(
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
    fn given_crossterm_key_events_when_converted_then_returns_correct_alfred_key_events() {
        // Each tuple: (crossterm_code, crossterm_modifiers, expected_alfred_code, ctrl, alt, shift, label)
        let cases: Vec<(CtKeyCode, CtKeyModifiers, KeyCode, bool, bool, bool, &str)> = vec![
            (
                CtKeyCode::Up,
                CtKeyModifiers::NONE,
                KeyCode::Up,
                false,
                false,
                false,
                "Up",
            ),
            (
                CtKeyCode::Down,
                CtKeyModifiers::NONE,
                KeyCode::Down,
                false,
                false,
                false,
                "Down",
            ),
            (
                CtKeyCode::Left,
                CtKeyModifiers::NONE,
                KeyCode::Left,
                false,
                false,
                false,
                "Left",
            ),
            (
                CtKeyCode::Right,
                CtKeyModifiers::NONE,
                KeyCode::Right,
                false,
                false,
                false,
                "Right",
            ),
            (
                CtKeyCode::Enter,
                CtKeyModifiers::NONE,
                KeyCode::Enter,
                false,
                false,
                false,
                "Enter",
            ),
            (
                CtKeyCode::Esc,
                CtKeyModifiers::NONE,
                KeyCode::Escape,
                false,
                false,
                false,
                "Escape",
            ),
            (
                CtKeyCode::Backspace,
                CtKeyModifiers::NONE,
                KeyCode::Backspace,
                false,
                false,
                false,
                "Backspace",
            ),
            (
                CtKeyCode::Tab,
                CtKeyModifiers::NONE,
                KeyCode::Tab,
                false,
                false,
                false,
                "Tab",
            ),
            (
                CtKeyCode::Home,
                CtKeyModifiers::NONE,
                KeyCode::Home,
                false,
                false,
                false,
                "Home",
            ),
            (
                CtKeyCode::End,
                CtKeyModifiers::NONE,
                KeyCode::End,
                false,
                false,
                false,
                "End",
            ),
            (
                CtKeyCode::PageUp,
                CtKeyModifiers::NONE,
                KeyCode::PageUp,
                false,
                false,
                false,
                "PageUp",
            ),
            (
                CtKeyCode::PageDown,
                CtKeyModifiers::NONE,
                KeyCode::PageDown,
                false,
                false,
                false,
                "PageDown",
            ),
            (
                CtKeyCode::Delete,
                CtKeyModifiers::NONE,
                KeyCode::Delete,
                false,
                false,
                false,
                "Delete",
            ),
            (
                CtKeyCode::Char('q'),
                CtKeyModifiers::CONTROL,
                KeyCode::Char('q'),
                true,
                false,
                false,
                "Ctrl+Char",
            ),
            (
                CtKeyCode::Char('x'),
                CtKeyModifiers::ALT,
                KeyCode::Char('x'),
                false,
                true,
                false,
                "Alt+Char",
            ),
        ];

        for (ct_code, ct_mods, expected_code, ctrl, alt, shift, label) in &cases {
            let ct_event = make_crossterm_key(ct_code.clone(), *ct_mods);
            let result = super::convert_crossterm_key(ct_event);
            assert_eq!(result.code, *expected_code, "code mismatch for {}", label);
            assert_eq!(result.modifiers.ctrl, *ctrl, "ctrl mismatch for {}", label);
            assert_eq!(result.modifiers.alt, *alt, "alt mismatch for {}", label);
            assert_eq!(
                result.modifiers.shift, *shift,
                "shift mismatch for {}",
                label
            );
        }
    }

    // -----------------------------------------------------------------------
    // Unit tests: handle_key_event -- individual key behaviors
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_when_arrow_key_pressed_then_cursor_moves_in_that_direction() {
        // Each tuple: (buffer, start_line, start_col, key, expected_line, expected_col, label)
        let cases: Vec<(&str, usize, usize, KeyCode, usize, usize, &str)> = vec![
            (
                "aaa\nbbb\nccc",
                0,
                0,
                KeyCode::Down,
                1,
                0,
                "Down increases line",
            ),
            (
                "aaa\nbbb\nccc",
                2,
                0,
                KeyCode::Up,
                1,
                0,
                "Up decreases line",
            ),
            (
                "Hello",
                0,
                0,
                KeyCode::Right,
                0,
                1,
                "Right increases column",
            ),
            ("Hello", 0, 3, KeyCode::Left, 0, 2, "Left decreases column"),
        ];

        for (buffer_text, start_line, start_col, key, expected_line, expected_col, label) in &cases
        {
            let mut state = editor_state::new(80, 24);
            state.buffer = Buffer::from_string(buffer_text);
            setup_standard_keymaps(&mut state);
            state.cursor = cursor::new(*start_line, *start_col);

            dispatch_key(
                &mut state,
                KeyEvent::plain(key.clone()),
                super::InputState::Normal,
            );
            assert_eq!(
                state.cursor.line, *expected_line,
                "line mismatch for {}",
                label
            );
            assert_eq!(
                state.cursor.column, *expected_col,
                "col mismatch for {}",
                label
            );
        }
    }

    #[test]
    fn given_editor_when_colon_q_enter_then_running_becomes_false() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
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
        setup_standard_keymaps(&mut state);

        // Enter command mode
        let result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        assert!(matches!(result, super::InputState::Command(_)));

        // Type some chars
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('x')), result);

        // Escape cancels
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Escape), result);
        assert_eq!(result, super::InputState::Normal);
        assert!(state.running);
        assert_eq!(state.message, None);
    }

    #[test]
    fn given_editor_when_unknown_command_then_shows_error_message() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        // :foo Enter
        let result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('f')), result);
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('o')), result);
        let result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('o')), result);
        dispatch_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        assert!(state.running); // Did NOT quit
        assert_eq!(state.message, Some("Unknown command: foo".to_string()));
    }

    #[test]
    fn given_editor_when_quit_command_then_also_accepts_full_word() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        // :quit Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "quit".chars() {
            result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        dispatch_key(&mut state, KeyEvent::plain(KeyCode::Enter), result);
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
        setup_standard_keymaps(&mut state);
        assert_eq!(state.viewport.top_line, 0);

        // Move cursor past viewport bottom
        for _ in 0..4 {
            dispatch_key(
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
        {
            let mut state = state_rc.borrow_mut();
            setup_standard_keymaps(&mut state);
        }

        // And: a Lisp runtime with core primitives registered
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());

        // When: simulate typing `:eval (message "hi")` and pressing Enter
        let deferred = {
            let mut state = state_rc.borrow_mut();
            let mut result = dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Char(':')),
                super::InputState::Normal,
            );
            for c in "eval (message \"hi\")".chars() {
                result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
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
        setup_standard_keymaps(&mut state);

        // Type `:eval (+ 1 2)` and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "eval (+ 1 2)".chars() {
            result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
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

        // Given: runtime with bridge and keymaps
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            setup_standard_keymaps(&mut state);
        }
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
        setup_standard_keymaps(&mut state);
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

    // -----------------------------------------------------------------------
    // Acceptance test (04-04): line-numbers plugin produces gutter content
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_numbers_plugin_loaded_when_gutter_computed_then_gutter_contains_formatted_line_numbers_and_width_set(
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: an editor state with a 5-line buffer and viewport height=3
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 3)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("Line1\nLine2\nLine3\nLine4\nLine5");
        }

        // And: a Lisp runtime with core + hook primitives
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        // And: the line-numbers plugin is loaded (registers render-gutter hook)
        runtime
            .eval(r#"(add-hook "render-gutter" (lambda (start end total) start))"#)
            .unwrap();

        // When: compute_gutter_content is called with viewport info
        let (gutter_width, gutter_lines) = {
            let state = state_rc.borrow();
            super::compute_gutter_content(&state)
        };

        // Then: gutter_width is set based on digit count (5 lines -> 1 digit + 1 padding = 2)
        assert!(
            gutter_width > 0,
            "gutter_width should be > 0 when line-numbers plugin is loaded"
        );

        // And: gutter_lines contains formatted line numbers for visible lines
        assert!(
            !gutter_lines.is_empty(),
            "gutter_lines should not be empty when line-numbers plugin is loaded"
        );
        // First visible line should be "1" (right-aligned with padding)
        assert!(
            gutter_lines[0].contains("1"),
            "first gutter line should contain '1', got: '{}'",
            gutter_lines[0]
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests (04-04): gutter content computation
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_render_gutter_hook_when_gutter_computed_then_empty_gutter_and_zero_width() {
        // Given: an editor state with buffer but no hooks registered
        let mut state = editor_state::new(80, 5);
        state.buffer = Buffer::from_string("Line1\nLine2\nLine3");

        // When: compute_gutter_content is called
        let (gutter_width, gutter_lines) = super::compute_gutter_content(&state);

        // Then: gutter_width is 0 and gutter_lines is empty
        assert_eq!(gutter_width, 0, "no hook means gutter_width should be 0");
        assert!(
            gutter_lines.is_empty(),
            "no hook means gutter_lines should be empty"
        );
    }

    #[test]
    fn given_render_gutter_hook_when_gutter_computed_then_gutter_width_matches_digit_count() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: a buffer with 1000+ lines (4 digits) and a registered hook
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 5)));
        {
            let mut state = state_rc.borrow_mut();
            let lines: Vec<&str> = (0..1050).map(|_| "x").collect();
            state.buffer = Buffer::from_string(&lines.join("\n"));
        }

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        // Register a simple hook that returns something
        runtime
            .eval(r#"(add-hook "render-gutter" (lambda (start end total) start))"#)
            .unwrap();

        // When: compute_gutter_content is called
        let (gutter_width, _gutter_lines) = {
            let state = state_rc.borrow();
            super::compute_gutter_content(&state)
        };

        // Then: gutter_width accommodates 4 digits + 1 padding = 5
        assert_eq!(
            gutter_width, 5,
            "1050 lines need 4 digits + 1 padding = gutter_width 5"
        );
    }

    #[test]
    fn given_render_gutter_hook_when_viewport_scrolled_then_gutter_shows_correct_line_numbers() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: a 10-line buffer with viewport scrolled to top_line=5, height=3
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 3)));
        {
            let mut state = state_rc.borrow_mut();
            let lines: Vec<String> = (0..10).map(|i| format!("Line{}", i)).collect();
            state.buffer = Buffer::from_string(&lines.join("\n"));
            state.viewport.top_line = 5;
        }

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        // Register hook that returns the start line (simulating awareness of scroll)
        runtime
            .eval(r#"(add-hook "render-gutter" (lambda (start end total) start))"#)
            .unwrap();

        // When: compute_gutter_content is called
        let (_gutter_width, gutter_lines) = {
            let state = state_rc.borrow();
            super::compute_gutter_content(&state)
        };

        // Then: gutter lines should show line numbers starting from 6 (top_line=5, 1-indexed)
        assert!(
            !gutter_lines.is_empty(),
            "gutter should have lines when hook registered"
        );
        assert!(
            gutter_lines[0].contains("6"),
            "first visible line should be 6 (0-indexed line 5), got: '{}'",
            gutter_lines[0]
        );
    }

    #[test]
    fn given_small_buffer_when_gutter_computed_then_gutter_width_is_minimal() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: a buffer with 3 lines (1 digit)
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 5)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("A\nB\nC");
        }

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-gutter" (lambda (start end total) start))"#)
            .unwrap();

        // When: compute_gutter_content is called
        let (gutter_width, _gutter_lines) = {
            let state = state_rc.borrow();
            super::compute_gutter_content(&state)
        };

        // Then: gutter_width = 1 digit + 1 padding = 2
        assert_eq!(
            gutter_width, 2,
            "3 lines need 1 digit + 1 padding = gutter_width 2"
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (05-03): status-bar plugin produces status content
    // -----------------------------------------------------------------------

    #[test]
    fn given_status_bar_plugin_loaded_when_status_computed_then_status_contains_filename_and_cursor_position(
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: an editor state with a buffer loaded from a file
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let dir = std::env::temp_dir();
            let file_path = dir.join("test_status.txt");
            std::fs::write(&file_path, "Hello\nWorld").unwrap();
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_file(&file_path).unwrap();
            // Move cursor to line 1, col 3
            state.cursor = cursor::new(1, 3);
        }

        // And: a Lisp runtime with core + hook primitives
        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        // And: the status-bar plugin is loaded (registers render-status hook)
        runtime
            .eval(r#"(add-hook "render-status" (lambda () "status-bar-active"))"#)
            .unwrap();

        // When: compute_status_content is called
        let status = {
            let state = state_rc.borrow();
            super::compute_status_content(&state)
        };

        // Then: status is Some and contains filename and cursor position
        assert!(status.is_some(), "status should be Some when plugin loaded");
        let status_str = status.unwrap();
        assert!(
            status_str.contains("test_status.txt"),
            "status should contain filename, got: '{}'",
            status_str
        );
        assert!(
            status_str.contains("Ln 2") && status_str.contains("Col 3"),
            "status should contain cursor position (1-indexed line), got: '{}'",
            status_str
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests (05-03): status bar content computation
    // Test Budget: 6 behaviors x 2 = 12 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_render_status_hook_when_status_computed_then_returns_none() {
        // Given: an editor state with no hooks registered
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");

        // When: compute_status_content is called
        let status = super::compute_status_content(&state);

        // Then: returns None (no status bar)
        assert!(status.is_none(), "no hook means no status bar");
    }

    #[test]
    fn given_status_hook_and_no_filename_when_status_computed_then_shows_no_name() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: buffer with no filename and render-status hook registered
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        // When: compute_status_content is called
        let status = {
            let state = state_rc.borrow();
            super::compute_status_content(&state)
        };

        // Then: status contains "[No Name]"
        let status_str = status.unwrap();
        assert!(
            status_str.contains("[No Name]"),
            "unnamed buffer should show [No Name], got: '{}'",
            status_str
        );
    }

    #[test]
    fn given_status_hook_and_modified_buffer_when_status_computed_then_shows_modified_indicator() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: a modified buffer with render-status hook registered
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            // insert_at marks the buffer as modified
            state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 0, "x");
        }

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        // When: compute_status_content is called
        let status = {
            let state = state_rc.borrow();
            super::compute_status_content(&state)
        };

        // Then: status contains "[+]"
        let status_str = status.unwrap();
        assert!(
            status_str.contains("[+]"),
            "modified buffer should show [+], got: '{}'",
            status_str
        );
    }

    #[test]
    fn given_status_hook_and_unmodified_buffer_when_status_computed_then_no_modified_indicator() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: an unmodified buffer with render-status hook registered
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        // When: compute_status_content is called
        let status = {
            let state = state_rc.borrow();
            super::compute_status_content(&state)
        };

        // Then: status does not contain "[+]"
        let status_str = status.unwrap();
        assert!(
            !status_str.contains("[+]"),
            "unmodified buffer should not show [+], got: '{}'",
            status_str
        );
    }

    #[test]
    fn given_status_hook_when_status_computed_then_shows_mode_name() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor in normal mode with render-status hook registered
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        // When: compute_status_content is called
        let status = {
            let state = state_rc.borrow();
            super::compute_status_content(&state)
        };

        // Then: status contains mode name "NORMAL" (uppercased for display)
        let status_str = status.unwrap();
        assert!(
            status_str.contains("NORMAL"),
            "status should contain mode name, got: '{}'",
            status_str
        );
    }

    #[test]
    fn given_status_hook_when_cursor_moved_then_status_reflects_new_position() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with multiline buffer and render-status hook
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("Hello\nWorld\nBye");
        }

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, state_rc.clone());
        alfred_lisp::bridge::register_hook_primitives(&runtime, state_rc.clone());

        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        // When: cursor is at (0, 0)
        let status_at_origin = {
            let state = state_rc.borrow();
            super::compute_status_content(&state).unwrap()
        };

        // Then: shows Ln 1, Col 0
        assert!(
            status_at_origin.contains("Ln 1") && status_at_origin.contains("Col 0"),
            "cursor at origin should show Ln 1, Col 0, got: '{}'",
            status_at_origin
        );

        // When: cursor moves to (2, 1)
        {
            let mut state = state_rc.borrow_mut();
            state.cursor = cursor::new(2, 1);
        }
        let status_after_move = {
            let state = state_rc.borrow();
            super::compute_status_content(&state).unwrap()
        };

        // Then: shows Ln 3, Col 1
        assert!(
            status_after_move.contains("Ln 3") && status_after_move.contains("Col 1"),
            "cursor at (2,1) should show Ln 3, Col 1, got: '{}'",
            status_after_move
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (06-02): keymap-based key dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn given_keymap_with_up_binding_when_up_pressed_then_returns_exec_command_cursor_up() {
        use alfred_core::editor_state::Keymap;

        // Given: an EditorState with a keymap binding Up -> "cursor-up"
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld\nBye");

        let mut keymap = Keymap::new();
        keymap.insert(KeyEvent::plain(KeyCode::Up), "cursor-up".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Down), "cursor-down".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Left), "cursor-left".to_string());
        keymap.insert(KeyEvent::plain(KeyCode::Right), "cursor-right".to_string());
        keymap.insert(
            KeyEvent::plain(KeyCode::Char(':')),
            "enter-command-mode".to_string(),
        );
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());

        // When: Up key pressed in Normal mode
        let (_input_state, action) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Up),
            super::InputState::Normal,
        );

        // Then: returns ExecCommand("cursor-up")
        assert_eq!(
            action,
            super::DeferredAction::ExecCommand("cursor-up".to_string()),
            "keymap lookup should resolve Up to cursor-up command"
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests (06-02): keymap-based dispatch behaviors
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_keymap_when_unbound_key_pressed_then_no_action_no_error() {
        use alfred_core::editor_state::Keymap;

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");

        // Keymap with only Up bound
        let mut keymap = Keymap::new();
        keymap.insert(KeyEvent::plain(KeyCode::Up), "cursor-up".to_string());
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());

        let cursor_before = state.cursor;

        // When: Tab key pressed (not in keymap)
        let (input_state, action) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Tab),
            super::InputState::Normal,
        );

        // Then: no action, state unchanged
        assert_eq!(action, super::DeferredAction::None);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(state.cursor, cursor_before);
    }

    #[test]
    fn given_keymap_with_colon_binding_when_colon_pressed_then_enters_command_mode() {
        use alfred_core::editor_state::Keymap;

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");

        let mut keymap = Keymap::new();
        keymap.insert(
            KeyEvent::plain(KeyCode::Char(':')),
            "enter-command-mode".to_string(),
        );
        state.keymaps.insert("global".to_string(), keymap);
        state.active_keymaps.push("global".to_string());

        // When: colon pressed in Normal mode
        let (input_state, action) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );

        // Then: enters Command mode (same behavior as before, via keymap)
        assert!(
            matches!(input_state, super::InputState::Command(_)),
            "colon via keymap should enter command mode"
        );
        assert_eq!(state.message, Some(":".to_string()));
        assert_eq!(action, super::DeferredAction::None);
    }

    #[test]
    fn given_no_keymaps_when_key_pressed_in_normal_mode_then_falls_through_silently() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld");
        let cursor_before = state.cursor;

        // No keymaps configured at all
        let (input_state, action) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Up),
            super::InputState::Normal,
        );

        // Then: no action, no crash
        assert_eq!(action, super::DeferredAction::None);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(state.cursor, cursor_before);
    }

    // -----------------------------------------------------------------------
    // Unit tests (06-03): self-insert and delete-backward behaviors
    // Test Budget: 3 behaviors x 2 = 6 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_active_keymaps_when_unbound_printable_char_pressed_then_char_inserted_and_cursor_advances(
    ) {
        // Given: editor in insert mode with active keymaps (simulating basic-keybindings loaded)
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 5);
        state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
        setup_standard_keymaps(&mut state);

        // When: press 'x' (not bound in keymap)
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char('x')),
            super::InputState::Normal,
        );

        // Then: 'x' is inserted at cursor position and cursor advances
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content, "Hellox",
            "Unbound char 'x' should be inserted at cursor"
        );
        assert_eq!(
            state.cursor.column, 6,
            "Cursor should advance after self-insert"
        );
    }

    #[test]
    fn given_active_keymaps_when_unbound_non_printable_key_pressed_then_no_insert() {
        // Given: editor with active keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        let cursor_before = state.cursor;
        setup_standard_keymaps(&mut state);

        // When: press Tab (not bound, not a printable char)
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Tab),
            super::InputState::Normal,
        );

        // Then: no insertion, cursor unchanged
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content, "Hello",
            "Non-printable unbound key should not insert"
        );
        assert_eq!(state.cursor, cursor_before, "Cursor should not move");
    }

    #[test]
    fn given_delete_backward_command_when_executed_then_char_before_cursor_deleted_and_cursor_moves_back(
    ) {
        // Given: editor with buffer "Hello" and cursor at column 5 (end)
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 5);
        editor_state::register_builtin_commands(&mut state);

        // When: execute delete-backward command
        let result = alfred_core::command::execute(&mut state, "delete-backward");

        // Then: 'o' is deleted, buffer is "Hell", cursor moves to column 4
        assert!(result.is_ok(), "delete-backward should succeed");
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(content, "Hell", "Character before cursor should be deleted");
        assert_eq!(state.cursor.column, 4, "Cursor should move back one column");
    }

    #[test]
    fn given_delete_backward_at_beginning_of_buffer_when_executed_then_nothing_happens() {
        // Given: editor with cursor at (0, 0)
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // When: execute delete-backward at beginning
        let result = alfred_core::command::execute(&mut state, "delete-backward");

        // Then: buffer unchanged, cursor unchanged
        assert!(result.is_ok());
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(content, "Hello", "Nothing should be deleted at beginning");
        assert_eq!(state.cursor.column, 0);
        assert_eq!(state.cursor.line, 0);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (06-03): basic-keybindings plugin behaviors
    // -----------------------------------------------------------------------

    /// Helper: set up runtime with all bridge primitives and register builtin commands,
    /// then evaluate basic-keybindings Lisp expressions to configure keymaps.
    fn setup_basic_keybindings_via_lisp(
        state_rc: &std::rc::Rc<std::cell::RefCell<alfred_core::editor_state::EditorState>>,
    ) -> alfred_lisp::runtime::LispRuntime {
        use std::rc::Rc;

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_define_command(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_keymap_primitives(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_hook_primitives(&runtime, Rc::clone(state_rc));

        // Register native commands (cursor-up/down/left/right, delete-backward)
        {
            let mut state = state_rc.borrow_mut();
            editor_state::register_builtin_commands(&mut state);
        }

        // Evaluate the same Lisp that basic-keybindings/init.lisp would contain
        let lisp_code = r#"
            (make-keymap "global")
            (define-key "global" "Up" "cursor-up")
            (define-key "global" "Down" "cursor-down")
            (define-key "global" "Left" "cursor-left")
            (define-key "global" "Right" "cursor-right")
            (define-key "global" "Char::" "enter-command-mode")
            (define-key "global" "Backspace" "delete-backward")
            (set-active-keymap "global")
        "#;
        for line in lisp_code.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                runtime
                    .eval(trimmed)
                    .unwrap_or_else(|e| panic!("Lisp eval failed for '{}': {}", trimmed, e));
            }
        }

        runtime
    }

    #[test]
    fn given_basic_keybindings_loaded_when_key_events_sent_then_arrows_navigate_chars_insert_backspace_deletes_colon_enters_command_mode(
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with multiline buffer and basic-keybindings loaded via Lisp
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("Hello\nWorld");
        }
        let _runtime = setup_basic_keybindings_via_lisp(&state_rc);

        // AC1: Arrow keys navigate via plugin-defined bindings
        {
            let mut state = state_rc.borrow_mut();
            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Down),
                super::InputState::Normal,
            );
            assert_eq!(
                state.cursor.line, 1,
                "Down arrow should move cursor to line 1"
            );
            assert_eq!(state.cursor.column, 0);

            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Right),
                super::InputState::Normal,
            );
            assert_eq!(
                state.cursor.column, 1,
                "Right arrow should move cursor to column 1"
            );

            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Up),
                super::InputState::Normal,
            );
            assert_eq!(
                state.cursor.line, 0,
                "Up arrow should move cursor to line 0"
            );

            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Left),
                super::InputState::Normal,
            );
            assert_eq!(
                state.cursor.column, 0,
                "Left arrow should move cursor to column 0"
            );
        }

        // AC2: Character insertion works for printable keys (unbound char auto-insert)
        // Self-insert only fires in insert mode.
        {
            let mut state = state_rc.borrow_mut();
            state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
            state.cursor = cursor::new(0, 5); // end of "Hello"
            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Char('!')),
                super::InputState::Normal,
            );
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                content.starts_with("Hello!"),
                "Unbound printable char '!' should be inserted at cursor, got: '{}'",
                content
            );
            assert_eq!(state.cursor.column, 6, "Cursor should advance after insert");
        }

        // AC3: Backspace deletes character before cursor
        {
            let mut state = state_rc.borrow_mut();
            // cursor is at column 6 (after "Hello!"), backspace should delete '!'
            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Backspace),
                super::InputState::Normal,
            );
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                content.starts_with("Hello\n"),
                "Backspace should delete char before cursor, got: '{}'",
                content
            );
            assert_eq!(
                state.cursor.column, 5,
                "Cursor should move back after backspace"
            );
        }

        // AC4: Colon enters command mode via plugin binding
        {
            let mut state = state_rc.borrow_mut();
            let (input_state, _action) = super::handle_key_event(
                &mut state,
                KeyEvent::plain(KeyCode::Char(':')),
                super::InputState::Normal,
            );
            assert!(
                matches!(input_state, super::InputState::Command(_)),
                "Colon should enter command mode"
            );
            assert_eq!(state.message, Some(":".to_string()));
        }
    }

    // -----------------------------------------------------------------------
    // Unit tests (06-04): no keymaps means no key dispatch
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_keymaps_when_multiple_arrow_keys_pressed_then_cursor_stays_at_origin() {
        // Given: editor with multiline buffer but no keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld\nBye");
        assert!(state.active_keymaps.is_empty());

        // When: all four arrow keys pressed
        for key_code in &[KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            let (input_state, action) = super::handle_key_event(
                &mut state,
                KeyEvent::plain(*key_code),
                super::InputState::Normal,
            );
            assert_eq!(action, super::DeferredAction::None);
            assert_eq!(input_state, super::InputState::Normal);
        }

        // Then: cursor remains at origin
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn given_no_active_keymaps_when_printable_char_pressed_then_no_self_insert() {
        // Given: editor with no active keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        assert!(state.active_keymaps.is_empty());
        let content_before = alfred_core::buffer::content(&state.buffer);

        // When: printable character pressed
        let (_, action) = super::handle_key_event(
            &mut state,
            KeyEvent::plain(KeyCode::Char('x')),
            super::InputState::Normal,
        );

        // Then: no self-insert, buffer unchanged
        assert_eq!(action, super::DeferredAction::None);
        assert_eq!(alfred_core::buffer::content(&state.buffer), content_before);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (06-04): without keymaps, editor starts but keys do nothing
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_with_no_keymaps_when_all_key_types_pressed_then_cursor_unchanged_and_buffer_unchanged(
    ) {
        // Given: an editor with a multiline buffer but NO keymaps loaded (no plugin)
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello\nWorld\nBye");
        // Deliberately do NOT call setup_standard_keymaps -- no plugin loaded
        assert!(
            state.active_keymaps.is_empty(),
            "No keymaps should be active"
        );
        assert!(state.running, "Editor should start running");

        let cursor_before = state.cursor;
        let buffer_content_before = alfred_core::buffer::content(&state.buffer);

        // When: press all four arrow keys
        for key_code in &[KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            dispatch_key(
                &mut state,
                KeyEvent::plain(*key_code),
                super::InputState::Normal,
            );
        }

        // And: press printable characters
        for ch in &['a', 'z', '!', ' '] {
            dispatch_key(
                &mut state,
                KeyEvent::plain(KeyCode::Char(*ch)),
                super::InputState::Normal,
            );
        }

        // And: press colon (which would enter command mode if bound)
        let input_state = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );

        // Then: cursor has not moved
        assert_eq!(
            state.cursor, cursor_before,
            "Without keymaps, cursor should not move for any key"
        );

        // And: buffer content is unchanged (no self-insert without active keymaps)
        let buffer_content_after = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            buffer_content_after, buffer_content_before,
            "Without keymaps, buffer should not change"
        );

        // And: we are still in Normal mode (colon did not enter command mode)
        assert_eq!(
            input_state,
            super::InputState::Normal,
            "Without keymaps, colon should not enter command mode"
        );

        // And: editor is still running (no quit occurred)
        assert!(state.running, "Editor should still be running");
    }

    // -----------------------------------------------------------------------
    // Unit tests (07-01): self-insert is mode-aware
    // -----------------------------------------------------------------------

    #[test]
    fn given_insert_mode_with_active_keymaps_when_unbound_printable_char_then_char_inserted() {
        // Given: editor in insert mode with active keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 5);
        state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
        setup_standard_keymaps(&mut state);

        // When: press 'x' (not bound in keymap)
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char('x')),
            super::InputState::Normal,
        );

        // Then: 'x' is inserted
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content, "Hellox",
            "Insert mode should self-insert unbound chars"
        );
        assert_eq!(state.cursor.column, 6);
    }

    #[test]
    fn given_normal_mode_with_active_keymaps_when_unbound_printable_char_then_no_insert() {
        // Given: editor in normal mode with active keymaps
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("Hello");
        state.cursor = cursor::new(0, 5);
        // mode defaults to "normal" from new()
        setup_standard_keymaps(&mut state);
        let content_before = alfred_core::buffer::content(&state.buffer);

        // When: press 'x' (not bound in keymap)
        dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char('x')),
            super::InputState::Normal,
        );

        // Then: no insertion in normal mode
        let content_after = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content_after, content_before,
            "Normal mode should NOT self-insert unbound chars"
        );
    }

    // -----------------------------------------------------------------------
    // Vim plugin helpers (07-03): dispatch through Rc<RefCell<EditorState>>
    // to support Lisp-registered Dynamic commands (enter-insert-mode, etc.)
    // -----------------------------------------------------------------------

    /// Helper: set up runtime with all bridge primitives, register builtin commands,
    /// and load the vim-keybindings plugin, then also register a render-status hook.
    fn setup_vim_keybindings_via_lisp(
        state_rc: &std::rc::Rc<std::cell::RefCell<alfred_core::editor_state::EditorState>>,
    ) -> alfred_lisp::runtime::LispRuntime {
        use std::rc::Rc;

        let runtime = alfred_lisp::runtime::LispRuntime::new();
        alfred_lisp::bridge::register_core_primitives(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_define_command(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_keymap_primitives(&runtime, Rc::clone(state_rc));
        alfred_lisp::bridge::register_hook_primitives(&runtime, Rc::clone(state_rc));

        // Register native commands (cursor-up/down/left/right, delete-backward, etc.)
        {
            let mut state = state_rc.borrow_mut();
            editor_state::register_builtin_commands(&mut state);
        }

        // Load the actual vim-keybindings plugin
        let plugin_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("plugins/vim-keybindings/init.lisp"),
        )
        .expect("vim-keybindings plugin should exist");
        runtime.eval(&plugin_source).unwrap();

        // Register a render-status hook so compute_status_content works
        runtime
            .eval(r#"(add-hook "render-status" (lambda () "active"))"#)
            .unwrap();

        runtime
    }

    /// Helper: dispatch a key event through the Rc<RefCell<EditorState>> path,
    /// replicating the real event loop pattern for both Native and Dynamic commands.
    /// This correctly handles Lisp-registered commands (like enter-insert-mode)
    /// which internally borrow the Rc<RefCell<EditorState>>.
    fn dispatch_key_rc(
        state_rc: &std::rc::Rc<std::cell::RefCell<alfred_core::editor_state::EditorState>>,
        key: KeyEvent,
        input_state: super::InputState,
    ) -> super::InputState {
        let (new_input_state, action) = {
            let mut state = state_rc.borrow_mut();
            super::handle_key_event(&mut state, key, input_state)
        }; // borrow dropped before deferred action

        if let super::DeferredAction::ExecCommand(ref cmd_name) = action {
            let handler = {
                let state = state_rc.borrow();
                state.commands.extract_handler(cmd_name)
            }; // borrow dropped

            match handler {
                Some(alfred_core::command::ClonedHandler::Native(f)) => {
                    let _ = f(&mut state_rc.borrow_mut());
                }
                Some(alfred_core::command::ClonedHandler::Dynamic(f)) => {
                    // Dynamic (Lisp) handlers capture their own Rc<RefCell<EditorState>>
                    // and call borrow_mut() internally. Pass a dummy state.
                    let mut dummy = alfred_core::editor_state::new(1, 1);
                    let _ = f(&mut dummy);
                }
                None => {}
            }
        }

        new_input_state
    }

    // -----------------------------------------------------------------------
    // Acceptance test (07-03): vim insert mode full workflow
    // -----------------------------------------------------------------------

    #[test]
    fn given_vim_plugin_loaded_when_i_pressed_then_type_chars_then_escape_then_buffer_changed_and_mode_restored(
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with buffer "Hello", cursor at end, and vim-keybindings loaded
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("Hello");
            state.cursor = cursor::new(0, 5); // at end of "Hello"
        }
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // Verify starting state: normal mode, normal-mode keymap active
        {
            let state = state_rc.borrow();
            assert_eq!(state.mode, "normal", "Should start in normal mode");
            assert_eq!(
                state.active_keymaps,
                vec!["normal-mode".to_string()],
                "Should have normal-mode keymap active"
            );
        }

        // When: press 'i' to enter insert mode
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // Then: mode is "insert" and active keymap is "insert-mode"
        {
            let state = state_rc.borrow();
            assert_eq!(state.mode, "insert", "After 'i', mode should be insert");
            assert_eq!(
                state.active_keymaps,
                vec!["insert-mode".to_string()],
                "After 'i', active keymap should be insert-mode"
            );
        }

        // And: status bar shows INSERT
        {
            let state = state_rc.borrow();
            let status = super::compute_status_content(&state).unwrap();
            assert!(
                status.contains("INSERT"),
                "Status bar should show INSERT in insert mode, got: '{}'",
                status
            );
        }

        // When: type " World" (characters should self-insert)
        let mut is = super::InputState::Normal;
        for ch in " World".chars() {
            is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char(ch)), is);
        }

        // Then: buffer contains "Hello World"
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert_eq!(
                content, "Hello World",
                "Typed chars should be inserted, got: '{}'",
                content
            );
        }

        // When: press Backspace to delete the 'd'
        is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Backspace), is);

        // Then: buffer is "Hello Worl"
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert_eq!(
                content, "Hello Worl",
                "Backspace should delete last char, got: '{}'",
                content
            );
        }

        // When: press Escape to return to normal mode
        dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Escape), is);

        // Then: mode is back to "normal" and active keymap is "normal-mode"
        {
            let state = state_rc.borrow();
            assert_eq!(state.mode, "normal", "After Escape, mode should be normal");
            assert_eq!(
                state.active_keymaps,
                vec!["normal-mode".to_string()],
                "After Escape, active keymap should be normal-mode"
            );
        }

        // And: status bar shows NORMAL
        {
            let state = state_rc.borrow();
            let status = super::compute_status_content(&state).unwrap();
            assert!(
                status.contains("NORMAL"),
                "Status bar should show NORMAL after escape, got: '{}'",
                status
            );
        }

        // And: typing characters in normal mode does NOT insert
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('z')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert_eq!(
                content, "Hello Worl",
                "Normal mode should NOT self-insert chars, got: '{}'",
                content
            );
        }
    }

    // -----------------------------------------------------------------------
    // Unit tests (07-03): vim insert mode behaviors
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_vim_normal_mode_when_i_pressed_then_mode_switches_to_insert_and_keymap_updated() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with vim plugin loaded, in normal mode
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // When: press 'i'
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // Then: mode is insert and keymap is insert-mode
        let state = state_rc.borrow();
        assert_eq!(state.mode, "insert");
        assert_eq!(state.active_keymaps, vec!["insert-mode".to_string()]);
    }

    #[test]
    fn given_vim_insert_mode_when_escape_pressed_then_mode_switches_to_normal_and_keymap_updated() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with vim plugin loaded, switched to insert mode
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // Enter insert mode first
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // When: press Escape
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Escape),
            super::InputState::Normal,
        );

        // Then: mode is normal and keymap is normal-mode
        let state = state_rc.borrow();
        assert_eq!(state.mode, "normal");
        assert_eq!(state.active_keymaps, vec!["normal-mode".to_string()]);
    }

    #[test]
    fn given_vim_insert_mode_when_chars_typed_then_chars_inserted_in_buffer() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with vim plugin loaded, in insert mode, buffer "AB"
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("AB");
            state.cursor = cursor::new(0, 2); // end of "AB"
        }
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // Enter insert mode
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // When: type "CD"
        let is = dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('C')),
            super::InputState::Normal,
        );
        dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char('D')), is);

        // Then: buffer is "ABCD"
        let state = state_rc.borrow();
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content, "ABCD",
            "Characters should be inserted in insert mode"
        );
        assert_eq!(
            state.cursor.column, 4,
            "Cursor should advance after each insert"
        );
    }

    #[test]
    fn given_vim_insert_mode_when_backspace_pressed_then_char_deleted() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with vim plugin loaded, in insert mode, buffer "Hello"
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("Hello");
            state.cursor = cursor::new(0, 5);
        }
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // Enter insert mode
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // When: press Backspace
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Backspace),
            super::InputState::Normal,
        );

        // Then: last character deleted, cursor moves back
        let state = state_rc.borrow();
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(
            content, "Hell",
            "Backspace should delete char before cursor"
        );
        assert_eq!(
            state.cursor.column, 4,
            "Cursor should move back after backspace"
        );
    }

    #[test]
    fn given_vim_insert_mode_when_status_computed_then_shows_insert_mode() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Given: editor with vim plugin loaded, switched to insert mode
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // Enter insert mode
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );

        // When: compute status
        let state = state_rc.borrow();
        let status = super::compute_status_content(&state);

        // Then: status shows INSERT
        let status_str = status.expect("Status should be present with render-status hook");
        assert!(
            status_str.contains("INSERT"),
            "Status should show INSERT in insert mode, got: '{}'",
            status_str
        );
    }

    // -----------------------------------------------------------------------
    // Capstone integration test (07-04): full modal editing workflow
    // Proves the architecture thesis: a complex, stateful feature (vim modal
    // editing) works entirely as a Lisp plugin with zero hardcoded key handling.
    // -----------------------------------------------------------------------

    #[test]
    fn given_vim_plugin_loaded_when_full_modal_editing_workflow_then_all_modes_and_commands_work_via_plugin(
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // ---- Setup: multi-line buffer with vim-keybindings plugin ----
        let state_rc = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut state = state_rc.borrow_mut();
            state.buffer = Buffer::from_string("First line\nSecond line\nThird line");
            state.cursor = cursor::new(0, 0);
        }
        let _runtime = setup_vim_keybindings_via_lisp(&state_rc);

        // ---- Step 1: Verify initial state (normal mode) ----
        {
            let state = state_rc.borrow();
            assert_eq!(state.mode, "normal", "Should start in normal mode");
            assert_eq!(
                state.active_keymaps,
                vec!["normal-mode".to_string()],
                "Should have normal-mode keymap active"
            );
            let status = super::compute_status_content(&state).unwrap();
            assert!(
                status.contains("NORMAL"),
                "Status should show NORMAL, got: '{}'",
                status
            );
        }

        // ---- Step 2: hjkl navigation in normal mode ----
        // j moves down
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('j')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            assert_eq!(state.cursor.line, 1, "j should move cursor down to line 1");
            assert_eq!(state.cursor.column, 0);
        }

        // l moves right (3 times to reach column 3)
        let mut is = super::InputState::Normal;
        for _ in 0..3 {
            is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char('l')), is);
        }
        {
            let state = state_rc.borrow();
            assert_eq!(
                state.cursor.column, 3,
                "l x3 should move cursor to column 3"
            );
            assert_eq!(state.cursor.line, 1);
        }

        // k moves up
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('k')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            assert_eq!(state.cursor.line, 0, "k should move cursor up to line 0");
            assert_eq!(state.cursor.column, 3);
        }

        // h moves left
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('h')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            assert_eq!(
                state.cursor.column, 2,
                "h should move cursor left to column 2"
            );
            assert_eq!(state.cursor.line, 0);
        }

        // ---- Step 3: Enter insert mode, type text, verify buffer ----
        // Move to end of first line first
        {
            let mut state = state_rc.borrow_mut();
            state.cursor = cursor::new(0, 10); // end of "First line"
        }

        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('i')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            assert_eq!(state.mode, "insert", "After 'i', mode should be insert");
            assert_eq!(
                state.active_keymaps,
                vec!["insert-mode".to_string()],
                "After 'i', keymap should be insert-mode"
            );
            let status = super::compute_status_content(&state).unwrap();
            assert!(
                status.contains("INSERT"),
                "Status should show INSERT, got: '{}'",
                status
            );
        }

        // Type " added" in insert mode
        is = super::InputState::Normal;
        for ch in " added".chars() {
            is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char(ch)), is);
        }
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                content.starts_with("First line added"),
                "Typed text should appear in buffer, got: '{}'",
                content
            );
        }

        // ---- Step 4: Press Escape to return to normal mode ----
        dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Escape), is);
        {
            let state = state_rc.borrow();
            assert_eq!(
                state.mode, "normal",
                "After Escape, should be in normal mode"
            );
            assert_eq!(
                state.active_keymaps,
                vec!["normal-mode".to_string()],
                "After Escape, keymap should be normal-mode"
            );
            let status = super::compute_status_content(&state).unwrap();
            assert!(
                status.contains("NORMAL"),
                "Status should show NORMAL after Escape, got: '{}'",
                status
            );
        }

        // Verify: typing in normal mode does NOT insert
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('z')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                !content.contains('z'),
                "Normal mode should not self-insert, got: '{}'",
                content
            );
        }

        // ---- Step 5: Navigate to second line and delete it with d ----
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('j')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            assert_eq!(
                state.cursor.line, 1,
                "j should move to line 1 (Second line)"
            );
        }

        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('d')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                !content.contains("Second line"),
                "d should delete 'Second line', got: '{}'",
                content
            );
            assert!(
                content.contains("First line added"),
                "First line should remain, got: '{}'",
                content
            );
            assert!(
                content.contains("Third line"),
                "Third line should remain, got: '{}'",
                content
            );
        }

        // ---- Step 6: Delete char at cursor with x ----
        // Cursor should be on "Third line" now (line 1 after deletion).
        // Move to column 0 to target 'T'.
        {
            let mut state = state_rc.borrow_mut();
            state.cursor = cursor::new(state.cursor.line, 0);
        }
        dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char('x')),
            super::InputState::Normal,
        );
        {
            let state = state_rc.borrow();
            let content = alfred_core::buffer::content(&state.buffer);
            assert!(
                content.contains("hird line"),
                "x should delete char at cursor ('T'), got: '{}'",
                content
            );
            assert!(
                !content.contains("Third line"),
                "Original 'Third line' should have 'T' removed, got: '{}'",
                content
            );
        }

        // ---- Step 7: Use : to enter command mode, type :q! to force quit ----
        // Buffer is modified (text inserted, lines deleted), so :q would warn.
        // Use :q! to force quit without saving.
        is = dispatch_key_rc(
            &state_rc,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        assert!(
            matches!(is, super::InputState::Command(_)),
            "Colon should enter command mode"
        );
        {
            let state = state_rc.borrow();
            assert_eq!(state.message, Some(":".to_string()));
        }

        // Type 'q!' and press Enter
        is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char('q')), is);
        is = dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Char('!')), is);
        dispatch_key_rc(&state_rc, KeyEvent::plain(KeyCode::Enter), is);
        {
            let state = state_rc.borrow();
            assert!(!state.running, ":q! should force quit the editor");
        }
    }

    // -----------------------------------------------------------------------
    // Acceptance test (08-02): colon commands for save and open
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_from_file_when_colon_w_then_buffer_saved_and_message_shows_written() {
        // Given: a buffer loaded from a file, then modified
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test_save.txt");
        std::fs::write(&file_path, "Original").unwrap();

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_file(&file_path).unwrap();
        state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 8, " modified");
        setup_standard_keymaps(&mut state);

        // Precondition: buffer is modified
        assert!(state.buffer.is_modified());

        // When: type :w and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('w')), result);
        let (_, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        // Then: action is SaveBuffer(None)
        assert_eq!(action, super::DeferredAction::SaveBuffer(None));

        // And: when SaveBuffer is executed, file is written and message shows written
        // (Simulate the event loop's deferred action handling)
        match action {
            super::DeferredAction::SaveBuffer(opt_path) => {
                let save_path = match opt_path {
                    Some(ref p) => Some(std::path::PathBuf::from(p)),
                    None => state.buffer.file_path().map(|p| p.to_path_buf()),
                };
                match save_path {
                    Some(path) => match alfred_core::buffer::save_to_file(&state.buffer, &path) {
                        Ok(saved_buffer) => {
                            let byte_count = alfred_core::buffer::content(&saved_buffer).len();
                            state.buffer = saved_buffer;
                            state.message = Some(format!(
                                "\"{}\" written, {} bytes",
                                path.display(),
                                byte_count
                            ));
                        }
                        Err(e) => {
                            state.message = Some(format!("{}", e));
                        }
                    },
                    None => {
                        state.message = Some("No file name".to_string());
                    }
                }
            }
            _ => panic!("Expected SaveBuffer action"),
        }

        // Then: file on disk has updated content
        let on_disk = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, "Original modified");

        // And: buffer is no longer modified
        assert!(!state.buffer.is_modified());

        // And: message shows "written"
        let msg = state.message.as_ref().unwrap();
        assert!(
            msg.contains("written"),
            "Message should contain 'written', got: '{}'",
            msg
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests (08-02): colon save and open commands
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_when_colon_w_entered_then_returns_save_buffer_none() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        // Type :w and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('w')), result);
        let (input_state, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::SaveBuffer(None));
    }

    #[test]
    fn given_editor_when_colon_w_filename_entered_then_returns_save_buffer_with_path() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        // Type :w /tmp/test.txt and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "w /tmp/test.txt".chars() {
            result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        let (input_state, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(
            action,
            super::DeferredAction::SaveBuffer(Some("/tmp/test.txt".to_string()))
        );
    }

    #[test]
    fn given_editor_when_colon_e_filename_entered_then_returns_open_file_with_path() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        // Type :e /tmp/test.txt and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "e /tmp/test.txt".chars() {
            result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        let (input_state, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(
            action,
            super::DeferredAction::OpenFile("/tmp/test.txt".to_string())
        );
    }

    #[test]
    fn given_unnamed_buffer_when_colon_w_with_no_filename_then_save_buffer_none_returned() {
        // Given: a buffer with no file_path (unnamed)
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("some text");
        setup_standard_keymaps(&mut state);

        // When: :w Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char('w')), result);
        let (_, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        // Then: action is SaveBuffer(None) -- the event loop handler will check
        // for file_path and show "No file name" error
        assert_eq!(action, super::DeferredAction::SaveBuffer(None));

        // Simulate the event loop: unnamed buffer with SaveBuffer(None) -> error message
        assert!(state.buffer.file_path().is_none());
        // The event loop would set: state.message = Some("No file name".to_string());
    }

    #[test]
    fn given_buffer_from_file_when_colon_w_path_then_file_written_to_specified_path() {
        // Given: a buffer loaded from one file
        let dir = tempfile::TempDir::new().unwrap();
        let original_path = dir.path().join("original.txt");
        std::fs::write(&original_path, "Hello").unwrap();

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_file(&original_path).unwrap();

        // When: execute_colon_command with "w <new_path>"
        let new_path = dir.path().join("saveas.txt");
        let (input_state, action) =
            super::execute_colon_command(&mut state, &format!("w {}", new_path.display()));

        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(
            action,
            super::DeferredAction::SaveBuffer(Some(new_path.display().to_string()))
        );

        // Simulate executing the deferred save action
        if let super::DeferredAction::SaveBuffer(Some(ref p)) = action {
            let path = std::path::Path::new(p);
            let saved_buffer = alfred_core::buffer::save_to_file(&state.buffer, path).unwrap();
            state.buffer = saved_buffer;
        }

        // Then: file written to new path
        let on_disk = std::fs::read_to_string(&new_path).unwrap();
        assert_eq!(on_disk, "Hello");
    }

    #[test]
    fn given_existing_file_when_colon_e_then_buffer_replaced_and_cursor_reset() {
        // Given: a file exists with known content
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("open_test.txt");
        std::fs::write(&file_path, "Line1\nLine2\nLine3").unwrap();

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("old content");
        state.cursor = cursor::new(5, 10); // somewhere in old buffer

        // When: execute_colon_command with "e <path>"
        let (input_state, action) =
            super::execute_colon_command(&mut state, &format!("e {}", file_path.display()));

        assert_eq!(input_state, super::InputState::Normal);

        // Simulate executing the deferred open action
        if let super::DeferredAction::OpenFile(ref path_str) = action {
            let path = std::path::Path::new(path_str);
            match alfred_core::buffer::Buffer::from_file(path) {
                Ok(new_buffer) => {
                    let filename = new_buffer.filename().unwrap_or(path_str).to_string();
                    state.buffer = new_buffer;
                    state.cursor = alfred_core::cursor::new(0, 0);
                    state.viewport.top_line = 0;
                    state.message = Some(format!("\"{}\"", filename));
                }
                Err(e) => {
                    state.message = Some(format!("{}", e));
                }
            }
        }

        // Then: buffer contains the file content
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(content, "Line1\nLine2\nLine3");

        // And: cursor is reset to origin
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);

        // And: message shows the filename
        let msg = state.message.as_ref().unwrap();
        assert!(
            msg.contains("open_test.txt"),
            "Message should contain filename, got: '{}'",
            msg
        );
    }

    #[test]
    fn given_nonexistent_file_when_colon_e_then_error_message_shown() {
        // Given: a path to a nonexistent file
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("original");

        // When: execute_colon_command with "e /nonexistent/path.txt"
        let (_, action) =
            super::execute_colon_command(&mut state, "e /tmp/alfred_nonexistent_08_02.txt");

        // Simulate executing the deferred open action
        if let super::DeferredAction::OpenFile(ref path_str) = action {
            let path = std::path::Path::new(path_str);
            match alfred_core::buffer::Buffer::from_file(path) {
                Ok(new_buffer) => {
                    state.buffer = new_buffer;
                    state.cursor = alfred_core::cursor::new(0, 0);
                    state.viewport.top_line = 0;
                }
                Err(e) => {
                    state.message = Some(format!("{}", e));
                }
            }
        }

        // Then: error message is shown
        let msg = state.message.as_ref().unwrap();
        assert!(
            msg.contains("failed to read file") || msg.contains("error"),
            "Should show error message for nonexistent file, got: '{}'",
            msg
        );

        // And: original buffer is preserved
        let content = alfred_core::buffer::content(&state.buffer);
        assert_eq!(content, "original");
    }

    // -----------------------------------------------------------------------
    // Acceptance test (08-03): :wq saves and quits, :q warns on modified buffer
    // -----------------------------------------------------------------------

    #[test]
    fn given_modified_buffer_when_colon_wq_then_buffer_saved_and_editor_quits() {
        // Given: a buffer loaded from a file, then modified
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test_wq.txt");
        std::fs::write(&file_path, "Original").unwrap();

        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_file(&file_path).unwrap();
        state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 8, " changed");
        setup_standard_keymaps(&mut state);

        // Precondition: buffer is modified and running
        assert!(state.buffer.is_modified());
        assert!(state.running);

        // When: type :wq and press Enter
        let mut result = dispatch_key(
            &mut state,
            KeyEvent::plain(KeyCode::Char(':')),
            super::InputState::Normal,
        );
        for c in "wq".chars() {
            result = dispatch_key(&mut state, KeyEvent::plain(KeyCode::Char(c)), result);
        }
        let (_, action) =
            super::handle_key_event(&mut state, KeyEvent::plain(KeyCode::Enter), result);

        // Then: action is SaveAndQuit
        assert_eq!(action, super::DeferredAction::SaveAndQuit);

        // And: when SaveAndQuit is executed (simulate event loop)
        if let super::DeferredAction::SaveAndQuit = action {
            let save_path = state.buffer.file_path().map(|p| p.to_path_buf());
            match save_path {
                Some(path) => match alfred_core::buffer::save_to_file(&state.buffer, &path) {
                    Ok(saved_buffer) => {
                        let byte_count = alfred_core::buffer::content(&saved_buffer).len();
                        state.buffer = saved_buffer;
                        state.message = Some(format!(
                            "\"{}\" written, {} bytes",
                            path.display(),
                            byte_count
                        ));
                        state.running = false;
                    }
                    Err(e) => {
                        state.message = Some(format!("{}", e));
                    }
                },
                None => {
                    state.message = Some("No file name".to_string());
                }
            }
        }

        // Then: file on disk has updated content
        let on_disk = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, "Original changed");

        // And: buffer is no longer modified
        assert!(!state.buffer.is_modified());

        // And: editor is no longer running (quit)
        assert!(!state.running);

        // And: message shows "written"
        let msg = state.message.as_ref().unwrap();
        assert!(
            msg.contains("written"),
            "Message should contain 'written', got: '{}'",
            msg
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests (08-03): :wq, :q on modified buffer, :q! force quit
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_when_colon_wq_entered_then_returns_save_and_quit() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);

        let (input_state, action) = super::execute_colon_command(&mut state, "wq");

        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::SaveAndQuit);
        // running should NOT be set to false yet (deferred action handles that)
        assert!(state.running);
    }

    #[test]
    fn given_modified_buffer_when_colon_q_then_warns_unsaved_changes() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
        // Make the buffer modified
        state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 0, "text");
        assert!(state.buffer.is_modified());

        let (input_state, action) = super::execute_colon_command(&mut state, "q");

        // Should NOT quit
        assert!(state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
        // Should show warning message
        assert_eq!(
            state.message,
            Some("Unsaved changes! Use :q! to force quit".to_string())
        );
    }

    #[test]
    fn given_modified_buffer_when_colon_quit_then_warns_unsaved_changes() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
        state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 0, "text");
        assert!(state.buffer.is_modified());

        let (input_state, action) = super::execute_colon_command(&mut state, "quit");

        assert!(state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
        assert_eq!(
            state.message,
            Some("Unsaved changes! Use :q! to force quit".to_string())
        );
    }

    #[test]
    fn given_unmodified_buffer_when_colon_q_then_quits_normally() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
        assert!(!state.buffer.is_modified());

        let (input_state, action) = super::execute_colon_command(&mut state, "q");

        assert!(!state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
    }

    #[test]
    fn given_modified_buffer_when_colon_q_bang_then_force_quits() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
        state.buffer = alfred_core::buffer::insert_at(&state.buffer, 0, 0, "unsaved");
        assert!(state.buffer.is_modified());

        let (input_state, action) = super::execute_colon_command(&mut state, "q!");

        // Should quit despite modified buffer
        assert!(!state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
    }

    #[test]
    fn given_unmodified_buffer_when_colon_q_bang_then_also_quits() {
        let mut state = editor_state::new(80, 24);
        setup_standard_keymaps(&mut state);
        assert!(!state.buffer.is_modified());

        let (input_state, action) = super::execute_colon_command(&mut state, "q!");

        assert!(!state.running);
        assert_eq!(input_state, super::InputState::Normal);
        assert_eq!(action, super::DeferredAction::None);
    }
}
