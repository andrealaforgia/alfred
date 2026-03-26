//! Input state machine for the Alfred editor.
//!
//! This module contains the pure input handling logic: given the current editor
//! state, a key event, and the current input state, it computes the next input
//! state and a deferred action for the caller to execute. No terminal I/O or
//! rendering happens here -- this is the functional core of key processing.

use alfred_core::editor_state::EditorState;
use alfred_core::key_event::{KeyCode, KeyEvent};

// ---------------------------------------------------------------------------
// Input state types
// ---------------------------------------------------------------------------

/// Vim operator that waits for a motion to define a range.
///
/// Operators are the first half of the operator-motion composition:
/// pressing `d` enters `OperatorPending(Delete)`, then the next key
/// is resolved as a motion that defines the range to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operator {
    /// Delete text in the motion range
    Delete,
    /// Change text in the motion range (delete + enter insert mode)
    Change,
    /// Yank (copy) text in the motion range to the yank register
    Yank,
}

/// Tracks multi-key input state (e.g., command-line after `:`)
#[derive(Debug, PartialEq)]
pub(crate) enum InputState {
    /// Normal key dispatch
    Normal,
    /// Accumulating a command-line string (entered via `:`)
    Command(String),
    /// Accumulating a search pattern (entered via `/`)
    Search(String),
    /// Waiting for a character key to complete a find/til command (f/F/t/T)
    PendingChar(alfred_core::editor_state::CharFindKind),
    /// Waiting for a motion key to complete an operator (d, c, y)
    OperatorPending(Operator),
    /// Waiting for a text object type key after operator + modifier (e.g., `di` + `w`, `ca` + `"`)
    TextObject(Operator, alfred_core::text_object::TextObjectModifier),
    /// Waiting for a character key to set a mark (`m` + `{a-z}`)
    PendingMark,
    /// Waiting for a character key to jump to a mark (`'` + `{a-z}`)
    PendingJumpMark,
    /// Waiting for a register name character after `"` prefix.
    /// The next 'a'-'z' selects the named register for the following command.
    PendingRegister,
    /// Waiting for a register name character after `q` to start macro recording.
    /// The next 'a'-'z' starts recording into that register.
    PendingMacroRecord,
    /// Waiting for a register name character after `@` to replay a macro.
    /// The next 'a'-'z' replays the macro from that register; `@` replays the last macro.
    PendingMacroPlay,
    /// Waiting for a character key to replace the char under cursor (`r` + `{char}`).
    PendingReplace,
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

// ---------------------------------------------------------------------------
// Motion helpers
// ---------------------------------------------------------------------------

/// The kind of motion: character-wise (w, e, $, h, l, etc.) or line-wise (j, k).
///
/// Line-wise motions operate on entire lines (the current line plus the motion target line).
/// Character-wise motions operate on the character range between the cursor and the motion endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionKind {
    CharWise,
    LineWise,
}

/// Resolves a command name to a cursor motion, returning the new cursor position and motion kind.
///
/// This is a pure function: given the current editor state and a command name,
/// it computes where the cursor would move to, and whether the motion is
/// character-wise or line-wise. Returns None if the command is not a recognized motion.
fn execute_motion(
    state: &EditorState,
    motion_cmd: &str,
) -> Option<(alfred_core::cursor::Cursor, MotionKind)> {
    match motion_cmd {
        "cursor-word-forward" => {
            let target = alfred_core::cursor::move_word_forward(state.cursor, &state.buffer);
            let line_len = alfred_core::buffer::get_line(&state.buffer, state.cursor.line)
                .map(|l| l.trim_end_matches('\n').len())
                .unwrap_or(0);

            if target.line > state.cursor.line {
                // Word-forward crossed to a new line -- clamp to end of current line.
                // Makes `cw` on the last word behave like `c$` (vim semantics).
                Some((
                    alfred_core::cursor::Cursor {
                        line: state.cursor.line,
                        column: line_len,
                    },
                    MotionKind::CharWise,
                ))
            } else if target.line == state.cursor.line
                && target.column >= line_len.saturating_sub(1)
            {
                // Word-forward stayed on same line but hit end -- use line_len (exclusive)
                // so the delete range includes the last character of the word.
                Some((
                    alfred_core::cursor::Cursor {
                        line: state.cursor.line,
                        column: line_len,
                    },
                    MotionKind::CharWise,
                ))
            } else {
                Some((target, MotionKind::CharWise))
            }
        }
        "cursor-word-end" => {
            let end_cursor = alfred_core::cursor::move_word_end(state.cursor, &state.buffer);
            // word-end motion is inclusive: advance one past the endpoint so the last char is included
            Some((
                alfred_core::cursor::Cursor {
                    line: end_cursor.line,
                    column: end_cursor.column + 1,
                },
                MotionKind::CharWise,
            ))
        }
        "cursor-line-end" => {
            let line_len = alfred_core::buffer::get_line(&state.buffer, state.cursor.line)
                .map(|l| l.trim_end_matches('\n').len())
                .unwrap_or(0);
            // $ motion is inclusive of the last character on the line
            Some((
                alfred_core::cursor::Cursor {
                    line: state.cursor.line,
                    column: line_len,
                },
                MotionKind::CharWise,
            ))
        }
        "cursor-line-start" => Some((
            alfred_core::cursor::move_to_line_start(state.cursor, &state.buffer),
            MotionKind::CharWise,
        )),
        "cursor-word-backward" => Some((
            alfred_core::cursor::move_word_backward(state.cursor, &state.buffer),
            MotionKind::CharWise,
        )),
        "cursor-right" => Some((
            alfred_core::cursor::move_right(state.cursor, &state.buffer),
            MotionKind::CharWise,
        )),
        "cursor-left" => Some((
            alfred_core::cursor::move_left(state.cursor, &state.buffer),
            MotionKind::CharWise,
        )),
        "cursor-down" => Some((
            alfred_core::cursor::move_down(state.cursor, &state.buffer),
            MotionKind::LineWise,
        )),
        "cursor-up" => Some((
            alfred_core::cursor::move_up(state.cursor, &state.buffer),
            MotionKind::LineWise,
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Operator execution helpers
// ---------------------------------------------------------------------------

/// Executes a delete operator with the given motion, modifying editor state.
///
/// For character-wise motions, deletes text from min(cursor, motion) to max(cursor, motion).
/// For line-wise motions, deletes entire lines from the current line to the motion target line.
/// Pushes undo state before any mutation.
fn execute_delete_with_motion(
    state: &mut EditorState,
    motion_cursor: alfred_core::cursor::Cursor,
    motion_kind: MotionKind,
) {
    alfred_core::editor_state::push_undo(state);

    match motion_kind {
        MotionKind::CharWise => {
            let (from, to) = if (state.cursor.line, state.cursor.column)
                <= (motion_cursor.line, motion_cursor.column)
            {
                (state.cursor, motion_cursor)
            } else {
                (motion_cursor, state.cursor)
            };
            state.buffer = alfred_core::buffer::delete_char_range(
                &state.buffer,
                from.line,
                from.column,
                to.line,
                to.column,
            );
            state.cursor = alfred_core::cursor::ensure_within_bounds(from, &state.buffer);
        }
        MotionKind::LineWise => {
            let min_line = state.cursor.line.min(motion_cursor.line);
            let max_line = state.cursor.line.max(motion_cursor.line);
            // Delete lines from max to min (reverse order to preserve indices)
            for line in (min_line..=max_line).rev() {
                state.buffer = alfred_core::buffer::delete_line(&state.buffer, line);
            }
            state.cursor = alfred_core::cursor::new(min_line, 0);
            state.cursor = alfred_core::cursor::ensure_within_bounds(state.cursor, &state.buffer);
        }
    }

    state.viewport = alfred_core::facade::viewport_adjust(state);
}

/// Executes a delete operator over an explicit character range (for text objects).
///
/// Deletes text from `from` (inclusive) to `to` (exclusive). Pushes undo state.
/// Cursor is placed at `from`, clamped to buffer bounds.
fn execute_delete_range(
    state: &mut EditorState,
    from: alfred_core::cursor::Cursor,
    to: alfred_core::cursor::Cursor,
) {
    alfred_core::editor_state::push_undo(state);
    state.buffer = alfred_core::buffer::delete_char_range(
        &state.buffer,
        from.line,
        from.column,
        to.line,
        to.column,
    );
    state.cursor = alfred_core::cursor::ensure_within_bounds(from, &state.buffer);
    state.viewport = alfred_core::facade::viewport_adjust(state);
}

/// Executes a yank operator over an explicit character range (for text objects).
///
/// Copies text from `from` (inclusive) to `to` (exclusive) to the yank register.
fn execute_yank_range(
    state: &mut EditorState,
    from: alfred_core::cursor::Cursor,
    to: alfred_core::cursor::Cursor,
) {
    let text = alfred_core::buffer::get_text_range(
        &state.buffer,
        from.line,
        from.column,
        to.line,
        to.column,
    );
    let reg = state.pending_register.take();
    alfred_core::editor_state::set_register(state, reg, text, false);
    state.message = Some("yanked".to_string());
}

/// Executes a yank operator with the given motion, copying text to the yank register.
///
/// For character-wise motions, yanks text from min(cursor, motion) to max(cursor, motion).
/// For line-wise motions, yanks entire lines from the current line to the motion target line.
/// The cursor stays at its original position after yanking.
fn execute_yank_with_motion(
    state: &mut EditorState,
    motion_cursor: alfred_core::cursor::Cursor,
    motion_kind: MotionKind,
) {
    let reg = state.pending_register.take();
    match motion_kind {
        MotionKind::CharWise => {
            let (from, to) = if (state.cursor.line, state.cursor.column)
                <= (motion_cursor.line, motion_cursor.column)
            {
                (state.cursor, motion_cursor)
            } else {
                (motion_cursor, state.cursor)
            };
            let text = alfred_core::buffer::get_text_range(
                &state.buffer,
                from.line,
                from.column,
                to.line,
                to.column,
            );
            alfred_core::editor_state::set_register(state, reg, text, false);
            state.message = Some("yanked".to_string());
        }
        MotionKind::LineWise => {
            let min_line = state.cursor.line.min(motion_cursor.line);
            let max_line = state.cursor.line.max(motion_cursor.line);
            let mut lines = Vec::new();
            for line in min_line..=max_line {
                lines.push(alfred_core::buffer::get_line_content(&state.buffer, line));
            }
            let line_count = lines.len();
            alfred_core::editor_state::set_register(state, reg, lines.join("\n"), true);
            state.message = Some(format!(
                "{} line{} yanked",
                line_count,
                if line_count == 1 { "" } else { "s" }
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Colon command delegation
// ---------------------------------------------------------------------------

/// Executes a colon command, returning the new input state and a deferred action.
///
/// Delegates to [`crate::colon_commands::execute_colon_command`] which contains
/// the pure parsing logic and execution for all colon commands.
pub(crate) fn execute_colon_command(
    state: &mut EditorState,
    command: &str,
) -> (InputState, DeferredAction) {
    crate::colon_commands::execute_colon_command(state, command)
}

// ---------------------------------------------------------------------------
// Main key event handler
// ---------------------------------------------------------------------------

/// Handles a key event by updating the editor state.
///
/// Returns `(InputState, DeferredAction, Option<u32>)` where:
/// - `InputState` tracks multi-key input mode (normal vs command-line)
/// - `DeferredAction` tells the caller what to do after dropping the EditorState borrow
/// - `Option<u32>` is the pending numeric count prefix (for Vim-style `5j`, `3x`, etc.)
///
/// In normal mode, digit keys (1-9 to start, 0-9 to continue) accumulate into
/// a count prefix. When a non-digit key arrives, the command is dispatched and
/// the count is returned so the caller can execute it that many times.
/// `0` alone (no pending count) maps to `cursor-line-start` as usual.
pub(crate) fn handle_key_event(
    state: &mut EditorState,
    key: KeyEvent,
    input_state: InputState,
    pending_count: Option<u32>,
) -> (InputState, DeferredAction, Option<u32>) {
    // Macro recording: if recording and this key is `q` (stop recording),
    // store the accumulated buffer and return to Normal without recording the `q`.
    // Otherwise, push the key into the macro buffer before processing.
    if state.macro_recording.is_some() && !state.macro_replaying {
        if input_state == InputState::Normal && key.code == KeyCode::Char('q') {
            let register = state.macro_recording.take().unwrap();
            state
                .macro_registers
                .insert(register, state.macro_buffer.drain(..).collect());
            state.message = None;
            return (InputState::Normal, DeferredAction::None, None);
        }
        // Record the key before processing it normally
        state.macro_buffer.push(key);
    }

    // Command-line mode: accumulating input after `:`
    // Count prefix is discarded when entering command mode.
    if let InputState::Command(mut cmd) = input_state {
        match key.code {
            KeyCode::Enter => {
                let (is, da) = execute_colon_command(state, cmd.trim());
                return (is, da, None);
            }
            KeyCode::Escape => {
                state.message = None;
                return (InputState::Normal, DeferredAction::None, None);
            }
            KeyCode::Backspace => {
                cmd.pop();
                if cmd.is_empty() {
                    state.message = None;
                    return (InputState::Normal, DeferredAction::None, None);
                }
                state.message = Some(format!(":{}", cmd));
                return (InputState::Command(cmd), DeferredAction::None, None);
            }
            KeyCode::Char(c) => {
                cmd.push(c);
                state.message = Some(format!(":{}", cmd));
                return (InputState::Command(cmd), DeferredAction::None, None);
            }
            _ => {
                return (InputState::Command(cmd), DeferredAction::None, None);
            }
        }
    }

    // Search mode: accumulating a search pattern after `/`
    // Count prefix is discarded when entering search mode.
    if let InputState::Search(mut pattern) = input_state {
        match key.code {
            KeyCode::Enter => {
                if !pattern.is_empty() {
                    // Push current position to jump list before search jump
                    alfred_core::editor_state::push_jump(state);
                    state.search_pattern = Some(pattern.clone());
                    state.search_forward = true;
                    // Execute the forward search
                    let found = alfred_core::buffer::find_forward(
                        &state.buffer,
                        state.cursor.line,
                        state.cursor.column,
                        &pattern,
                    );
                    match found {
                        Some((line, col)) => {
                            state.cursor = alfred_core::cursor::new(line, col);
                            state.viewport = alfred_core::facade::viewport_adjust(state);
                            state.message = None;
                        }
                        None => {
                            state.message = Some(format!("Pattern not found: {}", pattern));
                        }
                    }
                } else {
                    state.message = None;
                }
                return (InputState::Normal, DeferredAction::None, None);
            }
            KeyCode::Escape => {
                state.message = None;
                return (InputState::Normal, DeferredAction::None, None);
            }
            KeyCode::Backspace => {
                pattern.pop();
                if pattern.is_empty() {
                    state.message = None;
                    return (InputState::Normal, DeferredAction::None, None);
                }
                state.message = Some(format!("/{}", pattern));
                return (InputState::Search(pattern), DeferredAction::None, None);
            }
            KeyCode::Char(c) => {
                pattern.push(c);
                state.message = Some(format!("/{}", pattern));
                return (InputState::Search(pattern), DeferredAction::None, None);
            }
            _ => {
                return (InputState::Search(pattern), DeferredAction::None, None);
            }
        }
    }

    // PendingChar mode: waiting for a character key after f/F/t/T.
    // Execute the char find, store it for repeat, return to Normal.
    if let InputState::PendingChar(kind) = input_state {
        if let KeyCode::Char(ch) = key.code {
            if let Some(new_cursor) =
                alfred_core::editor_state::execute_char_find(state.cursor, &state.buffer, kind, ch)
            {
                state.cursor = new_cursor;
                state.viewport = alfred_core::facade::viewport_adjust(state);
            }
            state.last_char_find = Some((kind, ch));
        }
        // Any non-Char key (e.g., Escape) just cancels the pending find.
        return (InputState::Normal, DeferredAction::None, None);
    }

    // PendingReplace mode: waiting for a character key after `r`.
    // Replace the character under cursor with the pressed character, return to Normal.
    if let InputState::PendingReplace = input_state {
        if let KeyCode::Char(ch) = key.code {
            alfred_core::editor_state::push_undo(state);
            state.buffer = alfred_core::buffer::replace_char_at(
                &state.buffer,
                state.cursor.line,
                state.cursor.column,
                ch,
            );
        }
        // Any non-Char key (e.g., Escape) just cancels the pending replace.
        return (InputState::Normal, DeferredAction::None, None);
    }

    // PendingMark mode: waiting for a character key after `m`.
    // Store the mark at the current cursor position, return to Normal.
    if let InputState::PendingMark = input_state {
        if let KeyCode::Char(ch) = key.code {
            if alfred_core::editor_state::is_valid_mark_char(ch) {
                alfred_core::editor_state::set_mark(state, ch);
            }
            // Invalid mark characters (digits, uppercase, etc.) are silently ignored.
        }
        // Any non-Char key (e.g., Escape) just cancels the pending mark.
        return (InputState::Normal, DeferredAction::None, None);
    }

    // PendingJumpMark mode: waiting for a character key after `'`.
    // Jump cursor to the stored mark position, or show error if not set.
    if let InputState::PendingJumpMark = input_state {
        if let KeyCode::Char(ch) = key.code {
            // Push current position to jump list before jumping to mark
            alfred_core::editor_state::push_jump(state);
            if let Err(msg) = alfred_core::editor_state::jump_to_mark(state, ch) {
                state.message = Some(msg);
            }
        }
        // Any non-Char key (e.g., Escape) just cancels the pending jump.
        return (InputState::Normal, DeferredAction::None, None);
    }

    // PendingRegister mode: waiting for a register name character after `"`.
    // Valid register names are 'a'-'z'. Escape cancels.
    if let InputState::PendingRegister = input_state {
        match key.code {
            KeyCode::Char(ch) if alfred_core::editor_state::is_valid_named_register(ch) => {
                state.pending_register = Some(ch);
                return (InputState::Normal, DeferredAction::None, pending_count);
            }
            KeyCode::Char('"') => {
                // `""` explicitly selects the unnamed register (no-op, but valid)
                state.pending_register = None;
                return (InputState::Normal, DeferredAction::None, pending_count);
            }
            KeyCode::Escape => {
                // Cancel register prefix
                state.pending_register = None;
                return (InputState::Normal, DeferredAction::None, None);
            }
            _ => {
                // Invalid register character -- ignore and cancel
                state.pending_register = None;
                return (InputState::Normal, DeferredAction::None, None);
            }
        }
    }

    // PendingMacroRecord mode: waiting for a register name character after `q`.
    // Valid register names are 'a'-'z'. Escape cancels.
    if let InputState::PendingMacroRecord = input_state {
        match key.code {
            KeyCode::Char(ch) if ch.is_ascii_lowercase() => {
                state.macro_recording = Some(ch);
                state.macro_buffer.clear();
                state.message = Some(format!("recording @{}", ch));
                return (InputState::Normal, DeferredAction::None, None);
            }
            _ => {
                // Escape or invalid character cancels
                return (InputState::Normal, DeferredAction::None, None);
            }
        }
    }

    // PendingMacroPlay mode: waiting for a register name character after `@`.
    // Valid register names are 'a'-'z'. `@` replays the last played macro.
    if let InputState::PendingMacroPlay = input_state {
        let register = match key.code {
            KeyCode::Char('@') => state.last_macro_register,
            KeyCode::Char(ch) if ch.is_ascii_lowercase() => Some(ch),
            _ => {
                // Escape or invalid character cancels
                return (InputState::Normal, DeferredAction::None, None);
            }
        };

        if let Some(reg) = register {
            if let Some(keys) = state.macro_registers.get(&reg).cloned() {
                state.last_macro_register = Some(reg);
                state.macro_replaying = true;
                let mut replay_input_state = InputState::Normal;
                for replay_key in keys {
                    let (new_is, action, _count) =
                        handle_key_event(state, replay_key, replay_input_state, None);
                    replay_input_state = new_is;
                    // Execute deferred commands inline during replay
                    if let DeferredAction::ExecCommand(ref cmd_name) = action {
                        if alfred_core::editor_state::is_jump_command(cmd_name) {
                            alfred_core::editor_state::push_jump(state);
                        }
                        let _ = alfred_core::command::execute(state, cmd_name);
                    }
                }
                state.macro_replaying = false;
            }
            // If register not set, no-op
        }
        return (InputState::Normal, DeferredAction::None, None);
    }

    // OperatorPending mode: waiting for a motion key after an operator (d, c, y).
    // Resolve the next key as a motion, compute the range, execute the operator.
    if let InputState::OperatorPending(operator) = input_state {
        // Escape cancels the operator
        if key.code == KeyCode::Escape {
            return (InputState::Normal, DeferredAction::None, None);
        }

        // Check if same operator key pressed again (dd/cc/yy = line-wise operation)
        let doubled = matches!(
            (operator, key.code),
            (Operator::Delete, KeyCode::Char('d'))
                | (Operator::Change, KeyCode::Char('c'))
                | (Operator::Yank, KeyCode::Char('y'))
        );

        if doubled {
            match operator {
                Operator::Delete => {
                    alfred_core::editor_state::push_undo(state);
                    state.buffer =
                        alfred_core::buffer::delete_line(&state.buffer, state.cursor.line);
                    state.cursor =
                        alfred_core::cursor::ensure_within_bounds(state.cursor, &state.buffer);
                    state.viewport = alfred_core::facade::viewport_adjust(state);
                }
                Operator::Change => {
                    // cc = clear current line content, enter insert mode
                    alfred_core::editor_state::push_undo(state);
                    state.buffer =
                        alfred_core::buffer::replace_line(&state.buffer, state.cursor.line, "");
                    state.cursor = alfred_core::cursor::new(state.cursor.line, 0);
                    state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
                    state.active_keymaps =
                        vec![format!("{}-mode", alfred_core::editor_state::MODE_INSERT)];
                    state.viewport = alfred_core::facade::viewport_adjust(state);
                }
                Operator::Yank => {
                    // yy = yank entire line
                    let content =
                        alfred_core::buffer::get_line_content(&state.buffer, state.cursor.line);
                    let reg = state.pending_register.take();
                    alfred_core::editor_state::set_register(state, reg, content, true);
                    state.message = Some("1 line yanked".to_string());
                }
            }
            return (InputState::Normal, DeferredAction::None, None);
        }

        // Text object modifier: 'i' (inner) or 'a' (around) enters TextObject sub-state
        if let KeyCode::Char('i') = key.code {
            return (
                InputState::TextObject(
                    operator,
                    alfred_core::text_object::TextObjectModifier::Inner,
                ),
                DeferredAction::None,
                None,
            );
        }
        if let KeyCode::Char('a') = key.code {
            return (
                InputState::TextObject(
                    operator,
                    alfred_core::text_object::TextObjectModifier::Around,
                ),
                DeferredAction::None,
                None,
            );
        }

        // Look up the key in the keymap to get a command name
        if let Some(cmd_name) = alfred_core::facade::resolve_key(state, key) {
            if let Some((motion_cursor, motion_kind)) = execute_motion(state, &cmd_name) {
                match operator {
                    Operator::Delete => {
                        execute_delete_with_motion(state, motion_cursor, motion_kind);
                    }
                    Operator::Change => {
                        // Change = delete range + enter insert mode
                        execute_delete_with_motion(state, motion_cursor, motion_kind);
                        state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
                        state.active_keymaps =
                            vec![format!("{}-mode", alfred_core::editor_state::MODE_INSERT)];
                    }
                    Operator::Yank => {
                        // Yank = copy text in range to register, don't delete
                        execute_yank_with_motion(state, motion_cursor, motion_kind);
                    }
                }
                return (InputState::Normal, DeferredAction::None, None);
            }
        }

        // Unrecognized motion key: cancel operator
        return (InputState::Normal, DeferredAction::None, None);
    }

    // TextObject mode: after operator + modifier (i/a), waiting for the object type key.
    // Resolves the text object, computes the range, and applies the operator.
    if let InputState::TextObject(operator, modifier) = input_state {
        // Escape cancels the text object
        if key.code == KeyCode::Escape {
            return (InputState::Normal, DeferredAction::None, None);
        }

        // Resolve the text object type from the key
        let range = match key.code {
            KeyCode::Char('w') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_word(state.cursor, &state.buffer)
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_word(state.cursor, &state.buffer)
                }
            },
            KeyCode::Char('"') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_quotes(state.cursor, &state.buffer, '"')
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_quotes(state.cursor, &state.buffer, '"')
                }
            },
            KeyCode::Char('\'') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_quotes(state.cursor, &state.buffer, '\'')
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_quotes(state.cursor, &state.buffer, '\'')
                }
            },
            KeyCode::Char('(' | ')') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_parens(state.cursor, &state.buffer, '(', ')')
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_parens(state.cursor, &state.buffer, '(', ')')
                }
            },
            KeyCode::Char('[' | ']') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_parens(state.cursor, &state.buffer, '[', ']')
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_parens(state.cursor, &state.buffer, '[', ']')
                }
            },
            KeyCode::Char('{' | '}') => match modifier {
                alfred_core::text_object::TextObjectModifier::Inner => {
                    alfred_core::text_object::inner_parens(state.cursor, &state.buffer, '{', '}')
                }
                alfred_core::text_object::TextObjectModifier::Around => {
                    alfred_core::text_object::around_parens(state.cursor, &state.buffer, '{', '}')
                }
            },
            _ => None, // Unrecognized text object type: cancel
        };

        if let Some((range_start, range_end)) = range {
            match operator {
                Operator::Delete => {
                    execute_delete_range(state, range_start, range_end);
                }
                Operator::Change => {
                    // Change = delete range + enter insert mode
                    execute_delete_range(state, range_start, range_end);
                    state.mode = alfred_core::editor_state::MODE_INSERT.to_string();
                    state.active_keymaps =
                        vec![format!("{}-mode", alfred_core::editor_state::MODE_INSERT)];
                }
                Operator::Yank => {
                    execute_yank_range(state, range_start, range_end);
                }
            }
        }

        return (InputState::Normal, DeferredAction::None, None);
    }

    // Normal mode only: accumulate digit keys into a count prefix.
    // 1-9 starts a new count; 0-9 appends when a count is already pending.
    // 0 alone (no pending count) falls through to keymap resolution (cursor-line-start).
    // In insert mode, digits are handled by self-insert (below), not as counts.
    if state.mode == alfred_core::editor_state::MODE_NORMAL {
        if let KeyCode::Char(digit @ '0'..='9') = key.code {
            let is_start_digit = ('1'..='9').contains(&digit);
            if is_start_digit || pending_count.is_some() {
                let current = pending_count.unwrap_or(0);
                let new_count = current
                    .saturating_mul(10)
                    .saturating_add(digit as u32 - '0' as u32);
                return (InputState::Normal, DeferredAction::None, Some(new_count));
            }
        }
    } // end normal-mode-only digit check

    // Non-digit key in normal mode: resolve through active keymaps.
    // The accumulated count (if any) is returned for the caller to repeat the command.
    let repeat_count = pending_count;
    match alfred_core::facade::resolve_key(state, key) {
        Some(ref cmd) if cmd == "enter-command-mode" => {
            state.message = Some(":".to_string());
            // Discard count when entering command mode
            (
                InputState::Command(String::new()),
                DeferredAction::None,
                None,
            )
        }
        Some(ref cmd) if cmd == "enter-search-mode" => {
            state.message = Some("/".to_string());
            // Discard count when entering search mode
            (
                InputState::Search(String::new()),
                DeferredAction::None,
                None,
            )
        }
        Some(ref cmd) if cmd == "enter-char-find-forward" => (
            InputState::PendingChar(alfred_core::editor_state::CharFindKind::FindForward),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-char-find-backward" => (
            InputState::PendingChar(alfred_core::editor_state::CharFindKind::FindBackward),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-char-til-forward" => (
            InputState::PendingChar(alfred_core::editor_state::CharFindKind::TilForward),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-char-til-backward" => (
            InputState::PendingChar(alfred_core::editor_state::CharFindKind::TilBackward),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-register-prefix" => (
            InputState::PendingRegister,
            DeferredAction::None,
            pending_count,
        ),
        Some(ref cmd) if cmd == "enter-set-mark" => {
            (InputState::PendingMark, DeferredAction::None, None)
        }
        Some(ref cmd) if cmd == "enter-jump-mark" => {
            (InputState::PendingJumpMark, DeferredAction::None, None)
        }
        Some(ref cmd) if cmd == "enter-macro-record" => {
            (InputState::PendingMacroRecord, DeferredAction::None, None)
        }
        Some(ref cmd) if cmd == "enter-macro-play" => {
            (InputState::PendingMacroPlay, DeferredAction::None, None)
        }
        Some(ref cmd) if cmd == "enter-replace-char" => {
            (InputState::PendingReplace, DeferredAction::None, None)
        }
        Some(ref cmd) if cmd == "enter-operator-delete" => (
            InputState::OperatorPending(Operator::Delete),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-operator-change" => (
            InputState::OperatorPending(Operator::Change),
            DeferredAction::None,
            None,
        ),
        Some(ref cmd) if cmd == "enter-operator-yank" => (
            InputState::OperatorPending(Operator::Yank),
            DeferredAction::None,
            None,
        ),
        Some(cmd) => (
            InputState::Normal,
            DeferredAction::ExecCommand(cmd),
            repeat_count,
        ),
        None => {
            // Self-insert: only in insert mode with active keymaps.
            // Handles printable characters and Enter (newline).
            // Count prefix does not apply to insert-mode self-insert.
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
                        state.viewport = alfred_core::facade::viewport_adjust(state);
                    }
                    KeyCode::Enter => {
                        alfred_core::editor_state::push_undo(state);
                        let line = state.cursor.line;
                        let col = state.cursor.column;
                        state.buffer =
                            alfred_core::buffer::insert_at(&state.buffer, line, col, "\n");
                        // Move cursor to beginning of new line
                        state.cursor = alfred_core::cursor::new(line + 1, 0);
                        state.viewport = alfred_core::facade::viewport_adjust(state);
                    }
                    KeyCode::Tab => {
                        alfred_core::editor_state::push_undo(state);
                        let line = state.cursor.line;
                        let col = state.cursor.column;
                        let spaces = " ".repeat(state.tab_width);
                        state.buffer =
                            alfred_core::buffer::insert_at(&state.buffer, line, col, &spaces);
                        for _ in 0..state.tab_width {
                            state.cursor =
                                alfred_core::cursor::move_right(state.cursor, &state.buffer);
                        }
                        state.viewport = alfred_core::facade::viewport_adjust(state);
                    }
                    _ => {}
                }
            }
            (InputState::Normal, DeferredAction::None, None)
        }
    }
}
