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
pub const MODE_VISUAL: &str = "visual";

/// The kind of character find operation (f/F/t/T).
///
/// Used to track what was last executed so `;` and `,` can repeat or reverse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharFindKind {
    FindForward,
    FindBackward,
    TilForward,
    TilBackward,
}

/// A snapshot of buffer and cursor state for undo/redo.
///
/// Rope cloning is O(1) due to structural sharing, making
/// whole-buffer snapshots cheap.
#[derive(Debug, Clone)]
pub struct UndoSnapshot {
    pub buffer: Buffer,
    pub cursor: Cursor,
}

/// The unnamed register key, used when no register prefix is specified.
pub const UNNAMED_REGISTER: char = '"';

/// An entry in a named register, storing both content and whether the yank was line-wise.
///
/// Line-wise registers paste on new lines; character-wise registers paste inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterEntry {
    pub content: String,
    pub linewise: bool,
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
    /// Named registers ('a'-'z') and the unnamed register ('"').
    /// Each register stores content and whether the yank was line-wise.
    pub registers: HashMap<char, RegisterEntry>,
    /// The register selected by the `"x` prefix for the next command.
    /// When `Some('a')`, the next yank/delete/paste uses register 'a'.
    /// Cleared after use by the consuming command.
    pub pending_register: Option<char>,
    pub undo_stack: Vec<UndoSnapshot>,
    pub redo_stack: Vec<UndoSnapshot>,
    pub theme: Theme,
    pub named_themes: HashMap<String, Theme>,
    /// Maps mode name to cursor shape name (e.g., "normal" -> "block", "insert" -> "bar").
    pub cursor_shapes: HashMap<String, String>,
    /// The most recent search pattern (stored for `n`/`N` repeat).
    pub search_pattern: Option<String>,
    /// True means last search was forward (`/`), false means backward (`?`).
    pub search_forward: bool,
    /// The most recent character find (f/F/t/T) for `;`/`,` repeat.
    pub last_char_find: Option<(CharFindKind, char)>,
    /// The name of the last buffer-mutating command, for `.` (repeat-last-change).
    pub last_edit_command: Option<String>,
    /// The anchor point where visual selection started (`v` or `V`).
    /// When `Some`, visual mode is active; the selection spans from this cursor to `self.cursor`.
    pub selection_start: Option<Cursor>,
    /// Whether the current visual selection is line-wise (`V`) or character-wise (`v`).
    /// When true, visual operators expand the selection to full lines before acting.
    pub visual_line_mode: bool,
    /// Named marks ('a'-'z') mapping to cursor positions.
    /// Users set marks with `m{a-z}` and jump to them with `'{a-z}`.
    pub marks: HashMap<char, Cursor>,
    /// Macro registers ('a'-'z') storing recorded key sequences.
    /// Separate from yank registers since macros store `Vec<KeyEvent>`.
    pub macro_registers: HashMap<char, Vec<KeyEvent>>,
    /// Which register is currently being recorded to (`q{a-z}` starts, `q` stops).
    /// `None` means not recording.
    pub macro_recording: Option<char>,
    /// Keys accumulated during the current macro recording session.
    pub macro_buffer: Vec<KeyEvent>,
    /// True while replaying a macro, to prevent re-recording replayed keys.
    pub macro_replaying: bool,
    /// The register of the last played macro, for `@@` (repeat last macro).
    pub last_macro_register: Option<char>,
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

/// All recognized cursor shape names.
///
/// These names can be used with `set-cursor-shape` to configure the terminal
/// cursor appearance per mode.
pub const VALID_CURSOR_SHAPES: &[&str] = &[
    "default",
    "block",
    "steady-block",
    "blinking-block",
    "bar",
    "steady-bar",
    "blinking-bar",
    "underline",
    "steady-underline",
    "blinking-underline",
];

/// Returns true if the given shape name is a recognized cursor shape.
///
/// This is a pure validation function with no side effects.
pub fn is_valid_cursor_shape(shape_name: &str) -> bool {
    VALID_CURSOR_SHAPES.contains(&shape_name)
}

/// Looks up the cursor shape name for the current mode.
///
/// Returns the configured shape name for the given mode, or "default" if
/// no shape has been configured for that mode.
pub fn cursor_shape_for_mode(state: &EditorState) -> &str {
    state
        .cursor_shapes
        .get(&state.mode)
        .map(|s| s.as_str())
        .unwrap_or("default")
}

/// Returns true if the given character is a valid named register ('a'-'z').
pub fn is_valid_named_register(c: char) -> bool {
    c.is_ascii_lowercase()
}

/// Gets the content of the specified register, or the unnamed register if `None`.
///
/// Returns `None` if the register has no content.
pub fn get_register(state: &EditorState, register: Option<char>) -> Option<&RegisterEntry> {
    let key = register.unwrap_or(UNNAMED_REGISTER);
    state.registers.get(&key)
}

/// Sets the content of the specified register (or the unnamed register if `None`).
///
/// Also always copies into the unnamed register, matching Vim behavior:
/// every yank/delete populates both the target and the unnamed register.
pub fn set_register(
    state: &mut EditorState,
    register: Option<char>,
    content: String,
    linewise: bool,
) {
    let entry = RegisterEntry {
        content: content.clone(),
        linewise,
    };
    let key = register.unwrap_or(UNNAMED_REGISTER);
    state.registers.insert(key, entry.clone());
    // Always update the unnamed register (Vim behavior)
    if key != UNNAMED_REGISTER {
        state.registers.insert(UNNAMED_REGISTER, entry);
    }
}

/// Convenience: get the yank register content as Option<String> + linewise flag.
///
/// Used for backwards-compatible access matching the old `yank_register` + `yank_linewise` API.
pub fn get_yank_content(state: &EditorState, register: Option<char>) -> Option<(String, bool)> {
    get_register(state, register).map(|e| (e.content.clone(), e.linewise))
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
            if s.cursor.line == 0 && s.cursor.column == 0 {
                return Ok(());
            }
            push_undo(s);
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
            push_undo(s);
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
            push_undo(s);
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
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
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
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
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
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "open-line-below".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            let current_line = s.cursor.line;
            let line_len = crate::buffer::get_line(&s.buffer, current_line)
                .map(|l| l.trim_end_matches('\n').len())
                .unwrap_or(0);
            s.buffer = crate::buffer::insert_at(&s.buffer, current_line, line_len, "\n");
            s.cursor = crate::cursor::new(current_line + 1, 0);
            s.mode = MODE_INSERT.to_string();
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "open-line-above".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            let current_line = s.cursor.line;
            s.buffer = crate::buffer::insert_at(&s.buffer, current_line, 0, "\n");
            s.cursor = crate::cursor::new(current_line, 0);
            s.mode = MODE_INSERT.to_string();
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
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
            let reg = s.pending_register.take();
            set_register(s, reg, content, true);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "paste-below".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let reg = s.pending_register.take();
            if let Some((text, linewise)) = get_yank_content(s, reg) {
                push_undo(s);
                if linewise {
                    // Line-wise paste: insert on a new line below
                    let current_line = s.cursor.line;
                    let line_len = crate::buffer::get_line(&s.buffer, current_line)
                        .map(|l| l.trim_end_matches('\n').len())
                        .unwrap_or(0);
                    s.buffer = crate::buffer::insert_at(&s.buffer, current_line, line_len, "\n");
                    s.buffer = crate::buffer::insert_at(&s.buffer, current_line + 1, 0, &text);
                    s.cursor = crate::cursor::new(current_line + 1, 0);
                } else {
                    // Character-wise paste: insert after cursor position
                    let col = s.cursor.column + 1;
                    s.buffer = crate::buffer::insert_at(&s.buffer, s.cursor.line, col, &text);
                    // Cursor moves to end of pasted text - 1 (on last pasted char)
                    let end_col = col + text.len().saturating_sub(1);
                    s.cursor = crate::cursor::new(s.cursor.line, end_col);
                }
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
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
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
            s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
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
    // --- Search repeat commands: n (next) and N (prev) ---
    crate::command::register(
        &mut state.commands,
        "search-next".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(ref pattern) = s.search_pattern.clone() {
                let found = if s.search_forward {
                    crate::buffer::find_forward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
                } else {
                    crate::buffer::find_backward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
                };
                match found {
                    Some((line, col)) => {
                        s.cursor = crate::cursor::new(line, col);
                        s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
                        s.message = None;
                    }
                    None => {
                        s.message = Some(format!("Pattern not found: {}", pattern));
                    }
                }
            } else {
                s.message = Some("No previous search pattern".to_string());
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "search-prev".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(ref pattern) = s.search_pattern.clone() {
                // search-prev is the opposite direction of the last search
                let found = if s.search_forward {
                    crate::buffer::find_backward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
                } else {
                    crate::buffer::find_forward(&s.buffer, s.cursor.line, s.cursor.column, pattern)
                };
                match found {
                    Some((line, col)) => {
                        s.cursor = crate::cursor::new(line, col);
                        s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
                        s.message = None;
                    }
                    None => {
                        s.message = Some(format!("Pattern not found: {}", pattern));
                    }
                }
            } else {
                s.message = Some("No previous search pattern".to_string());
            }
            Ok(())
        }),
    );
    // --- Character find repeat commands: ; (repeat) and , (reverse) ---
    crate::command::register(
        &mut state.commands,
        "repeat-char-find".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some((kind, ch)) = s.last_char_find {
                if let Some(new_cursor) = execute_char_find(s.cursor, &s.buffer, kind, ch) {
                    s.cursor = new_cursor;
                    s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
                }
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "reverse-char-find".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some((kind, ch)) = s.last_char_find {
                let reversed_kind = reverse_char_find_kind(kind);
                if let Some(new_cursor) = execute_char_find(s.cursor, &s.buffer, reversed_kind, ch)
                {
                    s.cursor = new_cursor;
                    s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
                }
            }
            Ok(())
        }),
    );
    // --- Dot repeat: repeat last buffer-mutating command ---
    crate::command::register(
        &mut state.commands,
        "repeat-last-change".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(cmd_name) = s.last_edit_command.clone() {
                crate::command::execute(s, &cmd_name)?;
            }
            Ok(())
        }),
    );
    // --- Match bracket: jump to matching bracket (vim %) ---
    crate::command::register(
        &mut state.commands,
        "match-bracket".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(new_cursor) = crate::cursor::find_matching_bracket(s.cursor, &s.buffer) {
                s.cursor = new_cursor;
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
    // --- Indent / Unindent current line ---
    crate::command::register(
        &mut state.commands,
        "indent-line".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            s.buffer = crate::buffer::indent_line(&s.buffer, s.cursor.line, "    ");
            s.cursor = crate::cursor::ensure_within_bounds(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "unindent-line".to_string(),
        crate::command::CommandHandler::Native(|s| {
            push_undo(s);
            s.buffer = crate::buffer::unindent_line(&s.buffer, s.cursor.line, 4);
            s.cursor = crate::cursor::ensure_within_bounds(s.cursor, &s.buffer);
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    // --- Toggle case (vim ~): toggle char case and advance cursor ---
    crate::command::register(
        &mut state.commands,
        "toggle-case".to_string(),
        crate::command::CommandHandler::Native(|s| {
            let line_content = crate::buffer::get_line_content(&s.buffer, s.cursor.line);
            if line_content.is_empty() {
                return Ok(());
            }
            if s.cursor.column >= line_content.len() {
                return Ok(());
            }
            push_undo(s);
            s.buffer = crate::buffer::toggle_case_at(&s.buffer, s.cursor.line, s.cursor.column);
            // Advance cursor right (within line, like vim ~)
            let new_line_content = crate::buffer::get_line_content(&s.buffer, s.cursor.line);
            let max_col = new_line_content.len().saturating_sub(1);
            if s.cursor.column < max_col {
                s.cursor = crate::cursor::new(s.cursor.line, s.cursor.column + 1);
            }
            s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            Ok(())
        }),
    );
    // --- Visual mode commands ---
    crate::command::register(
        &mut state.commands,
        "enter-visual-mode".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.selection_start = Some(s.cursor);
            s.visual_line_mode = false;
            s.mode = MODE_VISUAL.to_string();
            s.active_keymaps = vec![format!("{}-mode", MODE_VISUAL)];
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "enter-visual-line-mode".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.selection_start = Some(s.cursor);
            s.visual_line_mode = true;
            s.mode = MODE_VISUAL.to_string();
            s.active_keymaps = vec![format!("{}-mode", MODE_VISUAL)];
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "exit-visual-mode".to_string(),
        crate::command::CommandHandler::Native(|s| {
            s.selection_start = None;
            s.visual_line_mode = false;
            s.mode = MODE_NORMAL.to_string();
            s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "visual-delete".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(anchor) = s.selection_start {
                let (from, to) = selection_range(anchor, s.cursor);
                let reg = s.pending_register.take();
                push_undo(s);
                if s.visual_line_mode {
                    // Line-wise: delete entire lines from min_line to max_line
                    let min_line = from.line;
                    let max_line = to.line;
                    let yanked = collect_lines_content(&s.buffer, min_line, max_line);
                    set_register(s, reg, yanked, true);
                    let mut buf = s.buffer.clone();
                    for _ in min_line..=max_line {
                        buf = crate::buffer::delete_line(&buf, min_line);
                    }
                    s.buffer = buf;
                    s.cursor = crate::cursor::ensure_within_bounds(
                        crate::cursor::new(min_line, 0),
                        &s.buffer,
                    );
                } else {
                    // Character-wise: inclusive selection, extend to by one char
                    let to_exclusive = advance_cursor_by_one(to, &s.buffer);
                    let text = crate::buffer::get_text_range(
                        &s.buffer,
                        from.line,
                        from.column,
                        to_exclusive.line,
                        to_exclusive.column,
                    );
                    set_register(s, reg, text, false);
                    s.buffer = crate::buffer::delete_char_range(
                        &s.buffer,
                        from.line,
                        from.column,
                        to_exclusive.line,
                        to_exclusive.column,
                    );
                    s.cursor = crate::cursor::ensure_within_bounds(from, &s.buffer);
                }
                s.selection_start = None;
                s.visual_line_mode = false;
                s.mode = MODE_NORMAL.to_string();
                s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "visual-yank".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(anchor) = s.selection_start {
                let (from, to) = selection_range(anchor, s.cursor);
                let reg = s.pending_register.take();
                if s.visual_line_mode {
                    // Line-wise: yank entire lines
                    let min_line = from.line;
                    let max_line = to.line;
                    let yanked = collect_lines_content(&s.buffer, min_line, max_line);
                    set_register(s, reg, yanked, true);
                    s.cursor = crate::cursor::new(min_line, 0);
                } else {
                    // Character-wise: inclusive selection
                    let to_exclusive = advance_cursor_by_one(to, &s.buffer);
                    let text = crate::buffer::get_text_range(
                        &s.buffer,
                        from.line,
                        from.column,
                        to_exclusive.line,
                        to_exclusive.column,
                    );
                    set_register(s, reg, text, false);
                    s.cursor = from;
                }
                s.selection_start = None;
                s.visual_line_mode = false;
                s.mode = MODE_NORMAL.to_string();
                s.active_keymaps = vec![format!("{}-mode", MODE_NORMAL)];
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
                s.message = Some("yanked".to_string());
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "visual-change".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some(anchor) = s.selection_start {
                let (from, to) = selection_range(anchor, s.cursor);
                let reg = s.pending_register.take();
                push_undo(s);
                if s.visual_line_mode {
                    // Line-wise: delete line contents but leave an empty line, enter insert
                    let min_line = from.line;
                    let max_line = to.line;
                    let yanked = collect_lines_content(&s.buffer, min_line, max_line);
                    set_register(s, reg, yanked, true);
                    // Delete lines from max down to min+1, keeping min_line
                    let mut buf = s.buffer.clone();
                    for _ in (min_line + 1)..=max_line {
                        buf = crate::buffer::delete_line(&buf, min_line + 1);
                    }
                    // Clear the remaining line's content (replace with empty)
                    let line_content = crate::buffer::get_line_content(&buf, min_line);
                    if !line_content.is_empty() {
                        buf = crate::buffer::delete_char_range(
                            &buf,
                            min_line,
                            0,
                            min_line,
                            line_content.len(),
                        );
                    }
                    s.buffer = buf;
                    s.cursor = crate::cursor::new(min_line, 0);
                } else {
                    // Character-wise: inclusive selection
                    let to_exclusive = advance_cursor_by_one(to, &s.buffer);
                    let text = crate::buffer::get_text_range(
                        &s.buffer,
                        from.line,
                        from.column,
                        to_exclusive.line,
                        to_exclusive.column,
                    );
                    set_register(s, reg, text, false);
                    s.buffer = crate::buffer::delete_char_range(
                        &s.buffer,
                        from.line,
                        from.column,
                        to_exclusive.line,
                        to_exclusive.column,
                    );
                    s.cursor = crate::cursor::ensure_within_bounds(from, &s.buffer);
                }
                s.selection_start = None;
                s.visual_line_mode = false;
                s.mode = MODE_INSERT.to_string();
                s.active_keymaps = vec![format!("{}-mode", MODE_INSERT)];
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
    // --- Increment / Decrement number under cursor (vim Ctrl-a / Ctrl-x) ---
    crate::command::register(
        &mut state.commands,
        "increment-number".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some((start, end, value)) =
                crate::buffer::find_number_at_cursor(&s.buffer, s.cursor.line, s.cursor.column)
            {
                push_undo(s);
                let new_value = value.saturating_add(1);
                s.buffer = crate::buffer::replace_number_in_line(
                    &s.buffer,
                    s.cursor.line,
                    start,
                    end,
                    new_value,
                );
                // Position cursor on the last digit of the new number
                let new_num_str = new_value.to_string();
                let new_end = start + new_num_str.len();
                s.cursor = crate::cursor::new(s.cursor.line, new_end.saturating_sub(1));
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
    crate::command::register(
        &mut state.commands,
        "decrement-number".to_string(),
        crate::command::CommandHandler::Native(|s| {
            if let Some((start, end, value)) =
                crate::buffer::find_number_at_cursor(&s.buffer, s.cursor.line, s.cursor.column)
            {
                push_undo(s);
                let new_value = value.saturating_sub(1);
                s.buffer = crate::buffer::replace_number_in_line(
                    &s.buffer,
                    s.cursor.line,
                    start,
                    end,
                    new_value,
                );
                // Position cursor on the last digit of the new number
                let new_num_str = new_value.to_string();
                let new_end = start + new_num_str.len();
                s.cursor = crate::cursor::new(s.cursor.line, new_end.saturating_sub(1));
                s.viewport = crate::viewport::adjust(s.viewport, &s.cursor);
            }
            Ok(())
        }),
    );
}

/// Computes the ordered selection range from two cursor positions.
///
/// Returns `(min, max)` where `min` is the position that comes first in the buffer
/// and `max` is the position that comes last. This ensures correct behavior
/// regardless of whether the user selected forward or backward.
pub fn selection_range(
    anchor: crate::cursor::Cursor,
    current: crate::cursor::Cursor,
) -> (crate::cursor::Cursor, crate::cursor::Cursor) {
    if (anchor.line, anchor.column) <= (current.line, current.column) {
        (anchor, current)
    } else {
        (current, anchor)
    }
}

/// Advances a cursor by one character position for exclusive range computation.
///
/// Visual selection is inclusive (the character under the cursor is part of the selection),
/// but `delete_char_range` and `get_text_range` use exclusive end positions.
/// This function moves the cursor one character forward to convert inclusive to exclusive.
fn advance_cursor_by_one(
    cursor: crate::cursor::Cursor,
    buffer: &crate::buffer::Buffer,
) -> crate::cursor::Cursor {
    let line_content = crate::buffer::get_line(buffer, cursor.line).unwrap_or("");
    let line_len = line_content.trim_end_matches('\n').len();
    if cursor.column < line_len {
        crate::cursor::new(cursor.line, cursor.column + 1)
    } else {
        // At end of line: advance to start of next line
        let total_lines = crate::buffer::line_count(buffer);
        if cursor.line + 1 < total_lines {
            crate::cursor::new(cursor.line + 1, 0)
        } else {
            // At very end of buffer: use buffer length as end
            crate::cursor::new(cursor.line, line_len)
        }
    }
}

/// Collects the content of lines from `min_line` to `max_line` (inclusive),
/// joining them with newlines. Each line's trailing newline is stripped.
///
/// Used by line-wise visual operators to build the yank register content.
fn collect_lines_content(
    buffer: &crate::buffer::Buffer,
    min_line: usize,
    max_line: usize,
) -> String {
    (min_line..=max_line)
        .map(|line| crate::buffer::get_line_content(buffer, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Executes a character find operation, returning the new cursor position if found.
///
/// This is a pure function that dispatches to the appropriate cursor find function
/// based on the CharFindKind.
pub fn execute_char_find(
    cursor: crate::cursor::Cursor,
    buffer: &crate::buffer::Buffer,
    kind: CharFindKind,
    target: char,
) -> Option<crate::cursor::Cursor> {
    match kind {
        CharFindKind::FindForward => crate::cursor::find_char_forward(cursor, buffer, target),
        CharFindKind::FindBackward => crate::cursor::find_char_backward(cursor, buffer, target),
        CharFindKind::TilForward => crate::cursor::til_char_forward(cursor, buffer, target),
        CharFindKind::TilBackward => crate::cursor::til_char_backward(cursor, buffer, target),
    }
}

/// Returns the reverse direction for a CharFindKind.
///
/// Used by the `,` (reverse-char-find) command to repeat the last find in
/// the opposite direction.
pub fn reverse_char_find_kind(kind: CharFindKind) -> CharFindKind {
    match kind {
        CharFindKind::FindForward => CharFindKind::FindBackward,
        CharFindKind::FindBackward => CharFindKind::FindForward,
        CharFindKind::TilForward => CharFindKind::TilBackward,
        CharFindKind::TilBackward => CharFindKind::TilForward,
    }
}

/// Returns true if the given character is a valid mark name ('a'-'z').
///
/// Marks are lowercase ASCII letters only; digits and other characters are rejected.
pub fn is_valid_mark_char(c: char) -> bool {
    c.is_ascii_lowercase()
}

/// Sets a named mark at the current cursor position.
///
/// If the mark already exists, its position is overwritten.
/// Only valid mark characters ('a'-'z') are accepted; invalid characters
/// are silently ignored.
pub fn set_mark(state: &mut EditorState, mark_char: char) {
    if is_valid_mark_char(mark_char) {
        state.marks.insert(mark_char, state.cursor);
    }
}

/// Jumps the cursor to the position stored in the named mark.
///
/// Returns `Ok(())` if the mark exists and the cursor was moved.
/// Returns `Err(message)` if the mark is not set, leaving the cursor unchanged.
/// Invalid mark characters produce an error message.
pub fn jump_to_mark(state: &mut EditorState, mark_char: char) -> Result<(), String> {
    if !is_valid_mark_char(mark_char) {
        return Err(format!("Invalid mark character: '{}'", mark_char));
    }
    match state.marks.get(&mark_char) {
        Some(&cursor_pos) => {
            state.cursor = cursor_pos;
            state.viewport = crate::viewport::adjust(state.viewport, &state.cursor);
            Ok(())
        }
        None => Err(format!("Mark '{}' not set", mark_char)),
    }
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
    let mut cursor_shapes = HashMap::new();
    cursor_shapes.insert(MODE_NORMAL.to_string(), "block".to_string());
    cursor_shapes.insert(MODE_INSERT.to_string(), "bar".to_string());
    cursor_shapes.insert(MODE_VISUAL.to_string(), "block".to_string());

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
        registers: HashMap::new(),
        pending_register: None,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        theme: crate::theme::new_theme(),
        named_themes: HashMap::new(),
        cursor_shapes,
        search_pattern: None,
        search_forward: true,
        last_char_find: None,
        last_edit_command: None,
        selection_start: None,
        visual_line_mode: false,
        marks: HashMap::new(),
        macro_registers: HashMap::new(),
        macro_recording: None,
        macro_buffer: Vec::new(),
        macro_replaying: false,
        last_macro_register: None,
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

    // -----------------------------------------------------------------------
    // Unit tests: cursor_shapes on EditorState
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_new_editor_state_then_default_cursor_shapes_are_block_and_bar() {
        let state = editor_state::new(80, 24);
        assert_eq!(
            state.cursor_shapes.get("normal"),
            Some(&"block".to_string())
        );
        assert_eq!(state.cursor_shapes.get("insert"), Some(&"bar".to_string()));
    }

    #[test]
    fn given_editor_in_normal_mode_when_cursor_shape_for_mode_then_returns_block() {
        let state = editor_state::new(80, 24);
        assert_eq!(editor_state::cursor_shape_for_mode(&state), "block");
    }

    #[test]
    fn given_editor_in_insert_mode_when_cursor_shape_for_mode_then_returns_bar() {
        let mut state = editor_state::new(80, 24);
        state.mode = "insert".to_string();
        assert_eq!(editor_state::cursor_shape_for_mode(&state), "bar");
    }

    #[test]
    fn given_editor_in_unknown_mode_when_cursor_shape_for_mode_then_returns_default() {
        let mut state = editor_state::new(80, 24);
        state.mode = "unknown-mode".to_string();
        assert_eq!(editor_state::cursor_shape_for_mode(&state), "default");
    }

    #[test]
    fn given_custom_cursor_shape_when_cursor_shape_for_mode_then_returns_custom_shape() {
        let mut state = editor_state::new(80, 24);
        state
            .cursor_shapes
            .insert("normal".to_string(), "blinking-bar".to_string());
        assert_eq!(editor_state::cursor_shape_for_mode(&state), "blinking-bar");
    }

    #[test]
    fn given_valid_shape_names_when_is_valid_cursor_shape_then_returns_true() {
        let valid_names = [
            "default",
            "block",
            "steady-block",
            "blinking-block",
            "bar",
            "steady-bar",
            "blinking-bar",
            "underline",
            "steady-underline",
            "blinking-underline",
        ];
        for name in &valid_names {
            assert!(
                editor_state::is_valid_cursor_shape(name),
                "'{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn given_invalid_shape_name_when_is_valid_cursor_shape_then_returns_false() {
        assert!(!editor_state::is_valid_cursor_shape("triangle"));
        assert!(!editor_state::is_valid_cursor_shape(""));
        assert!(!editor_state::is_valid_cursor_shape("BLOCK"));
    }

    // -----------------------------------------------------------------------
    // Unit tests: repeat-last-change (dot command)
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_last_edit_is_delete_char_when_repeat_last_change_then_another_char_deleted() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Hello");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Execute delete-char-at-cursor (deletes 'H')
        let result = command::execute(&mut state, "delete-char-at-cursor");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "ello");

        // Record it as last edit command (normally done by event loop)
        state.last_edit_command = Some("delete-char-at-cursor".to_string());

        // When: repeat-last-change (dot)
        let result = command::execute(&mut state, "repeat-last-change");
        assert!(result.is_ok());

        // Then: another character deleted
        assert_eq!(buffer::content(&state.buffer), "llo");
    }

    #[test]
    fn given_last_edit_is_delete_line_when_repeat_last_change_then_another_line_deleted() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("First\nSecond\nThird");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Execute delete-line (deletes "First")
        let result = command::execute(&mut state, "delete-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Second\nThird");

        // Record it as last edit command
        state.last_edit_command = Some("delete-line".to_string());

        // When: repeat-last-change (dot)
        let result = command::execute(&mut state, "repeat-last-change");
        assert!(result.is_ok());

        // Then: another line deleted
        assert_eq!(buffer::content(&state.buffer), "Third");
    }

    #[test]
    fn given_no_prior_edit_when_repeat_last_change_then_noop() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("Unchanged");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // last_edit_command is None by default

        // When: repeat-last-change (dot) with no prior edit
        let result = command::execute(&mut state, "repeat-last-change");
        assert!(result.is_ok());

        // Then: buffer unchanged
        assert_eq!(buffer::content(&state.buffer), "Unchanged");
    }

    #[test]
    fn given_last_edit_is_join_lines_when_repeat_last_change_then_another_join_performed() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("A\nB\nC\nD");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Execute join-lines (joins A and B)
        let result = command::execute(&mut state, "join-lines");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "A B\nC\nD");

        // Record it as last edit command
        state.last_edit_command = Some("join-lines".to_string());

        // When: repeat-last-change (dot)
        let result = command::execute(&mut state, "repeat-last-change");
        assert!(result.is_ok());

        // Then: next lines joined
        assert_eq!(buffer::content(&state.buffer), "A B C\nD");
    }

    // -----------------------------------------------------------------------
    // Unit tests: indent-line and unindent-line commands
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_with_content_when_indent_line_command_then_4_spaces_prepended_and_undo_saved() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("hello\nworld");
        state.cursor = crate::cursor::new(0, 2);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "indent-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "    hello\nworld");
        // Undo stack should have an entry
        assert!(!state.undo_stack.is_empty());
    }

    #[test]
    fn given_line_with_4_spaces_when_unindent_line_command_then_spaces_removed_and_undo_saved() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("    hello\nworld");
        state.cursor = crate::cursor::new(0, 5);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "unindent-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "hello\nworld");
        assert!(!state.undo_stack.is_empty());
    }

    #[test]
    fn given_line_with_no_indent_when_unindent_line_command_then_buffer_unchanged() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("hello");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "unindent-line");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "hello");
    }

    // -----------------------------------------------------------------------
    // Marks: pure domain function tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_valid_lowercase_letter_when_is_valid_mark_char_then_true() {
        assert!(editor_state::is_valid_mark_char('a'));
        assert!(editor_state::is_valid_mark_char('m'));
        assert!(editor_state::is_valid_mark_char('z'));
    }

    #[test]
    fn given_invalid_chars_when_is_valid_mark_char_then_false() {
        assert!(!editor_state::is_valid_mark_char('A'));
        assert!(!editor_state::is_valid_mark_char('1'));
        assert!(!editor_state::is_valid_mark_char(' '));
        assert!(!editor_state::is_valid_mark_char('!'));
    }

    #[test]
    fn given_editor_when_set_mark_then_mark_stored_at_cursor_position() {
        let mut state = editor_state::new(80, 24);
        state.cursor = crate::cursor::new(5, 10);

        editor_state::set_mark(&mut state, 'a');

        assert_eq!(state.marks.get(&'a'), Some(&crate::cursor::new(5, 10)));
    }

    #[test]
    fn given_existing_mark_when_set_mark_same_char_then_position_overwritten() {
        let mut state = editor_state::new(80, 24);
        state.cursor = crate::cursor::new(1, 2);
        editor_state::set_mark(&mut state, 'a');

        state.cursor = crate::cursor::new(3, 4);
        editor_state::set_mark(&mut state, 'a');

        assert_eq!(state.marks.get(&'a'), Some(&crate::cursor::new(3, 4)));
    }

    #[test]
    fn given_invalid_char_when_set_mark_then_nothing_stored() {
        let mut state = editor_state::new(80, 24);
        state.cursor = crate::cursor::new(1, 2);

        editor_state::set_mark(&mut state, '1');

        assert!(state.marks.is_empty());
    }

    #[test]
    fn given_existing_mark_when_jump_to_mark_then_cursor_moved() {
        let mut state = editor_state::new(80, 24);
        state.buffer = crate::buffer::Buffer::from_string("aaa\nbbb\nccc");
        state.cursor = crate::cursor::new(2, 1);
        editor_state::set_mark(&mut state, 'b');

        state.cursor = crate::cursor::new(0, 0);
        let result = editor_state::jump_to_mark(&mut state, 'b');

        assert!(result.is_ok());
        assert_eq!(state.cursor.line, 2);
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_unset_mark_when_jump_to_mark_then_error_returned() {
        let mut state = editor_state::new(80, 24);
        state.cursor = crate::cursor::new(0, 0);

        let result = editor_state::jump_to_mark(&mut state, 'x');

        assert_eq!(result, Err("Mark 'x' not set".to_string()));
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn given_invalid_char_when_jump_to_mark_then_error_returned() {
        let mut state = editor_state::new(80, 24);
        state.cursor = crate::cursor::new(0, 0);

        let result = editor_state::jump_to_mark(&mut state, '1');

        assert_eq!(result, Err("Invalid mark character: '1'".to_string()));
    }

    #[test]
    fn given_new_editor_when_created_then_marks_empty() {
        let state = editor_state::new(80, 24);
        assert!(state.marks.is_empty());
    }

    // -----------------------------------------------------------------------
    // Unit tests: toggle-case command (vim ~)
    // Test Budget: 6 behaviors x 2 = 12 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_lowercase_char_when_toggle_case_then_uppercased_and_cursor_advances() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("hello");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "toggle-case");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "Hello");
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_uppercase_char_when_toggle_case_then_lowercased_and_cursor_advances() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("HELLO");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "toggle-case");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "hELLO");
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_non_letter_when_toggle_case_then_unchanged_and_cursor_advances() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("1abc");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "toggle-case");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "1abc");
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_cursor_at_last_char_when_toggle_case_then_toggled_and_cursor_stays() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("ab");
        state.cursor = crate::cursor::new(0, 1); // cursor on 'b', last char
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "toggle-case");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "aB");
        // Cursor stays at last char position (cannot advance further on line)
        assert_eq!(state.cursor.column, 1);
    }

    #[test]
    fn given_empty_buffer_when_toggle_case_then_noop() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "toggle-case");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "");
        assert_eq!(state.cursor.column, 0);
        // No undo snapshot pushed for empty buffer
        assert!(state.undo_stack.is_empty());
    }

    #[test]
    fn given_toggle_case_when_undo_then_original_restored() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("hello");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        // Toggle case: h -> H
        command::execute(&mut state, "toggle-case").unwrap();
        assert_eq!(buffer::content(&state.buffer), "Hello");

        // Undo: should restore to "hello"
        command::execute(&mut state, "undo").unwrap();
        assert_eq!(buffer::content(&state.buffer), "hello");
    }

    // -----------------------------------------------------------------------
    // Unit tests: increment-number and decrement-number commands
    // Test Budget: 9 behaviors x 2 = 18 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_on_number_when_increment_then_number_increases_by_one() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("count=42");
        state.cursor = crate::cursor::new(0, 6);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "increment-number");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "count=43");
    }

    #[test]
    fn given_cursor_on_number_when_decrement_then_number_decreases_by_one() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("count=42");
        state.cursor = crate::cursor::new(0, 6);
        editor_state::register_builtin_commands(&mut state);

        let result = command::execute(&mut state, "decrement-number");
        assert!(result.is_ok());
        assert_eq!(buffer::content(&state.buffer), "count=41");
    }

    #[test]
    fn given_zero_when_increment_then_becomes_one() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("0");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "1");
    }

    #[test]
    fn given_zero_when_decrement_then_becomes_negative_one() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("0");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "decrement-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "-1");
    }

    #[test]
    fn given_negative_number_when_increment_then_moves_toward_zero() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("-5");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "-4");
    }

    #[test]
    fn given_cursor_before_number_when_increment_then_finds_and_increments() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("count=42");
        state.cursor = crate::cursor::new(0, 0); // cursor at 'c', before the number
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "count=43");
    }

    #[test]
    fn given_no_number_on_line_when_increment_then_no_change() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("hello world");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "hello world");
        // No undo snapshot pushed when nothing changed
        assert!(state.undo_stack.is_empty());
    }

    #[test]
    fn given_number_at_start_of_line_when_increment_then_works() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("42 apples");
        state.cursor = crate::cursor::new(0, 0);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "43 apples");
    }

    #[test]
    fn given_number_at_end_of_line_when_increment_then_works() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("apples 42");
        state.cursor = crate::cursor::new(0, 7);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "apples 43");
    }

    #[test]
    fn given_increment_when_undo_then_original_restored() {
        use crate::buffer;

        let mut state = editor_state::new(80, 24);
        state.buffer = buffer::Buffer::from_string("count=42");
        state.cursor = crate::cursor::new(0, 6);
        editor_state::register_builtin_commands(&mut state);

        command::execute(&mut state, "increment-number").unwrap();
        assert_eq!(buffer::content(&state.buffer), "count=43");

        command::execute(&mut state, "undo").unwrap();
        assert_eq!(buffer::content(&state.buffer), "count=42");
    }
}
