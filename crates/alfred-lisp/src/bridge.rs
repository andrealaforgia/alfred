//! Bridge: registers Rust-implemented primitives into the Lisp runtime.
//!
//! The bridge connects the Lisp runtime to Alfred's editor state by
//! registering native closures that can read and mutate the buffer and cursor.
//! All primitives receive shared mutable access to `EditorState` via
//! `Rc<RefCell<EditorState>>`.

use std::cell::RefCell;
use std::rc::Rc;

use rust_lisp::model::{Env, List, RuntimeError, Symbol, Value};

use alfred_core::buffer;
use alfred_core::command;
use alfred_core::cursor;
use alfred_core::editor_state::EditorState;
use alfred_core::hook;
use alfred_core::viewport;

use crate::runtime::LispRuntime;

/// Registers a native closure into the Lisp environment under the given name.
fn define_native_closure<F>(env: &Rc<RefCell<Env>>, name: &str, closure: F)
where
    F: Fn(Rc<RefCell<Env>>, Vec<Value>) -> Result<Value, RuntimeError> + 'static,
{
    env.borrow_mut().define(
        Symbol(name.to_string()),
        Value::NativeClosure(Rc::new(RefCell::new(closure))),
    );
}

/// Extracts a required string argument from the args list, returning a clear error on type mismatch or missing arg.
fn extract_string_arg(args: &[Value], fn_name: &str) -> Result<String, RuntimeError> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(RuntimeError {
            msg: format!("{}: expected string argument, got {}", fn_name, other),
        }),
        None => Err(RuntimeError {
            msg: format!("{}: expected 1 argument, got 0", fn_name),
        }),
    }
}

/// Registers all core buffer, cursor, message, and mode primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(buffer-insert text)` -- insert text at cursor position
/// - `(buffer-delete)` -- delete character at cursor position
/// - `(buffer-content)` -- return buffer text as string
/// - `(cursor-position)` -- return (line column) as a list
/// - `(cursor-move direction [count])` -- move cursor by direction and optional count
/// - `(message text)` -- set the editor message line
/// - `(current-mode)` -- return the current mode name as a string
/// - `(set-mode name)` -- set the editor mode and switch active keymap
/// - `(buffer-filename)` -- return the buffer's filename or empty string if unnamed
/// - `(buffer-modified?)` -- return T if buffer modified, F otherwise
/// - `(save-buffer)` -- save buffer to its file path; `(save-buffer "path")` saves to explicit path
pub fn register_core_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_buffer_insert(env.clone(), state.clone());
    register_buffer_delete(env.clone(), state.clone());
    register_buffer_content(env.clone(), state.clone());
    register_cursor_position(env.clone(), state.clone());
    register_cursor_move(env.clone(), state.clone());
    register_message(env.clone(), state.clone());
    register_current_mode(env.clone(), state.clone());
    register_buffer_filename(env.clone(), state.clone());
    register_buffer_modified(env.clone(), state.clone());
    register_save_buffer(env.clone(), state.clone());
    register_set_mode(env, state);
}

/// Registers the `define-command` Lisp primitive.
///
/// Usage: `(define-command "name" callback-fn)`
///
/// Registers a Lisp function as a named command in the editor's CommandRegistry.
/// When the command is later executed, the callback is invoked via the Lisp runtime.
pub fn register_define_command(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();
    let lisp_env = runtime.env();

    define_native_closure(&env, "define-command", move |_env, args| {
        let name = extract_string_arg(&args, "define-command")?;

        let callback = args.get(1).ok_or_else(|| RuntimeError {
            msg: "define-command: expected 2 arguments (name, callback), got 1".to_string(),
        })?;

        // Verify the callback is callable (lambda, native func, or native closure)
        match callback {
            Value::Lambda(_) | Value::NativeFunc(_) | Value::NativeClosure(_) => {}
            other => {
                return Err(RuntimeError {
                    msg: format!(
                        "define-command: expected callable as second argument, got {}",
                        other
                    ),
                });
            }
        }

        let callback_value = callback.clone();
        let call_env = lisp_env.clone();

        let handler = command::CommandHandler::Dynamic(Rc::new(move |_editor_state| {
            // Build a call expression: (callback)
            let call_list: List = vec![callback_value.clone()].into_iter().collect();
            let call_expr = Value::List(call_list);
            rust_lisp::interpreter::eval(call_env.clone(), &call_expr)
                .map(|_| ())
                .map_err(|e| alfred_core::error::AlfredError::CommandNotFound {
                    name: format!("lisp callback error: {}", e.msg),
                })
        }));

        command::register(&mut state.borrow_mut().commands, name, handler);

        Ok(Value::NIL)
    });
}

/// Shared error buffer for hook callbacks to report errors without
/// needing to borrow EditorState during dispatch.
type HookErrorBuffer = Rc<RefCell<Vec<String>>>;

/// Registers hook primitives (`add-hook`, `dispatch-hook`, `remove-hook`) into the runtime.
///
/// These primitives bridge the Lisp runtime to the Rust HookRegistry,
/// allowing plugins to register Lisp callbacks as hooks and dispatch them.
///
/// After calling this, the following Lisp functions become available:
/// - `(add-hook "name" callback-fn)` -- register a Lisp callback for a named hook
/// - `(dispatch-hook "name" arg1 arg2 ...)` -- dispatch a hook, returning results as a list
/// - `(remove-hook "name" hook-id)` -- unregister a hook callback by its ID
pub fn register_hook_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();
    let hook_errors: HookErrorBuffer = Rc::new(RefCell::new(Vec::new()));

    register_add_hook(
        env.clone(),
        runtime.env(),
        state.clone(),
        hook_errors.clone(),
    );
    register_dispatch_hook(env.clone(), state.clone(), hook_errors);
    register_remove_hook(env, state);
}

/// Registers `add-hook`: registers a Lisp callback for a named hook.
///
/// Usage: `(add-hook "hook-name" callback-fn)`
///
/// Returns the HookId (as integer) for potential unregistration.
fn register_add_hook(
    env: Rc<RefCell<Env>>,
    lisp_env: Rc<RefCell<Env>>,
    state: Rc<RefCell<EditorState>>,
    hook_errors: HookErrorBuffer,
) {
    define_native_closure(&env, "add-hook", move |_env, args| {
        let hook_name = extract_string_arg(&args, "add-hook")?;

        let callback = args.get(1).ok_or_else(|| RuntimeError {
            msg: "add-hook: expected 2 arguments (name, callback), got 1".to_string(),
        })?;

        // Verify the callback is callable
        match callback {
            Value::Lambda(_) | Value::NativeFunc(_) | Value::NativeClosure(_) => {}
            other => {
                return Err(RuntimeError {
                    msg: format!(
                        "add-hook: expected callable as second argument, got {}",
                        other
                    ),
                });
            }
        }

        let callback_value = callback.clone();
        let call_env = lisp_env.clone();
        let error_buf = hook_errors.clone();

        // Wrap the Lisp callback in a Rust closure compatible with HookRegistry
        let wrapper: Rc<dyn Fn(&[String]) -> Vec<String>> =
            Rc::new(move |string_args: &[String]| {
                // Convert &[String] args to Lisp values and build call expression
                let mut call_values: Vec<Value> = Vec::with_capacity(string_args.len() + 1);
                call_values.push(callback_value.clone());
                for arg in string_args {
                    call_values.push(Value::String(arg.clone()));
                }
                let call_list: List = call_values.into_iter().collect();
                let call_expr = Value::List(call_list);

                match rust_lisp::interpreter::eval(call_env.clone(), &call_expr) {
                    Ok(result) => {
                        // Convert Lisp result to Vec<String>, extracting raw string value
                        let s = match &result {
                            Value::String(s) => s.clone(),
                            other => format!("{}", other),
                        };
                        vec![s]
                    }
                    Err(e) => {
                        // Store error in shared buffer (avoids borrowing EditorState during dispatch)
                        error_buf
                            .borrow_mut()
                            .push(format!("Hook error: {}", e.msg));
                        vec![]
                    }
                }
            });

        let hook_id = hook::register_hook(&mut state.borrow_mut().hooks, &hook_name, wrapper);

        Ok(Value::Int(hook_id.0 as i32))
    });
}

/// Registers `dispatch-hook`: dispatches all callbacks for a named hook.
///
/// Usage: `(dispatch-hook "hook-name" arg1 arg2 ...)`
///
/// Returns results as a Lisp list of strings. If any callback errors,
/// the error is displayed as an editor message (not a crash).
fn register_dispatch_hook(
    env: Rc<RefCell<Env>>,
    state: Rc<RefCell<EditorState>>,
    hook_errors: HookErrorBuffer,
) {
    define_native_closure(&env, "dispatch-hook", move |_env, args| {
        let hook_name = extract_string_arg(&args, "dispatch-hook")?;

        // Collect remaining args as strings (extract raw string values)
        let string_args: Vec<String> = args[1..]
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => format!("{}", other),
            })
            .collect();

        // Clear any previous hook errors
        hook_errors.borrow_mut().clear();

        // Borrow state briefly to dispatch hooks, then release.
        // Callbacks may accumulate errors in the shared error buffer.
        let results = {
            let editor = state.borrow();
            hook::dispatch_hook(&editor.hooks, &hook_name, &string_args)
        };

        // After dispatch (borrow released), propagate any errors as editor messages
        let errors = hook_errors.borrow().clone();
        if !errors.is_empty() {
            let mut editor = state.borrow_mut();
            editor.message = Some(errors.join("; "));
        }

        // Flatten results: each callback returns Vec<String>, collect all into one list
        let list_values: Vec<Value> = results
            .into_iter()
            .flat_map(|callback_results| callback_results.into_iter())
            .map(Value::String)
            .collect();

        let list: List = list_values.into_iter().collect();
        Ok(Value::List(list))
    });
}

/// Registers `remove-hook`: unregisters a hook callback by its ID.
///
/// Usage: `(remove-hook "hook-name" hook-id)`
fn register_remove_hook(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "remove-hook", move |_env, args| {
        let hook_name = extract_string_arg(&args, "remove-hook")?;

        let id_value = args.get(1).ok_or_else(|| RuntimeError {
            msg: "remove-hook: expected 2 arguments (name, hook-id), got 1".to_string(),
        })?;

        let id = match id_value {
            Value::Int(n) => hook::HookId(*n as usize),
            other => {
                return Err(RuntimeError {
                    msg: format!("remove-hook: expected integer hook-id, got {}", other),
                });
            }
        };

        hook::unregister_hook(&mut state.borrow_mut().hooks, &hook_name, id);

        Ok(Value::NIL)
    });
}

/// Parses a key-spec string into a `KeyEvent`.
///
/// Supported formats:
/// - Arrow keys: `"Up"`, `"Down"`, `"Left"`, `"Right"`
/// - Special keys: `"Enter"`, `"Escape"`, `"Backspace"`, `"Tab"`, `"Home"`, `"End"`, `"PageUp"`, `"PageDown"`, `"Delete"`
/// - Character keys: `"Char:a"`, `"Char::"` (colon character)
/// - Modifier keys: `"Ctrl:q"` (Ctrl + character)
///
/// Returns `Err(RuntimeError)` for unrecognized key-spec strings.
fn parse_key_spec(spec: &str) -> Result<alfred_core::key_event::KeyEvent, RuntimeError> {
    use alfred_core::key_event::{KeyCode, KeyEvent, Modifiers};

    // Check for modifier prefix "Ctrl:"
    if let Some(rest) = spec.strip_prefix("Ctrl:") {
        let ch = rest.chars().next().ok_or_else(|| RuntimeError {
            msg: "parse_key_spec: 'Ctrl:' requires a character, got empty string".to_string(),
        })?;
        return Ok(KeyEvent::new(KeyCode::Char(ch), Modifiers::ctrl()));
    }

    // Check for "Char:" prefix
    if let Some(rest) = spec.strip_prefix("Char:") {
        let ch = rest.chars().next().ok_or_else(|| RuntimeError {
            msg: "parse_key_spec: 'Char:' requires a character, got empty string".to_string(),
        })?;
        return Ok(KeyEvent::plain(KeyCode::Char(ch)));
    }

    // Named keys (no modifier)
    let code = match spec {
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Delete" => KeyCode::Delete,
        _ => {
            return Err(RuntimeError {
                msg: format!(
                    "parse_key_spec: unrecognized key-spec \"{}\". \
                     Expected: Up, Down, Left, Right, Enter, Escape, Backspace, Tab, \
                     Home, End, PageUp, PageDown, Delete, Char:<c>, or Ctrl:<c>",
                    spec
                ),
            });
        }
    };

    Ok(KeyEvent::plain(code))
}

/// Extracts a required string argument at a specific index from the args list.
fn extract_string_arg_at(
    args: &[Value],
    index: usize,
    fn_name: &str,
    param_name: &str,
) -> Result<String, RuntimeError> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(RuntimeError {
            msg: format!(
                "{}: expected string for {}, got {}",
                fn_name, param_name, other
            ),
        }),
        None => Err(RuntimeError {
            msg: format!("{}: missing required argument '{}'", fn_name, param_name),
        }),
    }
}

/// Registers keymap primitives (`make-keymap`, `define-key`, `set-active-keymap`) into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(make-keymap "name")` -- creates a named keymap in EditorState
/// - `(define-key "keymap-name" "key-spec" "command-name")` -- binds a key to a command
/// - `(set-active-keymap "keymap-name")` -- sets the active keymap
pub fn register_keymap_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_make_keymap(env.clone(), state.clone());
    register_define_key(env.clone(), state.clone());
    register_set_active_keymap(env, state);
}

/// Registers `make-keymap`: creates a named keymap in EditorState.
///
/// Usage: `(make-keymap "name")`
fn register_make_keymap(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "make-keymap", move |_env, args| {
        let name = extract_string_arg(&args, "make-keymap")?;
        let mut editor = state.borrow_mut();
        editor.keymaps.entry(name).or_default();
        Ok(Value::NIL)
    });
}

/// Registers `define-key`: binds a key-spec to a command name in a keymap.
///
/// Usage: `(define-key "keymap-name" "key-spec" "command-name")`
fn register_define_key(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "define-key", move |_env, args| {
        let keymap_name = extract_string_arg_at(&args, 0, "define-key", "keymap-name")?;
        let key_spec_str = extract_string_arg_at(&args, 1, "define-key", "key-spec")?;
        let command_name = extract_string_arg_at(&args, 2, "define-key", "command-name")?;

        let key_event = parse_key_spec(&key_spec_str)?;

        let mut editor = state.borrow_mut();
        let keymap = editor.keymaps.get_mut(&keymap_name).ok_or_else(|| RuntimeError {
            msg: format!(
                "define-key: keymap \"{}\" does not exist. Create it first with (make-keymap \"{}\")",
                keymap_name, keymap_name
            ),
        })?;

        keymap.insert(key_event, command_name);
        Ok(Value::NIL)
    });
}

/// Registers `set-active-keymap`: sets the active keymap(s) in EditorState.
///
/// Usage: `(set-active-keymap "keymap-name")`
fn register_set_active_keymap(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-active-keymap", move |_env, args| {
        let keymap_name = extract_string_arg(&args, "set-active-keymap")?;
        let mut editor = state.borrow_mut();
        editor.active_keymaps = vec![keymap_name];
        Ok(Value::NIL)
    });
}

/// Registers `buffer-insert`: inserts text at the current cursor position.
fn register_buffer_insert(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-insert", move |_env, args| {
        let text = extract_string_arg(&args, "buffer-insert")?;
        let mut editor = state.borrow_mut();
        let cursor_line = editor.cursor.line;
        let cursor_column = editor.cursor.column;
        editor.buffer = buffer::insert_at(&editor.buffer, cursor_line, cursor_column, &text);
        Ok(Value::NIL)
    });
}

/// Registers `buffer-delete`: removes one character at the cursor position.
fn register_buffer_delete(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-delete", move |_env, _args| {
        let mut editor = state.borrow_mut();
        let cursor_line = editor.cursor.line;
        let cursor_column = editor.cursor.column;
        editor.buffer = buffer::delete_at(&editor.buffer, cursor_line, cursor_column);
        Ok(Value::NIL)
    });
}

/// Registers `buffer-content`: returns the entire buffer text as a string.
fn register_buffer_content(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-content", move |_env, _args| {
        let editor = state.borrow();
        let text = buffer::content(&editor.buffer);
        Ok(Value::String(text))
    });
}

/// Registers `cursor-position`: returns the cursor's (line column) as a list.
fn register_cursor_position(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "cursor-position", move |_env, _args| {
        let editor = state.borrow();
        let line = editor.cursor.line as i32;
        let column = editor.cursor.column as i32;
        let list: List = vec![Value::Int(line), Value::Int(column)]
            .into_iter()
            .collect();
        Ok(Value::List(list))
    });
}

/// Extracts a direction string from either a symbol (`:down`) or a string (`"down"`).
fn extract_direction(value: &Value) -> Result<String, RuntimeError> {
    match value {
        Value::Symbol(Symbol(s)) => {
            let direction = s.strip_prefix(':').unwrap_or(s);
            Ok(direction.to_string())
        }
        Value::String(s) => Ok(s.clone()),
        other => Err(RuntimeError {
            msg: format!(
                "cursor-move: expected direction as symbol or string, got {}",
                other
            ),
        }),
    }
}

/// Applies a single cursor movement in the given direction.
fn apply_cursor_move(
    cursor_pos: cursor::Cursor,
    buf: &buffer::Buffer,
    direction: &str,
) -> Result<cursor::Cursor, RuntimeError> {
    match direction {
        "up" => Ok(cursor::move_up(cursor_pos, buf)),
        "down" => Ok(cursor::move_down(cursor_pos, buf)),
        "left" => Ok(cursor::move_left(cursor_pos, buf)),
        "right" => Ok(cursor::move_right(cursor_pos, buf)),
        unknown => Err(RuntimeError {
            msg: format!(
                "cursor-move: unknown direction \"{}\". Expected up, down, left, or right",
                unknown
            ),
        }),
    }
}

/// Registers `cursor-move`: moves the cursor by direction and optional count.
///
/// Usage: `(cursor-move :direction)` or `(cursor-move :direction count)`
/// Direction can be a symbol (`:up`, `:down`, `:left`, `:right`) or a
/// string (`"up"`, `"down"`, `"left"`, `"right"`). Count defaults to 1.
fn register_cursor_move(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "cursor-move", move |_env, args| {
        let direction_value = args.first().ok_or_else(|| RuntimeError {
            msg: "cursor-move: expected at least 1 argument (direction), got 0".to_string(),
        })?;

        let direction = extract_direction(direction_value)?;

        let count = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("cursor-move: expected integer count, got {}", other),
                });
            }
            None => 1,
        };

        let mut editor = state.borrow_mut();
        let mut current_cursor = editor.cursor;
        for _ in 0..count {
            current_cursor = apply_cursor_move(current_cursor, &editor.buffer, &direction)?;
        }
        editor.cursor = current_cursor;
        editor.viewport = viewport::adjust(editor.viewport, &editor.cursor);

        Ok(Value::NIL)
    });
}

/// Registers `message`: sets the editor message line.
///
/// Usage: `(message "text")` -- sets `state.message = Some("text")`.
fn register_message(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "message", move |_env, args| {
        let text = extract_string_arg(&args, "message")?;
        let mut editor = state.borrow_mut();
        editor.message = Some(text);
        Ok(Value::NIL)
    });
}

/// Registers `buffer-filename`: returns the current buffer's filename or empty string.
///
/// Usage: `(buffer-filename)` -- returns the filename as a string, or `""` if unnamed.
fn register_buffer_filename(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-filename", move |_env, _args| {
        let editor = state.borrow();
        let filename = editor.buffer.filename().unwrap_or("").to_string();
        Ok(Value::String(filename))
    });
}

/// Registers `buffer-modified?`: returns whether the buffer has been modified.
///
/// Usage: `(buffer-modified?)` -- returns `T` if modified, `F` if not.
fn register_buffer_modified(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-modified?", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::from(editor.buffer.is_modified()))
    });
}

/// Registers `save-buffer`: saves the current buffer to disk.
///
/// Usage:
/// - `(save-buffer)` -- saves to the buffer's original file path (error if no path)
/// - `(save-buffer "path")` -- saves to the specified path
///
/// Returns NIL on success. Resets the buffer's modified flag to false.
fn register_save_buffer(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "save-buffer", move |_env, args| {
        let save_path = match args.first() {
            Some(Value::String(path_str)) => std::path::PathBuf::from(path_str),
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("save-buffer: expected string path argument, got {}", other),
                });
            }
            None => {
                // No argument: use buffer's file_path
                let editor = state.borrow();
                match editor.buffer.file_path() {
                    Some(path) => path.to_path_buf(),
                    None => {
                        return Err(RuntimeError {
                            msg: "save-buffer: buffer has no file path; provide a path argument"
                                .to_string(),
                        });
                    }
                }
            }
        };

        let mut editor = state.borrow_mut();
        match buffer::save_to_file(&editor.buffer, &save_path) {
            Ok(saved_buffer) => {
                editor.buffer = saved_buffer;
                Ok(Value::NIL)
            }
            Err(e) => Err(RuntimeError {
                msg: format!("save-buffer: {}", e),
            }),
        }
    });
}

/// Registers `current-mode`: returns the current editor mode name as a string.
///
/// Usage: `(current-mode)` -- returns `"normal"` (or other mode name).
fn register_current_mode(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "current-mode", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::String(editor.mode.clone()))
    });
}

/// Registers `set-mode`: changes the editor mode and updates active keymaps.
///
/// Usage: `(set-mode "insert")` or `(set-mode "normal")`
///
/// Sets `state.mode` to the given string and `state.active_keymaps`
/// to `["{mode-name}-mode"]`, switching the active keymap to match.
fn register_set_mode(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-mode", move |_env, args| {
        let mode_name = extract_string_arg(&args, "set-mode")?;
        let mut editor = state.borrow_mut();
        editor.mode = mode_name.clone();
        editor.active_keymaps = vec![format!("{}-mode", mode_name)];
        Ok(Value::NIL)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use alfred_core::cursor;
    use alfred_core::editor_state;

    // -----------------------------------------------------------------------
    // Acceptance test: buffer-insert through bridge modifies buffer
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_with_text_when_buffer_insert_evaluated_then_buffer_contains_inserted_text() {
        // Given: an editor state with buffer "Hello" and cursor at column 5
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
            editor.cursor = cursor::new(0, 5);
        }

        // And: a runtime with bridge primitives registered
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: buffer-insert is evaluated
        runtime.eval("(buffer-insert \" World\")").unwrap();

        // Then: the buffer content is "Hello World"
        let editor = state.borrow();
        assert_eq!(buffer::content(&editor.buffer), "Hello World");
    }

    // -----------------------------------------------------------------------
    // Unit tests: individual primitives
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_buffer_when_buffer_insert_evaluated_then_buffer_contains_text() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(buffer-insert \"hello\")").unwrap();

        let editor = state.borrow();
        assert_eq!(buffer::content(&editor.buffer), "hello");
    }

    #[test]
    fn given_buffer_with_text_when_buffer_delete_evaluated_then_character_removed() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
            editor.cursor = cursor::new(0, 4); // cursor at 'o'
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(buffer-delete)").unwrap();

        let editor = state.borrow();
        assert_eq!(buffer::content(&editor.buffer), "Hell");
    }

    #[test]
    fn given_buffer_with_text_when_buffer_content_evaluated_then_returns_text() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Test content");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-content)").unwrap();
        assert_eq!(result.as_string(), Some("Test content".to_string()));
    }

    #[test]
    fn given_cursor_at_known_position_when_cursor_position_evaluated_then_returns_line_and_column()
    {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Line 1\nLine 2\nLine 3");
            editor.cursor = cursor::new(2, 3);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(cursor-position)").unwrap();
        // Result should be a list (2 3)
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], Value::Int(2));
                assert_eq!(items[1], Value::Int(3));
            }
            _ => panic!("cursor-position should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_runtime_with_primitives_when_buffer_insert_wrong_type_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-insert 42)");
        assert!(result.is_err());
    }

    #[test]
    fn given_runtime_with_primitives_when_buffer_insert_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-insert)");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Acceptance test: cursor-move through bridge moves cursor position
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_cursor_move_down_evaluated_then_cursor_moves_down() {
        // Given: an editor state with a multi-line buffer and cursor at (0, 0)
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Line 1\nLine 2\nLine 3");
            editor.cursor = cursor::new(0, 0);
        }

        // And: a runtime with bridge primitives registered
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: cursor-move ':down 1 is evaluated (quoted symbol for keyword)
        runtime.eval("(cursor-move ':down 1)").unwrap();

        // Then: the cursor has moved to line 1
        let editor = state.borrow();
        assert_eq!(editor.cursor.line, 1);
        assert_eq!(editor.cursor.column, 0);
    }

    // -----------------------------------------------------------------------
    // Unit tests: cursor-move directions
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_at_line_1_when_cursor_move_up_then_cursor_moves_to_line_0() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Line 1\nLine 2");
            editor.cursor = cursor::new(1, 0);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(cursor-move ':up)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.cursor.line, 0);
    }

    #[test]
    fn given_cursor_at_col_0_when_cursor_move_right_then_cursor_moves_to_col_1() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
            editor.cursor = cursor::new(0, 0);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(cursor-move ':right)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.cursor.column, 1);
    }

    #[test]
    fn given_cursor_at_col_3_when_cursor_move_left_then_cursor_moves_to_col_2() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
            editor.cursor = cursor::new(0, 3);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(cursor-move ':left)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.cursor.column, 2);
    }

    // -----------------------------------------------------------------------
    // Unit tests: cursor-move with count
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_cursor_move_down_with_count_3_then_cursor_moves_3_lines() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("L1\nL2\nL3\nL4\nL5");
            editor.cursor = cursor::new(0, 0);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(cursor-move ':down 3)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.cursor.line, 3);
    }

    // -----------------------------------------------------------------------
    // Unit tests: cursor-move with string direction
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_cursor_move_with_string_direction_then_cursor_moves() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Line 1\nLine 2");
            editor.cursor = cursor::new(0, 0);
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(cursor-move \"down\" 1)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.cursor.line, 1);
    }

    // -----------------------------------------------------------------------
    // Unit tests: cursor-move wrong argument type
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_when_cursor_move_with_wrong_direction_type_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // 42 is not a valid direction (not a symbol or string)
        let result = runtime.eval("(cursor-move 42)");
        assert!(result.is_err());
    }

    #[test]
    fn given_runtime_when_cursor_move_with_invalid_direction_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Hello");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(cursor-move ':diagonal)");
        assert!(result.is_err());
    }

    #[test]
    fn given_runtime_when_cursor_move_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(cursor-move)");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unit tests: message primitive
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_when_message_evaluated_then_editor_message_is_set() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(message \"hello world\")").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.message, Some("hello world".to_string()));
    }

    #[test]
    fn given_runtime_when_message_with_wrong_type_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(message 42)");
        assert!(result.is_err());
    }

    #[test]
    fn given_runtime_when_message_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(message)");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unit tests: current-mode primitive
    // -----------------------------------------------------------------------

    #[test]
    fn given_normal_mode_when_current_mode_evaluated_then_returns_normal() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(current-mode)").unwrap();
        assert_eq!(result.as_string(), Some("normal".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests: define-command primitive (step 03-05)
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_when_define_command_with_lambda_then_command_registered_in_editor() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());

        runtime
            .eval("(define-command \"test-cmd\" (lambda () (message \"invoked\")))")
            .unwrap();

        let editor = state.borrow();
        assert!(
            alfred_core::command::lookup(&editor.commands, "test-cmd").is_some(),
            "define-command should register the command in the editor's CommandRegistry"
        );
    }

    #[test]
    fn given_runtime_when_define_command_with_wrong_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());

        // No args
        let result = runtime.eval("(define-command)");
        assert!(result.is_err(), "define-command with no args should fail");

        // First arg not a string
        let result = runtime.eval("(define-command 42 (lambda () #t))");
        assert!(
            result.is_err(),
            "define-command with non-string name should fail"
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test: add-hook + dispatch-hook round-trip through Lisp bridge
    // (step 04-02)
    // -----------------------------------------------------------------------

    #[test]
    fn given_lisp_lambda_when_add_hook_and_dispatch_hook_then_callback_called_with_args_and_results_returned(
    ) {
        // Given: an editor state and a runtime with hook primitives registered
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        // And: a Lisp lambda registered as a hook via add-hook
        runtime
            .eval("(add-hook \"on-save\" (lambda (arg) (+ \"saved:\" arg)))")
            .unwrap();

        // When: dispatch-hook is called with arguments
        let result = runtime
            .eval("(dispatch-hook \"on-save\" \"myfile.txt\")")
            .unwrap();

        // Then: the result is a list containing the callback's return values
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(
                    items.len(),
                    1,
                    "one callback registered, one result expected"
                );
                // The callback concatenates "saved:" + arg
                assert_eq!(items[0], Value::String("saved:myfile.txt".to_string()));
            }
            _ => panic!("dispatch-hook should return a list, got {:?}", inner),
        }
    }

    // -----------------------------------------------------------------------
    // Unit tests: hook primitives (step 04-02)
    // Test Budget: 6 behaviors x 2 = 12 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_add_hook_when_evaluated_then_returns_hook_id_as_integer() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        let result = runtime
            .eval("(add-hook \"test-hook\" (lambda () \"ok\"))")
            .unwrap();

        assert!(
            result.as_integer().is_some(),
            "add-hook should return a HookId as integer, got: {}",
            result
        );
    }

    #[test]
    fn given_dispatch_hook_on_unknown_hook_when_evaluated_then_returns_empty_list() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        let result = runtime.eval("(dispatch-hook \"nonexistent\")").unwrap();

        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert!(
                    items.is_empty(),
                    "dispatch-hook on unknown hook should return empty list"
                );
            }
            other if format!("{}", other) == "NIL" => {} // NIL is acceptable for empty
            _ => panic!(
                "dispatch-hook on unknown hook should return empty list or NIL, got {:?}",
                inner
            ),
        }
    }

    #[test]
    fn given_multiple_hooks_when_dispatched_then_all_callback_results_returned() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        runtime
            .eval("(add-hook \"multi\" (lambda () \"first\"))")
            .unwrap();
        runtime
            .eval("(add-hook \"multi\" (lambda () \"second\"))")
            .unwrap();

        let result = runtime.eval("(dispatch-hook \"multi\")").unwrap();

        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(
                    items.len(),
                    2,
                    "two callbacks registered, two results expected"
                );
                assert_eq!(items[0], Value::String("first".to_string()));
                assert_eq!(items[1], Value::String("second".to_string()));
            }
            _ => panic!("dispatch-hook should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_hook_callback_that_errors_when_dispatched_then_error_shown_as_message_not_crash() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        // Register a callback that will error (calling undefined function)
        runtime
            .eval("(add-hook \"err-hook\" (lambda () (undefined-fn)))")
            .unwrap();

        // dispatch-hook should NOT crash -- it should succeed and set a message
        let result = runtime.eval("(dispatch-hook \"err-hook\")");
        assert!(
            result.is_ok(),
            "dispatch-hook should not crash on callback error"
        );

        // The error should be captured as a message in editor state
        let editor = state.borrow();
        assert!(
            editor.message.is_some(),
            "hook error should be displayed as a message"
        );
    }

    #[test]
    fn given_remove_hook_when_evaluated_then_callback_no_longer_dispatched() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        // Register and capture the hook-id
        runtime
            .eval("(define hook-id (add-hook \"removable\" (lambda () \"should-not-appear\")))")
            .unwrap();

        // Remove the hook
        runtime.eval("(remove-hook \"removable\" hook-id)").unwrap();

        // Dispatch should return empty
        let result = runtime.eval("(dispatch-hook \"removable\")").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert!(
                    items.is_empty(),
                    "removed hook should no longer produce results"
                );
            }
            other if format!("{}", other) == "NIL" => {} // NIL is acceptable for empty
            _ => panic!(
                "dispatch after remove-hook should return empty list or NIL, got {:?}",
                inner
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Acceptance test: buffer-filename and buffer-modified? status bar primitives
    // (step 05-02)
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_with_filename_when_buffer_filename_evaluated_then_returns_filename() {
        // Given: an editor state with a buffer loaded from a file (simulated via filename)
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("some content");
            // We need a buffer with a filename -- use from_file or set manually
            // Since from_string doesn't set filename, we'll create one from a temp file
        }

        // Use a temp file to get a buffer with a filename
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_file.txt");
        std::fs::write(&temp_file, "hello").unwrap();
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_file(&temp_file).unwrap();
        }

        // And: a runtime with bridge primitives registered
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: buffer-filename is evaluated
        let result = runtime.eval("(buffer-filename)").unwrap();

        // Then: returns the filename as a string
        assert_eq!(result.as_string(), Some("test_file.txt".to_string()));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    // -----------------------------------------------------------------------
    // Unit tests: buffer-filename and buffer-modified? primitives (step 05-02)
    // Test Budget: 4 behaviors x 2 = 8 max unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_without_filename_when_buffer_filename_evaluated_then_returns_empty_string() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-filename)").unwrap();
        assert_eq!(result.as_string(), Some("".to_string()));
    }

    #[test]
    fn given_unmodified_buffer_when_buffer_modified_evaluated_then_returns_false() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-modified?)").unwrap();
        assert_eq!(*result.inner(), Value::False);
    }

    #[test]
    fn given_modified_buffer_when_buffer_modified_evaluated_then_returns_true() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // Modify the buffer via the bridge primitive
        runtime.eval("(buffer-insert \"text\")").unwrap();

        let result = runtime.eval("(buffer-modified?)").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_add_hook_with_wrong_args_when_evaluated_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_hook_primitives(&runtime, state.clone());

        // No args
        let result = runtime.eval("(add-hook)");
        assert!(result.is_err(), "add-hook with no args should fail");

        // First arg not a string
        let result = runtime.eval("(add-hook 42 (lambda () #t))");
        assert!(result.is_err(), "add-hook with non-string name should fail");

        // Second arg not callable
        let result = runtime.eval("(add-hook \"test\" 42)");
        assert!(
            result.is_err(),
            "add-hook with non-callable callback should fail"
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests: keymap primitives (step 06-01)
    // Test Budget: 6 behaviors x 2 = 12 max unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_with_keymap_primitives_when_make_keymap_then_keymap_exists_in_state() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        runtime.eval("(make-keymap \"insert\")").unwrap();

        let editor = state.borrow();
        assert!(
            editor.keymaps.contains_key("insert"),
            "make-keymap should create an entry in EditorState.keymaps"
        );
    }

    #[test]
    fn given_runtime_when_set_active_keymap_then_active_keymaps_updated() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        runtime.eval("(make-keymap \"visual\")").unwrap();
        runtime.eval("(set-active-keymap \"visual\")").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.active_keymaps, vec!["visual".to_string()]);
    }

    #[test]
    fn given_various_key_specs_when_define_key_then_all_parsed_correctly() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        runtime.eval("(make-keymap \"test\")").unwrap();

        let specs_and_expected: Vec<(&str, alfred_core::key_event::KeyEvent)> = vec![
            (
                "Up",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Up),
            ),
            (
                "Down",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Down),
            ),
            (
                "Left",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Left),
            ),
            (
                "Right",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Right),
            ),
            (
                "Enter",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Enter),
            ),
            (
                "Escape",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Escape),
            ),
            (
                "Backspace",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Backspace),
            ),
            (
                "Char:a",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Char('a')),
            ),
            (
                "Char::",
                alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Char(':')),
            ),
            (
                "Ctrl:q",
                alfred_core::key_event::KeyEvent::new(
                    alfred_core::key_event::KeyCode::Char('q'),
                    alfred_core::key_event::Modifiers::ctrl(),
                ),
            ),
        ];

        for (i, (spec, expected)) in specs_and_expected.iter().enumerate() {
            let cmd = format!("cmd-{}", i);
            let expr = format!("(define-key \"test\" \"{}\" \"{}\")", spec, cmd);
            runtime.eval(&expr).unwrap();

            let editor = state.borrow();
            let keymap = editor.keymaps.get("test").unwrap();
            assert_eq!(
                keymap.get(expected),
                Some(&cmd),
                "key-spec '{}' should parse correctly",
                spec
            );
        }
    }

    #[test]
    fn given_invalid_key_spec_when_define_key_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        runtime.eval("(make-keymap \"test\")").unwrap();

        let result = runtime.eval("(define-key \"test\" \"InvalidKey\" \"cmd\")");
        assert!(result.is_err(), "invalid key-spec should return error");
    }

    #[test]
    fn given_define_key_on_nonexistent_keymap_when_evaluated_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        let result = runtime.eval("(define-key \"nonexistent\" \"Up\" \"cmd\")");
        assert!(
            result.is_err(),
            "define-key on nonexistent keymap should return error"
        );
    }

    #[test]
    fn given_keymap_primitives_with_wrong_args_when_evaluated_then_returns_errors() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        // make-keymap: no args
        assert!(runtime.eval("(make-keymap)").is_err());
        // make-keymap: wrong type
        assert!(runtime.eval("(make-keymap 42)").is_err());
        // define-key: missing args
        assert!(runtime.eval("(define-key \"km\")").is_err());
        // set-active-keymap: no args
        assert!(runtime.eval("(set-active-keymap)").is_err());
    }

    // -----------------------------------------------------------------------
    // Acceptance test: keymap primitives round-trip through Lisp bridge
    // (step 06-01)
    // -----------------------------------------------------------------------

    #[test]
    fn given_keymap_primitives_when_make_keymap_define_key_set_active_then_keymap_stored_and_active(
    ) {
        // Given: an editor state and a runtime with keymap primitives registered
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        // When: a keymap is created, keys are bound, and it is activated
        runtime.eval("(make-keymap \"normal\")").unwrap();
        runtime
            .eval("(define-key \"normal\" \"Ctrl:q\" \"quit\")")
            .unwrap();
        runtime
            .eval("(define-key \"normal\" \"Up\" \"cursor-up\")")
            .unwrap();
        runtime.eval("(set-active-keymap \"normal\")").unwrap();

        // Then: the keymap exists with the correct bindings
        let editor = state.borrow();
        let keymap = editor
            .keymaps
            .get("normal")
            .expect("keymap 'normal' should exist");
        let ctrl_q = alfred_core::key_event::KeyEvent::new(
            alfred_core::key_event::KeyCode::Char('q'),
            alfred_core::key_event::Modifiers::ctrl(),
        );
        assert_eq!(
            keymap.get(&ctrl_q),
            Some(&"quit".to_string()),
            "Ctrl:q should be bound to 'quit'"
        );
        let up = alfred_core::key_event::KeyEvent::plain(alfred_core::key_event::KeyCode::Up);
        assert_eq!(
            keymap.get(&up),
            Some(&"cursor-up".to_string()),
            "Up should be bound to 'cursor-up'"
        );

        // And: the active keymaps include "normal"
        assert_eq!(editor.active_keymaps, vec!["normal".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (07-01): set-mode changes mode and active keymaps
    // -----------------------------------------------------------------------

    #[test]
    fn given_normal_mode_when_set_mode_insert_then_current_mode_returns_insert_and_active_keymaps_updated(
    ) {
        // Given: an editor in normal mode
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // Verify initial mode is normal
        let initial_mode = runtime.eval("(current-mode)").unwrap();
        assert_eq!(initial_mode.as_string(), Some("normal".to_string()));

        // When: set-mode to "insert"
        runtime.eval("(set-mode \"insert\")").unwrap();

        // Then: current-mode returns "insert"
        let result = runtime.eval("(current-mode)").unwrap();
        assert_eq!(result.as_string(), Some("insert".to_string()));

        // And: active keymaps is set to ["insert-mode"]
        let editor = state.borrow();
        assert_eq!(editor.active_keymaps, vec!["insert-mode".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Unit tests (07-01): set-mode primitive
    // Test Budget: 3 behaviors x 2 = 6 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_insert_mode_when_set_mode_normal_then_mode_is_normal_and_active_keymaps_updated() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // Switch to insert first
        runtime.eval("(set-mode \"insert\")").unwrap();

        // When: set-mode back to normal
        runtime.eval("(set-mode \"normal\")").unwrap();

        // Then: mode is normal
        let result = runtime.eval("(current-mode)").unwrap();
        assert_eq!(result.as_string(), Some("normal".to_string()));

        // And: active keymaps updated
        let editor = state.borrow();
        assert_eq!(editor.active_keymaps, vec!["normal-mode".to_string()]);
    }

    #[test]
    fn given_runtime_when_set_mode_with_wrong_type_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-mode 42)");
        assert!(result.is_err(), "set-mode with non-string should fail");
    }

    #[test]
    fn given_runtime_when_set_mode_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-mode)");
        assert!(result.is_err(), "set-mode with no args should fail");
    }

    // -----------------------------------------------------------------------
    // Acceptance test (07-02): vim-keybindings plugin creates normal/insert
    // mode keymaps with hjkl navigation, mode switching, and editing commands
    // -----------------------------------------------------------------------

    #[test]
    fn given_vim_keybindings_plugin_when_loaded_then_normal_mode_keymaps_with_hjkl_and_editing_commands_active(
    ) {
        use alfred_core::key_event::{KeyCode, KeyEvent};

        // Given: an editor state with builtin commands and a runtime with all primitives
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer =
                alfred_core::buffer::Buffer::from_string("Hello World\nSecond line\nThird line");
            editor.cursor = cursor::new(0, 0);
        }
        editor_state::register_builtin_commands(&mut state.borrow_mut());

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());

        // When: the vim-keybindings plugin is loaded
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

        // Then: normal-mode keymap exists with hjkl bindings
        let editor = state.borrow();
        let normal_keymap = editor
            .keymaps
            .get("normal-mode")
            .expect("normal-mode keymap should exist");

        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('h'))),
            Some(&"cursor-left".to_string()),
            "h should be bound to cursor-left"
        );
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('j'))),
            Some(&"cursor-down".to_string()),
            "j should be bound to cursor-down"
        );
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('k'))),
            Some(&"cursor-up".to_string()),
            "k should be bound to cursor-up"
        );
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('l'))),
            Some(&"cursor-right".to_string()),
            "l should be bound to cursor-right"
        );

        // And: i is bound to enter-insert-mode, x to delete-char-at-cursor, d to delete-line
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('i'))),
            Some(&"enter-insert-mode".to_string()),
            "i should be bound to enter-insert-mode"
        );
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('x'))),
            Some(&"delete-char-at-cursor".to_string()),
            "x should be bound to delete-char-at-cursor"
        );
        assert_eq!(
            normal_keymap.get(&KeyEvent::plain(KeyCode::Char('d'))),
            Some(&"delete-line".to_string()),
            "d should be bound to delete-line"
        );

        // And: insert-mode keymap exists with Escape bound to enter-normal-mode
        let insert_keymap = editor
            .keymaps
            .get("insert-mode")
            .expect("insert-mode keymap should exist");
        assert_eq!(
            insert_keymap.get(&KeyEvent::plain(KeyCode::Escape)),
            Some(&"enter-normal-mode".to_string()),
            "Escape should be bound to enter-normal-mode in insert mode"
        );

        // And: mode is "normal" and active keymap is "normal-mode"
        assert_eq!(editor.mode, "normal");
        assert_eq!(editor.active_keymaps, vec!["normal-mode".to_string()]);

        // And: enter-insert-mode and enter-normal-mode commands are registered
        assert!(
            alfred_core::command::lookup(&editor.commands, "enter-insert-mode").is_some(),
            "enter-insert-mode command should be registered"
        );
        assert!(
            alfred_core::command::lookup(&editor.commands, "enter-normal-mode").is_some(),
            "enter-normal-mode command should be registered"
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (08-01): save-buffer writes file and resets modified
    // -----------------------------------------------------------------------

    #[test]
    fn given_modified_buffer_with_filename_when_save_buffer_evaluated_then_file_written_and_modified_reset(
    ) {
        // Given: an editor state with a buffer loaded from a temp file, then modified
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("save_bridge_test.txt");
        std::fs::write(&file_path, "Original").unwrap();

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_file(&file_path).unwrap();
            // Modify the buffer by inserting text
            editor.buffer = alfred_core::buffer::insert_at(&editor.buffer, 0, 8, " modified");
        }

        // Precondition: buffer is modified
        assert!(state.borrow().buffer.is_modified());

        // And: a runtime with bridge primitives registered
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: save-buffer is evaluated (no args -> saves to buffer's filename)
        runtime.eval("(save-buffer)").unwrap();

        // Then: the file on disk contains the updated content
        let on_disk = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, "Original modified");

        // And: the buffer's modified flag is reset
        assert!(!state.borrow().buffer.is_modified());
    }

    // -----------------------------------------------------------------------
    // Unit tests (08-01): save-buffer primitive
    // Test Budget: 3 behaviors x 2 = 6 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_without_filename_when_save_buffer_no_args_then_returns_error() {
        // Given: a buffer with no filename (from_string)
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("content");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: save-buffer is called with no arguments
        let result = runtime.eval("(save-buffer)");

        // Then: it returns an error (no filename to save to)
        assert!(
            result.is_err(),
            "save-buffer with no filename and no args should fail"
        );
    }

    #[test]
    fn given_buffer_when_save_buffer_with_path_arg_then_saves_to_specified_path() {
        // Given: a buffer with some content (no filename)
        let temp_dir = tempfile::TempDir::new().unwrap();
        let save_path = temp_dir.path().join("explicit_save.txt");

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("save me here");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: save-buffer is called with an explicit path
        let expr = format!("(save-buffer \"{}\")", save_path.display());
        runtime.eval(&expr).unwrap();

        // Then: the file is written at the specified path
        let on_disk = std::fs::read_to_string(&save_path).unwrap();
        assert_eq!(on_disk, "save me here");

        // And: modified flag is reset
        assert!(!state.borrow().buffer.is_modified());
    }
}
