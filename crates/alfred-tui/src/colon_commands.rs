//! Colon command parsing and dispatch for the Alfred editor.
//!
//! This module extracts colon command handling from app.rs into focused,
//! testable functions. The parsing layer is pure (no state mutation),
//! while execution functions receive `&mut EditorState` to apply effects.

use alfred_core::editor_state::EditorState;

use crate::input::{DeferredAction, InputState};

// ---------------------------------------------------------------------------
// Parsed command representation
// ---------------------------------------------------------------------------

/// A parsed colon command -- the pure result of analyzing the command string.
///
/// This enum captures WHAT command was typed without performing any side effects.
/// Execution is handled separately by `execute_colon_command`.
#[derive(Debug, PartialEq)]
pub(crate) enum ColonCommand<'a> {
    /// Quit the editor (`:q`, `:quit`)
    Quit,
    /// Force quit without saving (`:q!`)
    ForceQuit,
    /// Save and quit (`:wq`)
    SaveAndQuit,
    /// Save buffer to current or specified path (`:w`, `:w path`)
    Save(Option<String>),
    /// Open a file (`:e path`)
    OpenFile(String),
    /// Evaluate a Lisp expression (`:eval expr`)
    Eval(String),
    /// Substitute command (`:s/old/new/g` or `:%s/old/new/g`)
    Substitute {
        whole_buffer: bool,
        pattern: &'a str,
        replacement: &'a str,
        global: bool,
    },
    /// Global delete command (`:g/pattern/d`, `:v/pattern/d`, `:g!/pattern/d`)
    GlobalDelete { pattern: &'a str, invert: bool },
    /// Invalid substitute syntax (`:s` without proper `/old/new/` pattern)
    InvalidSubstitute,
    /// Invalid global command syntax (`:g/` without proper `/pattern/d` format)
    InvalidGlobalCommand,
    /// A registered command name (fallback)
    RegisteredCommand(String),
    /// Unknown command
    Unknown(String),
}

// ---------------------------------------------------------------------------
// Pure parsing functions
// ---------------------------------------------------------------------------

/// Parse a colon command string into a `ColonCommand`.
///
/// This is a pure function -- no state access, no side effects.
/// The `has_registered_command` predicate allows the caller to inject
/// command registry lookup without coupling to `EditorState`.
pub(crate) fn parse_colon_command<'a, F>(
    command: &'a str,
    has_registered_command: F,
) -> ColonCommand<'a>
where
    F: Fn(&str) -> bool,
{
    match command {
        "q" | "quit" => ColonCommand::Quit,
        "q!" => ColonCommand::ForceQuit,
        "wq" => ColonCommand::SaveAndQuit,
        "w" => ColonCommand::Save(None),
        cmd if cmd.starts_with("w ") => {
            let path = cmd.strip_prefix("w ").unwrap().trim().to_string();
            ColonCommand::Save(Some(path))
        }
        cmd if cmd.starts_with("e ") => {
            let path = cmd.strip_prefix("e ").unwrap().trim().to_string();
            ColonCommand::OpenFile(path)
        }
        cmd if cmd.starts_with("eval ") => {
            let expression = cmd.strip_prefix("eval ").unwrap().to_string();
            ColonCommand::Eval(expression)
        }
        cmd if cmd.starts_with("g/") || cmd.starts_with("g!/") || cmd.starts_with("v/") => {
            match parse_global_command(cmd) {
                Some((pattern, invert)) => ColonCommand::GlobalDelete { pattern, invert },
                None => ColonCommand::InvalidGlobalCommand,
            }
        }
        cmd if cmd.starts_with("s/") || cmd.starts_with("%s/") => {
            let whole_buffer = cmd.starts_with('%');
            let args = if whole_buffer {
                &cmd[2..] // skip "%s"
            } else {
                &cmd[1..] // skip "s"
            };
            match parse_substitute_pattern(args) {
                Some((pattern, replacement, global)) if !pattern.is_empty() => {
                    ColonCommand::Substitute {
                        whole_buffer,
                        pattern,
                        replacement,
                        global,
                    }
                }
                _ => ColonCommand::InvalidSubstitute,
            }
        }
        cmd => {
            if has_registered_command(cmd) {
                ColonCommand::RegisteredCommand(cmd.to_string())
            } else {
                ColonCommand::Unknown(cmd.to_string())
            }
        }
    }
}

/// Parses a substitute command pattern like `/old/new/` or `/old/new/g`.
///
/// Returns `Some((pattern, replacement, global))` on success, or `None` if
/// the command does not match the expected format.
fn parse_substitute_pattern(args: &str) -> Option<(&str, &str, bool)> {
    // args should start with '/' -- e.g., "/old/new/" or "/old/new/g"
    if !args.starts_with('/') {
        return None;
    }
    let rest = &args[1..]; // skip leading '/'

    // Find the second '/' (separator between pattern and replacement)
    let second_slash = rest.find('/')?;
    let pattern = &rest[..second_slash];
    let after_pattern = &rest[second_slash + 1..];

    // Find the third '/' (trailing delimiter) -- may or may not be present
    let (replacement, flags) = match after_pattern.find('/') {
        Some(pos) => (&after_pattern[..pos], &after_pattern[pos + 1..]),
        None => (after_pattern, ""),
    };

    let global = flags.contains('g');
    Some((pattern, replacement, global))
}

/// Parses a global command pattern like `g/pattern/d`, `g!/pattern/d`, or `v/pattern/d`.
///
/// Returns `Some((pattern, invert))` on success, or `None` if the command format is invalid.
/// `invert` is true for `:v/` and `:g!/` forms (delete non-matching lines).
fn parse_global_command(command: &str) -> Option<(&str, bool)> {
    let (rest, invert) = if let Some(r) = command.strip_prefix("g!/") {
        (r, true)
    } else if let Some(r) = command.strip_prefix("v/") {
        (r, true)
    } else if let Some(r) = command.strip_prefix("g/") {
        (r, false)
    } else {
        return None;
    };

    // rest should be "pattern/d"
    let slash_pos = rest.find('/')?;
    let pattern = &rest[..slash_pos];
    let action = &rest[slash_pos + 1..];

    if pattern.is_empty() {
        return None;
    }

    if action != "d" {
        return None;
    }

    Some((pattern, invert))
}

// ---------------------------------------------------------------------------
// Command execution (state mutation)
// ---------------------------------------------------------------------------

/// Execute a parsed colon command against editor state.
///
/// Returns the new input state and a deferred action for commands that
/// need Lisp evaluation or registered command execution (avoiding RefCell
/// double-borrow panics in the caller).
pub(crate) fn execute_colon_command(
    state: &mut EditorState,
    command: &str,
) -> (InputState, DeferredAction) {
    let parsed = parse_colon_command(command, |cmd_name| {
        alfred_core::command::lookup(&state.commands, cmd_name).is_some()
    });

    match parsed {
        ColonCommand::Quit => {
            if state.buffer.is_modified() {
                state.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                (InputState::Normal, DeferredAction::None)
            } else {
                state.running = false;
                (InputState::Normal, DeferredAction::None)
            }
        }
        ColonCommand::ForceQuit => {
            state.running = false;
            (InputState::Normal, DeferredAction::None)
        }
        ColonCommand::SaveAndQuit => (InputState::Normal, DeferredAction::SaveAndQuit),
        ColonCommand::Save(path) => (InputState::Normal, DeferredAction::SaveBuffer(path)),
        ColonCommand::OpenFile(path) => (InputState::Normal, DeferredAction::OpenFile(path)),
        ColonCommand::Eval(expression) => (InputState::Normal, DeferredAction::Eval(expression)),
        ColonCommand::Substitute {
            whole_buffer,
            pattern,
            replacement,
            global,
        } => execute_substitute(state, whole_buffer, pattern, replacement, global),
        ColonCommand::GlobalDelete { pattern, invert } => {
            execute_global_delete(state, pattern, invert)
        }
        ColonCommand::InvalidSubstitute => {
            state.message = Some(
                "Invalid substitute command. Usage: :s/old/new/[g] or :%s/old/new/[g]".to_string(),
            );
            (InputState::Normal, DeferredAction::None)
        }
        ColonCommand::InvalidGlobalCommand => {
            state.message =
                Some("Invalid global command. Usage: :g/pattern/d or :v/pattern/d".to_string());
            (InputState::Normal, DeferredAction::None)
        }
        ColonCommand::RegisteredCommand(cmd_name) => {
            (InputState::Normal, DeferredAction::ExecCommand(cmd_name))
        }
        ColonCommand::Unknown(cmd_text) => {
            state.message = Some(format!("Unknown command: {}", cmd_text));
            (InputState::Normal, DeferredAction::None)
        }
    }
}

/// Execute a substitute command (`:s/old/new/` or `:%s/old/new/g`).
///
/// Performs the substitution on the buffer, pushes undo, and sets a status message.
fn execute_substitute(
    state: &mut EditorState,
    whole_buffer: bool,
    pattern: &str,
    replacement: &str,
    global: bool,
) -> (InputState, DeferredAction) {
    alfred_core::editor_state::push_undo(state);

    if whole_buffer {
        let (new_buffer, count) =
            alfred_core::buffer::substitute_all(&state.buffer, pattern, replacement);
        if count == 0 {
            state.message = Some("Pattern not found".to_string());
        } else {
            state.buffer = new_buffer;
            state.message = Some(format!("{} substitution(s) made", count));
        }
    } else {
        let cursor_line = state.cursor.line;
        let old_content = alfred_core::facade::buffer_content(state);
        let new_buffer = alfred_core::buffer::substitute_in_line(
            &state.buffer,
            cursor_line,
            pattern,
            replacement,
            global,
        );
        let new_content = alfred_core::buffer::content(&new_buffer);
        if old_content == new_content {
            state.message = Some("Pattern not found".to_string());
        } else {
            state.buffer = new_buffer;
            let scope = if global { "line (all)" } else { "line (first)" };
            state.message = Some(format!("Substituted on {}", scope));
        }
    }

    (InputState::Normal, DeferredAction::None)
}

/// Execute a global delete command (`:g/pattern/d` or `:v/pattern/d`).
///
/// Performs line deletion on the buffer, pushes undo, and sets a status message.
fn execute_global_delete(
    state: &mut EditorState,
    pattern: &str,
    invert: bool,
) -> (InputState, DeferredAction) {
    alfred_core::editor_state::push_undo(state);

    let (new_buffer, count) =
        alfred_core::buffer::delete_lines_matching(&state.buffer, pattern, invert);

    if count == 0 {
        state.message = Some("Pattern not found".to_string());
    } else {
        state.buffer = new_buffer;
        state.message = Some(format!("{} line(s) deleted", count));
    }

    (InputState::Normal, DeferredAction::None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: a predicate that always returns false (no registered commands).
    fn no_registered_commands(_: &str) -> bool {
        false
    }

    /// Helper: a predicate that recognizes specific command names.
    fn with_commands<'a>(known: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |cmd| known.contains(&cmd)
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: quit variants
    // -----------------------------------------------------------------------

    #[test]
    fn parse_q_returns_quit() {
        let result = parse_colon_command("q", no_registered_commands);
        assert_eq!(result, ColonCommand::Quit);
    }

    #[test]
    fn parse_quit_returns_quit() {
        let result = parse_colon_command("quit", no_registered_commands);
        assert_eq!(result, ColonCommand::Quit);
    }

    #[test]
    fn parse_q_bang_returns_force_quit() {
        let result = parse_colon_command("q!", no_registered_commands);
        assert_eq!(result, ColonCommand::ForceQuit);
    }

    #[test]
    fn parse_wq_returns_save_and_quit() {
        let result = parse_colon_command("wq", no_registered_commands);
        assert_eq!(result, ColonCommand::SaveAndQuit);
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: save variants
    // -----------------------------------------------------------------------

    #[test]
    fn parse_w_returns_save_none() {
        let result = parse_colon_command("w", no_registered_commands);
        assert_eq!(result, ColonCommand::Save(None));
    }

    #[test]
    fn parse_w_with_path_returns_save_some() {
        let result = parse_colon_command("w /tmp/out.txt", no_registered_commands);
        assert_eq!(result, ColonCommand::Save(Some("/tmp/out.txt".to_string())));
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: open file
    // -----------------------------------------------------------------------

    #[test]
    fn parse_e_with_path_returns_open_file() {
        let result = parse_colon_command("e /tmp/in.txt", no_registered_commands);
        assert_eq!(result, ColonCommand::OpenFile("/tmp/in.txt".to_string()));
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: eval
    // -----------------------------------------------------------------------

    #[test]
    fn parse_eval_returns_eval_with_expression() {
        let result = parse_colon_command("eval (+ 1 2)", no_registered_commands);
        assert_eq!(result, ColonCommand::Eval("(+ 1 2)".to_string()));
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: substitute
    // -----------------------------------------------------------------------

    #[test]
    fn parse_s_with_pattern_returns_substitute() {
        let result = parse_colon_command("s/foo/bar/", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::Substitute {
                whole_buffer: false,
                pattern: "foo",
                replacement: "bar",
                global: false,
            }
        );
    }

    #[test]
    fn parse_s_global_returns_substitute_with_global_flag() {
        let result = parse_colon_command("s/foo/bar/g", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::Substitute {
                whole_buffer: false,
                pattern: "foo",
                replacement: "bar",
                global: true,
            }
        );
    }

    #[test]
    fn parse_percent_s_returns_whole_buffer_substitute() {
        let result = parse_colon_command("%s/old/new/g", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::Substitute {
                whole_buffer: true,
                pattern: "old",
                replacement: "new",
                global: true,
            }
        );
    }

    #[test]
    fn parse_s_with_empty_replacement_returns_substitute() {
        let result = parse_colon_command("s/foo//g", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::Substitute {
                whole_buffer: false,
                pattern: "foo",
                replacement: "",
                global: true,
            }
        );
    }

    #[test]
    fn parse_bare_s_falls_through_to_unknown() {
        // "s" alone does not start with "s/" so it goes to the fallback
        let result = parse_colon_command("s", no_registered_commands);
        assert_eq!(result, ColonCommand::Unknown("s".to_string()));
    }

    #[test]
    fn parse_s_with_empty_pattern_returns_invalid_substitute() {
        let result = parse_colon_command("s//bar/", no_registered_commands);
        assert_eq!(result, ColonCommand::InvalidSubstitute);
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: global delete
    // -----------------------------------------------------------------------

    #[test]
    fn parse_g_delete_returns_global_delete() {
        let result = parse_colon_command("g/TODO/d", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::GlobalDelete {
                pattern: "TODO",
                invert: false,
            }
        );
    }

    #[test]
    fn parse_v_delete_returns_inverted_global_delete() {
        let result = parse_colon_command("v/keep/d", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::GlobalDelete {
                pattern: "keep",
                invert: true,
            }
        );
    }

    #[test]
    fn parse_g_bang_delete_returns_inverted_global_delete() {
        let result = parse_colon_command("g!/keep/d", no_registered_commands);
        assert_eq!(
            result,
            ColonCommand::GlobalDelete {
                pattern: "keep",
                invert: true,
            }
        );
    }

    #[test]
    fn parse_g_with_invalid_action_returns_invalid_global_command() {
        let result = parse_colon_command("g/pattern/x", no_registered_commands);
        assert_eq!(result, ColonCommand::InvalidGlobalCommand);
    }

    #[test]
    fn parse_g_with_missing_action_returns_invalid_global_command() {
        let result = parse_colon_command("g/pattern/", no_registered_commands);
        assert_eq!(result, ColonCommand::InvalidGlobalCommand);
    }

    // -----------------------------------------------------------------------
    // parse_colon_command: registered command fallback
    // -----------------------------------------------------------------------

    #[test]
    fn parse_registered_command_returns_registered_command() {
        let known = ["cursor-left", "cursor-right"];
        let result = parse_colon_command("cursor-left", with_commands(&known));
        assert_eq!(
            result,
            ColonCommand::RegisteredCommand("cursor-left".to_string())
        );
    }

    #[test]
    fn parse_unknown_command_returns_unknown() {
        let result = parse_colon_command("foobar", no_registered_commands);
        assert_eq!(result, ColonCommand::Unknown("foobar".to_string()));
    }

    // -----------------------------------------------------------------------
    // parse_substitute_pattern: internal helper
    // -----------------------------------------------------------------------

    #[test]
    fn parse_substitute_pattern_basic() {
        assert_eq!(
            parse_substitute_pattern("/old/new/"),
            Some(("old", "new", false))
        );
    }

    #[test]
    fn parse_substitute_pattern_global() {
        assert_eq!(
            parse_substitute_pattern("/old/new/g"),
            Some(("old", "new", true))
        );
    }

    #[test]
    fn parse_substitute_pattern_no_trailing_slash() {
        assert_eq!(
            parse_substitute_pattern("/old/new"),
            Some(("old", "new", false))
        );
    }

    #[test]
    fn parse_substitute_pattern_no_leading_slash() {
        assert_eq!(parse_substitute_pattern("old/new/"), None);
    }

    #[test]
    fn parse_substitute_pattern_empty_returns_none() {
        assert_eq!(parse_substitute_pattern(""), None);
    }

    // -----------------------------------------------------------------------
    // parse_global_command: internal helper
    // -----------------------------------------------------------------------

    #[test]
    fn parse_global_command_basic() {
        assert_eq!(parse_global_command("g/hello/d"), Some(("hello", false)));
    }

    #[test]
    fn parse_global_command_v_form() {
        assert_eq!(parse_global_command("v/hello/d"), Some(("hello", true)));
    }

    #[test]
    fn parse_global_command_g_bang_form() {
        assert_eq!(parse_global_command("g!/hello/d"), Some(("hello", true)));
    }

    #[test]
    fn parse_global_command_empty_pattern() {
        assert_eq!(parse_global_command("g//d"), None);
    }

    #[test]
    fn parse_global_command_wrong_action() {
        assert_eq!(parse_global_command("g/hello/x"), None);
    }

    #[test]
    fn parse_global_command_no_action() {
        assert_eq!(parse_global_command("g/hello/"), None);
    }
}
