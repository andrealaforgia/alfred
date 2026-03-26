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
use alfred_core::panel;
use alfred_core::theme;
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
/// - `(set-cursor-shape "mode" "shape")` -- set cursor shape for a mode
/// - `(get-cursor-shape "mode")` -- get cursor shape name for a mode
/// - `(set-tab-width n)` -- set the number of spaces per Tab (must be >= 1)
/// - `(get-tab-width)` -- return the current tab width as an integer
/// - `(buffer-set-content text)` -- replace entire buffer with text (virtual display, not user content)
/// - `(quit)` -- quit the editor (sets running to false)
pub fn register_core_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_buffer_insert(env.clone(), state.clone());
    register_buffer_delete(env.clone(), state.clone());
    register_buffer_content(env.clone(), state.clone());
    register_buffer_set_content(env.clone(), state.clone());
    register_cursor_position(env.clone(), state.clone());
    register_cursor_move(env.clone(), state.clone());
    register_message(env.clone(), state.clone());
    register_current_mode(env.clone(), state.clone());
    register_buffer_filename(env.clone(), state.clone());
    register_buffer_modified(env.clone(), state.clone());
    register_save_buffer(env.clone(), state.clone());
    register_set_cursor_shape(env.clone(), state.clone());
    register_get_cursor_shape(env.clone(), state.clone());
    register_set_tab_width(env.clone(), state.clone());
    register_get_tab_width(env.clone(), state.clone());
    register_quit(env.clone(), state.clone());
    register_set_mode(env, state);
}

/// Registers rendering-control primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(viewport-top-line)` -- returns the first visible line number (0-indexed)
/// - `(viewport-height)` -- returns the number of visible lines
pub fn register_rendering_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_viewport_top_line(env.clone(), state.clone());
    register_viewport_height(env, state);
}

/// Registers `viewport-top-line`: returns the first visible line number (0-indexed).
///
/// Usage: `(viewport-top-line)`
fn register_viewport_top_line(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "viewport-top-line", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::Int(
            alfred_core::facade::viewport_top_line(&editor) as i32
        ))
    });
}

/// Registers `viewport-height`: returns the number of visible lines.
///
/// Usage: `(viewport-height)`
fn register_viewport_height(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "viewport-height", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::Int(
            alfred_core::facade::viewport_height(&editor) as i32
        ))
    });
}

/// Registers the `define-command` Lisp primitive.
///
/// Usage: `(define-command "name" callback-fn)`
///
/// Registers a Lisp function as a named command in the editor's CommandRegistry.
/// When the command is later executed, the callback is invoked via the Lisp runtime.
pub fn register_define_command(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();
    let _lisp_env = runtime.env(); // no longer used; current_env captures the right scope

    define_native_closure(&env, "define-command", move |current_env, args| {
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
        // Use the current environment (at define-command call time) so the callback
        // can access variables defined earlier in the same plugin file.
        let call_env = current_env;

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
        let wrapper: Rc<alfred_core::hook::HookCallbackFn> =
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

        // Clone callback Rc pointers out of the registry, then release the borrow.
        // This allows callbacks to mutate state (e.g., `(message ...)`) without
        // hitting a RefCell borrow conflict during dispatch.
        let callbacks: Vec<Rc<hook::HookCallbackFn>> = {
            let editor = state.borrow();
            hook::get_callbacks(&editor.hooks, &hook_name)
        };

        // Execute callbacks outside the borrow
        let results: Vec<Vec<String>> = callbacks.iter().map(|cb| cb(&string_args)).collect();

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
        "DoubleQuote" => KeyCode::Char('"'),
        "SingleQuote" => KeyCode::Char('\''),
        _ => {
            return Err(RuntimeError {
                msg: format!(
                    "parse_key_spec: unrecognized key-spec \"{}\". \
                     Expected: Up, Down, Left, Right, Enter, Escape, Backspace, Tab, \
                     Home, End, PageUp, PageDown, Delete, DoubleQuote, SingleQuote, Char:<c>, or Ctrl:<c>",
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
        let text = alfred_core::facade::buffer_content(&editor);
        Ok(Value::String(text))
    });
}

/// Registers `buffer-set-content`: replaces the entire buffer with the given text.
///
/// Usage: `(buffer-set-content "new content here")`
///
/// Intended for virtual displays (e.g., the folder browser), not user edits.
/// The buffer's modified flag is set to false after replacement, since this
/// is programmatic content (not user changes that need saving).
/// Cursor is reset to (0,0) and viewport top_line to 0.
fn register_buffer_set_content(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-set-content", move |_env, args| {
        let text = extract_string_arg(&args, "buffer-set-content")?;
        let mut editor = state.borrow_mut();
        editor.buffer = buffer::Buffer::from_string(&text);
        editor.cursor = cursor::new(0, 0);
        editor.viewport.top_line = 0;
        Ok(Value::NIL)
    });
}

/// Registers `quit`: sets `running` to false, causing the event loop to exit.
///
/// Usage: `(quit)`
fn register_quit(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "quit", move |_env, _args| {
        state.borrow_mut().running = false;
        Ok(Value::NIL)
    });
}

/// Registers `cursor-position`: returns the cursor's (line column) as a list.
fn register_cursor_position(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "cursor-position", move |_env, _args| {
        let editor = state.borrow();
        let (line, column) = alfred_core::facade::cursor_position(&editor);
        let list: List = vec![Value::Int(line as i32), Value::Int(column as i32)]
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
        let filename = alfred_core::facade::buffer_filename(&editor)
            .unwrap_or("")
            .to_string();
        Ok(Value::String(filename))
    });
}

/// Registers `buffer-modified?`: returns whether the buffer has been modified.
///
/// Usage: `(buffer-modified?)` -- returns `T` if modified, `F` if not.
fn register_buffer_modified(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-modified?", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::from(alfred_core::facade::buffer_is_modified(
            &editor,
        )))
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
        Ok(Value::String(
            alfred_core::facade::current_mode(&editor).to_string(),
        ))
    });
}

/// Registers `set-cursor-shape`: sets the cursor shape for a given mode.
///
/// Usage: `(set-cursor-shape "mode-name" "shape-name")`
///
/// Valid shape names: "default", "block", "steady-block", "blinking-block",
/// "bar", "steady-bar", "blinking-bar", "underline", "steady-underline",
/// "blinking-underline".
///
/// Returns NIL on success. Returns error for invalid shape names.
fn register_set_cursor_shape(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-cursor-shape", move |_env, args| {
        let mode_name = extract_string_arg_at(&args, 0, "set-cursor-shape", "mode-name")?;
        let shape_name = extract_string_arg_at(&args, 1, "set-cursor-shape", "shape-name")?;

        if !alfred_core::editor_state::is_valid_cursor_shape(&shape_name) {
            return Err(RuntimeError {
                msg: format!(
                    "set-cursor-shape: invalid shape \"{}\". Valid shapes: {}",
                    shape_name,
                    alfred_core::editor_state::VALID_CURSOR_SHAPES.join(", ")
                ),
            });
        }

        state
            .borrow_mut()
            .cursor_shapes
            .insert(mode_name, shape_name);

        Ok(Value::NIL)
    });
}

/// Registers `get-cursor-shape`: returns the cursor shape name for a given mode.
///
/// Usage: `(get-cursor-shape "mode-name")`
///
/// Returns the shape name as a string, or NIL if no shape is configured for the mode.
fn register_get_cursor_shape(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "get-cursor-shape", move |_env, args| {
        let mode_name = extract_string_arg(&args, "get-cursor-shape")?;

        let editor = state.borrow();
        match alfred_core::facade::cursor_shape(&editor, &mode_name) {
            Some(shape) => Ok(Value::String(shape.to_string())),
            None => Ok(Value::NIL),
        }
    });
}

/// Registers `set-tab-width`: configures the number of spaces inserted per Tab key press.
///
/// Usage: `(set-tab-width 2)` or `(set-tab-width 4)`
///
/// The value must be a positive integer (>= 1). Returns NIL on success.
fn register_set_tab_width(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-tab-width", move |_env, args| {
        let value = args.first().ok_or_else(|| RuntimeError {
            msg: "set-tab-width: expected 1 argument, got 0".to_string(),
        })?;
        match value {
            Value::Int(n) => {
                if *n < 1 {
                    return Err(RuntimeError {
                        msg: format!("set-tab-width: value must be >= 1, got {}", n),
                    });
                }
                state.borrow_mut().tab_width = *n as usize;
                Ok(Value::NIL)
            }
            other => Err(RuntimeError {
                msg: format!("set-tab-width: expected integer argument, got {}", other),
            }),
        }
    });
}

/// Registers `get-tab-width`: returns the current tab width as an integer.
///
/// Usage: `(get-tab-width)`
///
/// Returns the current tab width (number of spaces per Tab).
fn register_get_tab_width(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "get-tab-width", move |_env, _args| {
        let editor = state.borrow();
        Ok(Value::Int(editor.tab_width as i32))
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

/// Registers all theme primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(set-theme-color "key" "color-value")` -- parses color, stores in active theme
/// - `(get-theme-color "key")` -- returns color value as string, or nil if not set
/// - `(define-theme "name" "key1" "color1" "key2" "color2" ...)` -- stores a named theme
/// - `(load-theme "name")` -- activates a named theme by copying its colors into the active theme
pub fn register_theme_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_set_theme_color(env.clone(), state.clone());
    register_get_theme_color(env.clone(), state.clone());
    register_define_theme(env.clone(), state.clone());
    register_load_theme(env, state);
}

/// Formats a ThemeColor back into a string representation.
///
/// RGB colors become "#rrggbb" hex strings. Named colors become their
/// lowercase name (e.g., "red", "dark-gray").
fn format_theme_color(color: &theme::ThemeColor) -> String {
    match color {
        theme::ThemeColor::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        theme::ThemeColor::Named(named) => match named {
            theme::NamedColor::Black => "black".to_string(),
            theme::NamedColor::Red => "red".to_string(),
            theme::NamedColor::Green => "green".to_string(),
            theme::NamedColor::Yellow => "yellow".to_string(),
            theme::NamedColor::Blue => "blue".to_string(),
            theme::NamedColor::Magenta => "magenta".to_string(),
            theme::NamedColor::Cyan => "cyan".to_string(),
            theme::NamedColor::White => "white".to_string(),
            theme::NamedColor::DarkGray => "dark-gray".to_string(),
            theme::NamedColor::LightRed => "light-red".to_string(),
            theme::NamedColor::LightGreen => "light-green".to_string(),
            theme::NamedColor::LightYellow => "light-yellow".to_string(),
            theme::NamedColor::LightBlue => "light-blue".to_string(),
            theme::NamedColor::LightMagenta => "light-magenta".to_string(),
            theme::NamedColor::LightCyan => "light-cyan".to_string(),
        },
    }
}

/// Registers `set-theme-color`: parses a color value and stores it in the active theme.
///
/// Usage: `(set-theme-color "key" "color-value")`
///
/// Returns NIL on success. Returns error for invalid color values.
fn register_set_theme_color(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-theme-color", move |_env, args| {
        let key = extract_string_arg_at(&args, 0, "set-theme-color", "key")?;
        let color_str = extract_string_arg_at(&args, 1, "set-theme-color", "color-value")?;

        match theme::parse_color(&color_str) {
            Some(color) => {
                state.borrow_mut().theme.insert(key, color);
            }
            None if color_str.trim().eq_ignore_ascii_case("default") => {
                // "default" means use terminal default -- remove from theme
                state.borrow_mut().theme.remove(&key);
            }
            None => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-theme-color: invalid color \"{}\". Expected #rrggbb, named color, or \"default\"",
                        color_str
                    ),
                });
            }
        }

        Ok(Value::NIL)
    });
}

/// Registers `get-theme-color`: reads a named color from the active theme.
///
/// Usage: `(get-theme-color "key")`
///
/// Returns the color as a string (e.g., "#3c3836" or "red"), or NIL if not set.
fn register_get_theme_color(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "get-theme-color", move |_env, args| {
        let key = extract_string_arg(&args, "get-theme-color")?;

        let editor = state.borrow();
        match editor.theme.get(&key) {
            Some(color) => Ok(Value::String(format_theme_color(color))),
            None => Ok(Value::NIL),
        }
    });
}

/// Registers `define-theme`: creates a named theme from variadic key-value pairs.
///
/// Usage: `(define-theme "name" "key1" "color1" "key2" "color2" ...)`
///
/// Stores the theme in `state.named_themes` for later activation via `load-theme`.
/// Returns error if an odd number of key-value arguments is provided or if any color is invalid.
fn register_define_theme(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "define-theme", move |_env, args| {
        let theme_name = extract_string_arg(&args, "define-theme")?;

        let pairs = &args[1..];
        if pairs.len() % 2 != 0 {
            return Err(RuntimeError {
                msg: format!(
                    "define-theme: expected even number of key-value arguments after name, got {}",
                    pairs.len()
                ),
            });
        }

        let mut new_theme = theme::new_theme();
        for chunk in pairs.chunks(2) {
            let key = match &chunk[0] {
                Value::String(s) => s.clone(),
                other => {
                    return Err(RuntimeError {
                        msg: format!("define-theme: expected string key, got {}", other),
                    });
                }
            };
            let color_str = match &chunk[1] {
                Value::String(s) => s.clone(),
                other => {
                    return Err(RuntimeError {
                        msg: format!("define-theme: expected string color value, got {}", other),
                    });
                }
            };
            let color = theme::parse_color(&color_str).ok_or_else(|| RuntimeError {
                msg: format!(
                    "define-theme: invalid color \"{}\" for key \"{}\". Expected #rrggbb or named color",
                    color_str, key
                ),
            })?;
            new_theme.insert(key, color);
        }

        state
            .borrow_mut()
            .named_themes
            .insert(theme_name, new_theme);
        Ok(Value::NIL)
    });
}

/// Registers `load-theme`: activates a previously defined named theme.
///
/// Usage: `(load-theme "name")`
///
/// Copies all color entries from the named theme into the active theme.
/// Returns error if the theme name is not found.
fn register_load_theme(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "load-theme", move |_env, args| {
        let theme_name = extract_string_arg(&args, "load-theme")?;

        let mut editor = state.borrow_mut();
        let named_theme = editor
            .named_themes
            .get(&theme_name)
            .ok_or_else(|| RuntimeError {
                msg: format!(
                    "load-theme: theme \"{}\" not found. Define it first with (define-theme ...)",
                    theme_name
                ),
            })?
            .clone();

        for (key, color) in named_theme {
            editor.theme.insert(key, color);
        }

        Ok(Value::NIL)
    });
}

/// Registers buffer and line-style primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(clear-line-styles)` -- clears all per-line style segments
/// - `(set-line-style line start end color)` -- adds a color segment for a line
/// - `(buffer-line-count)` -- returns the number of lines in the buffer
/// - `(buffer-get-line n)` -- returns the text content of line n (0-indexed)
pub fn register_buffer_style_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_clear_line_styles(env.clone(), state.clone());
    register_set_line_style(env.clone(), state.clone());
    register_set_line_background(env.clone(), state.clone());
    register_clear_line_backgrounds(env.clone(), state.clone());
    register_buffer_line_count(env.clone(), state.clone());
    register_buffer_get_line(env, state);
}

/// Registers `clear-line-styles`: clears all per-line style segments.
fn register_clear_line_styles(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "clear-line-styles", move |_env, _args| {
        let mut editor = state.borrow_mut();
        alfred_core::editor_state::clear_line_styles(&mut editor);
        Ok(Value::NIL)
    });
}

/// Registers `set-line-background`: sets a full-line background color.
///
/// Usage: `(set-line-background line fg-color bg-color)`
fn register_set_line_background(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-line-background", move |_env, args| {
        let line = match args.first() {
            Some(Value::Int(n)) => *n as usize,
            _ => {
                return Err(RuntimeError {
                    msg: "set-line-background: expected integer for line".to_string(),
                })
            }
        };
        let fg_str = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(RuntimeError {
                    msg: "set-line-background: expected fg color string".to_string(),
                })
            }
        };
        let bg_str = match args.get(2) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(RuntimeError {
                    msg: "set-line-background: expected bg color string".to_string(),
                })
            }
        };
        let fg = alfred_core::theme::parse_color(&fg_str).ok_or_else(|| RuntimeError {
            msg: format!("set-line-background: invalid fg color '{}'", fg_str),
        })?;
        let bg = alfred_core::theme::parse_color(&bg_str).ok_or_else(|| RuntimeError {
            msg: format!("set-line-background: invalid bg color '{}'", bg_str),
        })?;
        state.borrow_mut().line_backgrounds.insert(line, (fg, bg));
        Ok(Value::NIL)
    });
}

/// Registers `clear-line-backgrounds`: clears all per-line background colors.
fn register_clear_line_backgrounds(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "clear-line-backgrounds", move |_env, _args| {
        state.borrow_mut().line_backgrounds.clear();
        Ok(Value::NIL)
    });
}

/// Registers `set-line-style`: adds a style segment for a specific line.
///
/// Usage: `(set-line-style line start end color-string)`
///
/// Adds a color segment covering columns `start..end` on the given line.
/// The color string is parsed as a hex color (e.g., "#ff6b6b") or named color.
fn register_set_line_style(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-line-style", move |_env, args| {
        let line = match args.first() {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("set-line-style: expected integer for line, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-line-style: expected 4 arguments (line, start, end, color), got 0"
                        .to_string(),
                });
            }
        };
        let start_col = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("set-line-style: expected integer for start, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-line-style: expected 4 arguments (line, start, end, color), got 1"
                        .to_string(),
                });
            }
        };
        let end_col = match args.get(2) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("set-line-style: expected integer for end, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-line-style: expected 4 arguments (line, start, end, color), got 2"
                        .to_string(),
                });
            }
        };
        let color_str = match args.get(3) {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("set-line-style: expected string for color, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-line-style: expected 4 arguments (line, start, end, color), got 3"
                        .to_string(),
                });
            }
        };

        let color = theme::parse_color(&color_str).ok_or_else(|| RuntimeError {
            msg: format!("set-line-style: invalid color \"{}\"", color_str),
        })?;

        let mut editor = state.borrow_mut();
        alfred_core::editor_state::add_line_style(&mut editor, line, start_col, end_col, color);

        Ok(Value::NIL)
    });
}

/// Registers `buffer-line-count`: returns the number of lines in the buffer.
fn register_buffer_line_count(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-line-count", move |_env, _args| {
        let editor = state.borrow();
        let count = alfred_core::facade::buffer_line_count(&editor) as i32;
        Ok(Value::Int(count))
    });
}

/// Registers `buffer-get-line`: returns the text content of line n (0-indexed).
fn register_buffer_get_line(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "buffer-get-line", move |_env, args| {
        let line_num = match args.first() {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("buffer-get-line: expected integer, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "buffer-get-line: expected 1 argument, got 0".to_string(),
                });
            }
        };

        let editor = state.borrow();
        let content = alfred_core::facade::buffer_get_line_content(&editor, line_num);
        Ok(Value::String(content))
    });
}

/// Registers panel primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(define-panel name position size)` -- creates a panel at the given position
/// - `(remove-panel name)` -- removes a panel by name
/// - `(set-panel-content name text)` -- sets single-line content (top/bottom panels)
/// - `(set-panel-line name line-num text)` -- sets per-line content (left/right panels)
/// - `(set-panel-style name fg bg)` -- sets panel foreground/background colors
/// - `(set-panel-size name size)` -- resizes a panel
/// - `(viewport-top-line)` -- returns first visible line number (already registered by rendering)
/// - `(viewport-height)` -- returns visible line count (already registered by rendering)
pub fn register_panel_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_define_panel(env.clone(), state.clone());
    register_remove_panel(env.clone(), state.clone());
    register_set_panel_content(env.clone(), state.clone());
    register_set_panel_line(env.clone(), state.clone());
    register_set_panel_style(env.clone(), state.clone());
    register_set_panel_size(env.clone(), state.clone());
    register_set_panel_priority(env.clone(), state.clone());
    register_set_panel_line_style(env.clone(), state.clone());
    register_clear_panel_line_styles(env, state);
}

/// Registers `define-panel`: creates a panel at the given position.
///
/// Usage: `(define-panel "name" "position" size)`
///
/// Position must be one of "top", "bottom", "left", "right".
/// Size is the height (for top/bottom) or width (for left/right) in rows/columns.
fn register_define_panel(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "define-panel", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "define-panel", "name")?;
        let position_str = extract_string_arg_at(&args, 1, "define-panel", "position")?;
        let size = match args.get(2) {
            Some(Value::Int(n)) => *n as u16,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("define-panel: expected integer for size, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "define-panel: missing required argument 'size'".to_string(),
                });
            }
        };

        let position = parse_panel_position(&position_str).ok_or_else(|| RuntimeError {
            msg: format!(
                "define-panel: invalid position \"{}\", expected \"top\", \"bottom\", \"left\", or \"right\"",
                position_str
            ),
        })?;

        let mut editor = state.borrow_mut();
        panel::define_panel(&mut editor.panels, &name, position, size)
            .map_err(|e| RuntimeError { msg: e })?;

        Ok(Value::NIL)
    });
}

/// Registers `remove-panel`: removes a panel by name.
///
/// Usage: `(remove-panel "name")`
fn register_remove_panel(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "remove-panel", move |_env, args| {
        let name = extract_string_arg(&args, "remove-panel")?;
        let mut editor = state.borrow_mut();
        panel::remove_panel(&mut editor.panels, &name);
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-content`: sets single-line content for a panel.
///
/// Usage: `(set-panel-content "name" "text")`
fn register_set_panel_content(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-content", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "set-panel-content", "name")?;
        let text = extract_string_arg_at(&args, 1, "set-panel-content", "text")?;
        let mut editor = state.borrow_mut();
        panel::set_content(&mut editor.panels, &name, &text)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-line`: sets per-line content for a panel.
///
/// Usage: `(set-panel-line "name" line-num "text")`
fn register_set_panel_line(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-line", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "set-panel-line", "name")?;
        let line_num = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-panel-line: expected integer for line-num, got {}",
                        other
                    ),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-line: missing required argument 'line-num'".to_string(),
                });
            }
        };
        let text = extract_string_arg_at(&args, 2, "set-panel-line", "text")?;
        let mut editor = state.borrow_mut();
        panel::set_line(&mut editor.panels, &name, line_num, &text)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-style`: sets panel foreground/background colors.
///
/// Usage: `(set-panel-style "name" "fg" "bg")`
///
/// Pass "default" for either color to clear it (set to None).
fn register_set_panel_style(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-style", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "set-panel-style", "name")?;
        let fg_str = extract_string_arg_at(&args, 1, "set-panel-style", "fg")?;
        let bg_str = extract_string_arg_at(&args, 2, "set-panel-style", "bg")?;

        let fg = if fg_str == "default" {
            None
        } else {
            Some(fg_str.as_str())
        };
        let bg = if bg_str == "default" {
            None
        } else {
            Some(bg_str.as_str())
        };

        let mut editor = state.borrow_mut();
        panel::set_style(&mut editor.panels, &name, fg, bg).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-size`: resizes a panel.
///
/// Usage: `(set-panel-size "name" size)`
fn register_set_panel_size(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-size", move |_env, args| {
        let name = extract_string_arg(&args, "set-panel-size")?;
        let size = match args.get(1) {
            Some(Value::Int(n)) => *n as u16,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("set-panel-size: expected integer for size, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-size: missing required argument 'size'".to_string(),
                });
            }
        };
        let mut editor = state.borrow_mut();
        panel::set_size(&mut editor.panels, &name, size).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-priority`: sets the rendering priority of a panel.
///
/// Usage: `(set-panel-priority "name" priority)`
///
/// Lower priority = rendered more to the left for left panels.
fn register_set_panel_priority(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-priority", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "set-panel-priority", "name")?;
        let priority = match args.get(1) {
            Some(Value::Int(n)) => *n as u16,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-panel-priority: expected integer for priority, got {}",
                        other
                    ),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-priority: missing required argument 'priority'".to_string(),
                });
            }
        };
        let mut editor = state.borrow_mut();
        panel::set_panel_priority(&mut editor.panels, &name, priority)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `set-panel-line-style`: adds a color segment to a panel line.
///
/// Usage: `(set-panel-line-style "name" line start end color)`
fn register_set_panel_line_style(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "set-panel-line-style", move |_env, args| {
        let name = extract_string_arg_at(&args, 0, "set-panel-line-style", "name")?;
        let line = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-panel-line-style: expected integer for line, got {}",
                        other
                    ),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-line-style: missing required argument 'line'".to_string(),
                });
            }
        };
        let start_col = match args.get(2) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-panel-line-style: expected integer for start, got {}",
                        other
                    ),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-line-style: missing required argument 'start'".to_string(),
                });
            }
        };
        let end_col = match args.get(3) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!(
                        "set-panel-line-style: expected integer for end, got {}",
                        other
                    ),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "set-panel-line-style: missing required argument 'end'".to_string(),
                });
            }
        };
        let color_str = extract_string_arg_at(&args, 4, "set-panel-line-style", "color")?;

        let color = theme::parse_color(&color_str).ok_or_else(|| RuntimeError {
            msg: format!("set-panel-line-style: invalid color \"{}\"", color_str),
        })?;

        let mut editor = state.borrow_mut();
        panel::add_panel_line_style(&mut editor.panels, &name, line, start_col, end_col, color)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `clear-panel-line-styles`: clears all per-line styles from a panel.
///
/// Usage: `(clear-panel-line-styles "name")`
fn register_clear_panel_line_styles(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "clear-panel-line-styles", move |_env, args| {
        let name = extract_string_arg(&args, "clear-panel-line-styles")?;
        let mut editor = state.borrow_mut();
        panel::clear_panel_line_styles(&mut editor.panels, &name)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Parses a panel position string into the corresponding `PanelPosition` variant.
///
/// Returns `None` if the string does not match a known position.
fn parse_panel_position(position_str: &str) -> Option<panel::PanelPosition> {
    match position_str {
        "top" => Some(panel::PanelPosition::Top),
        "bottom" => Some(panel::PanelPosition::Bottom),
        "left" => Some(panel::PanelPosition::Left),
        "right" => Some(panel::PanelPosition::Right),
        _ => None,
    }
}

/// Registers panel focus and cursor primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(focus-panel name)` -- give keyboard focus to the named panel
/// - `(unfocus-panel)` -- return keyboard focus to the editor
/// - `(panel-cursor-line name)` -- return the cursor line of the named panel
/// - `(panel-cursor-down name)` -- move panel cursor down by 1
/// - `(panel-cursor-up name)` -- move panel cursor up by 1
/// - `(panel-entry-count name)` -- return number of lines set on the panel
/// - `(clear-panel-lines name)` -- clear all lines and reset cursor
/// - `(panel-set-cursor name line)` -- set panel cursor to specific line
pub fn register_panel_focus_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_focus_panel(env.clone(), state.clone());
    register_unfocus_panel(env.clone(), state.clone());
    register_panel_cursor_line(env.clone(), state.clone());
    register_panel_cursor_down(env.clone(), state.clone());
    register_panel_cursor_up(env.clone(), state.clone());
    register_panel_entry_count(env.clone(), state.clone());
    register_clear_panel_lines(env.clone(), state.clone());
    register_panel_set_cursor(env, state);
}

/// Registers `focus-panel`: gives keyboard focus to a named panel.
///
/// Usage: `(focus-panel "name")`
///
/// Sets `focused_panel` to the given name. Mode and keymaps are NOT changed --
/// the Lisp layer is responsible for setting mode/keymaps after calling this.
fn register_focus_panel(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "focus-panel", move |_env, args| {
        let name = extract_string_arg(&args, "focus-panel")?;
        let mut editor = state.borrow_mut();
        // Verify the panel exists
        if panel::get(&editor.panels, &name).is_none() {
            return Err(RuntimeError {
                msg: format!("focus-panel: panel '{}' not found", name),
            });
        }
        editor.focused_panel = Some(name.clone());
        Ok(Value::NIL)
    });
}

/// Registers `unfocus-panel`: returns keyboard focus to the editor.
///
/// Usage: `(unfocus-panel)`
///
/// Clears `focused_panel` only. Mode and keymaps are NOT changed --
/// the Lisp layer is responsible for restoring mode/keymaps after calling this.
fn register_unfocus_panel(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "unfocus-panel", move |_env, _args| {
        let mut editor = state.borrow_mut();
        editor.focused_panel = None;
        Ok(Value::NIL)
    });
}

/// Registers `panel-cursor-line`: returns the cursor line of a named panel.
///
/// Usage: `(panel-cursor-line "name")` -> integer
fn register_panel_cursor_line(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "panel-cursor-line", move |_env, args| {
        let name = extract_string_arg(&args, "panel-cursor-line")?;
        let editor = state.borrow();
        let line =
            panel::panel_cursor_line(&editor.panels, &name).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::Int(line as i32))
    });
}

/// Registers `panel-cursor-down`: moves panel cursor down by 1.
///
/// Usage: `(panel-cursor-down "name")`
fn register_panel_cursor_down(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "panel-cursor-down", move |_env, args| {
        let name = extract_string_arg(&args, "panel-cursor-down")?;
        let mut editor = state.borrow_mut();
        panel::panel_cursor_down(&mut editor.panels, &name).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `panel-cursor-up`: moves panel cursor up by 1.
///
/// Usage: `(panel-cursor-up "name")`
fn register_panel_cursor_up(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "panel-cursor-up", move |_env, args| {
        let name = extract_string_arg(&args, "panel-cursor-up")?;
        let mut editor = state.borrow_mut();
        panel::panel_cursor_up(&mut editor.panels, &name).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `panel-entry-count`: returns number of lines set on a panel.
///
/// Usage: `(panel-entry-count "name")` -> integer
fn register_panel_entry_count(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "panel-entry-count", move |_env, args| {
        let name = extract_string_arg(&args, "panel-entry-count")?;
        let editor = state.borrow();
        let count =
            panel::panel_entry_count(&editor.panels, &name).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::Int(count as i32))
    });
}

/// Registers `clear-panel-lines`: clears all lines from a panel and resets cursor to 0.
///
/// Usage: `(clear-panel-lines "name")`
fn register_clear_panel_lines(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "clear-panel-lines", move |_env, args| {
        let name = extract_string_arg(&args, "clear-panel-lines")?;
        let mut editor = state.borrow_mut();
        panel::clear_lines(&mut editor.panels, &name).map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers `panel-set-cursor`: sets panel cursor to a specific line.
///
/// Usage: `(panel-set-cursor "name" line)`
fn register_panel_set_cursor(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "panel-set-cursor", move |_env, args| {
        let name = extract_string_arg(&args, "panel-set-cursor")?;
        let line = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("panel-set-cursor: expected integer for line, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "panel-set-cursor: missing required argument 'line'".to_string(),
                });
            }
        };
        let mut editor = state.borrow_mut();
        panel::set_panel_cursor(&mut editor.panels, &name, line)
            .map_err(|e| RuntimeError { msg: e })?;
        Ok(Value::NIL)
    });
}

/// Registers pure string manipulation primitives into the Lisp runtime.
///
/// These are pure functions (no EditorState access) registered as `NativeFunc`
/// function pointers. After calling this, the following Lisp functions become available:
///
/// - `(str-split string delimiter)` -- split string into list
/// - `(str-join list delimiter)` -- join list elements with delimiter
/// - `(str-length string)` -- return string length as integer
/// - `(str-contains string substring)` -- test if string contains substring
/// - `(str-replace string old new)` -- replace all occurrences
/// - `(str-substring string start end)` -- extract substring (0-indexed, end exclusive)
/// - `(str-trim string)` -- trim leading and trailing whitespace
/// - `(str-upper string)` -- convert to uppercase
/// - `(str-lower string)` -- convert to lowercase
/// - `(str-starts-with string prefix)` -- test if string starts with prefix
/// - `(str-ends-with string suffix)` -- test if string ends with suffix
/// - `(str-index-of string substring)` -- find index of substring, or -1
/// - `(str string1 string2 ...)` -- concatenate strings (variadic)
/// - `(to-string value)` -- convert any value to its string representation
/// - `(parse-int string)` -- parse string as integer
pub fn register_string_primitives(runtime: &LispRuntime) {
    let env = runtime.env();

    env.borrow_mut().define(
        Symbol("str-split".to_string()),
        Value::NativeFunc(native_str_split),
    );
    env.borrow_mut().define(
        Symbol("str-join".to_string()),
        Value::NativeFunc(native_str_join),
    );
    env.borrow_mut().define(
        Symbol("str-concat".to_string()),
        Value::NativeFunc(native_str_concat),
    );
    env.borrow_mut().define(
        Symbol("str-length".to_string()),
        Value::NativeFunc(native_str_length),
    );
    env.borrow_mut().define(
        Symbol("str-contains".to_string()),
        Value::NativeFunc(native_str_contains),
    );
    env.borrow_mut().define(
        Symbol("str-replace".to_string()),
        Value::NativeFunc(native_str_replace),
    );
    env.borrow_mut().define(
        Symbol("str-substring".to_string()),
        Value::NativeFunc(native_str_substring),
    );
    env.borrow_mut().define(
        Symbol("str-trim".to_string()),
        Value::NativeFunc(native_str_trim),
    );
    env.borrow_mut().define(
        Symbol("str-upper".to_string()),
        Value::NativeFunc(native_str_upper),
    );
    env.borrow_mut().define(
        Symbol("str-lower".to_string()),
        Value::NativeFunc(native_str_lower),
    );
    env.borrow_mut().define(
        Symbol("str-starts-with".to_string()),
        Value::NativeFunc(native_str_starts_with),
    );
    env.borrow_mut().define(
        Symbol("str-ends-with".to_string()),
        Value::NativeFunc(native_str_ends_with),
    );
    env.borrow_mut().define(
        Symbol("str-index-of".to_string()),
        Value::NativeFunc(native_str_index_of),
    );
    env.borrow_mut().define(
        Symbol("str".to_string()),
        Value::NativeFunc(native_str_variadic),
    );
    env.borrow_mut().define(
        Symbol("to-string".to_string()),
        Value::NativeFunc(native_to_string),
    );
    env.borrow_mut().define(
        Symbol("parse-int".to_string()),
        Value::NativeFunc(native_parse_int),
    );

    // String constants: rust_lisp doesn't process escape sequences in string
    // literals, so "\n" produces literal backslash-n. Provide a newline constant.
    env.borrow_mut().define(
        Symbol("newline".to_string()),
        Value::String("\n".to_string()),
    );
}

// ---------------------------------------------------------------------------
// String primitive implementations (pure functions, no captured state)
// ---------------------------------------------------------------------------

/// `(str-split string delimiter)` -- splits string by delimiter, returns list of strings.
fn native_str_split(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-split")?;
    let delimiter = require_string_arg(&args, 1, "str-split")?;
    let parts: List = text
        .split(&delimiter)
        .map(|s| Value::String(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}

/// `(str-join list delimiter)` -- joins list elements with delimiter, returns string.
fn native_str_join(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = match args.first() {
        Some(Value::List(l)) => l,
        Some(other) => {
            return Err(RuntimeError {
                msg: format!("str-join: expected list as first arg, got {}", other),
            })
        }
        None => {
            return Err(RuntimeError {
                msg: "str-join: expected 2 arguments, got 0".to_string(),
            })
        }
    };
    let delimiter = require_string_arg(&args, 1, "str-join")?;
    let strings: Result<Vec<String>, RuntimeError> = list
        .into_iter()
        .map(|v| match v {
            Value::String(s) => Ok(s),
            other => Err(RuntimeError {
                msg: format!("str-join: list element is not a string: {}", other),
            }),
        })
        .collect();
    Ok(Value::String(strings?.join(&delimiter)))
}

/// `(str-concat list)` -- concatenates all strings in a list with no delimiter.
/// This is equivalent to `(str-join list "")` but avoids the empty-string literal
/// which rust_lisp cannot parse.
fn native_str_concat(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = match args.first() {
        Some(Value::List(l)) => l,
        Some(other) => {
            return Err(RuntimeError {
                msg: format!("str-concat: expected list as first arg, got {}", other),
            })
        }
        None => {
            return Err(RuntimeError {
                msg: "str-concat: expected 1 argument (list), got 0".to_string(),
            })
        }
    };
    let strings: Result<Vec<String>, RuntimeError> = list
        .into_iter()
        .map(|v| match v {
            Value::String(s) => Ok(s),
            other => Ok(format!("{}", other)),
        })
        .collect();
    Ok(Value::String(strings?.join("")))
}

/// `(str-length string)` -- returns the length of the string as an integer.
fn native_str_length(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-length")?;
    Ok(Value::Int(text.len() as i32))
}

/// `(str-contains string substring)` -- returns T if string contains substring, NIL otherwise.
fn native_str_contains(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-contains")?;
    let substring = require_string_arg(&args, 1, "str-contains")?;
    if text.contains(&substring) {
        Ok(Value::True)
    } else {
        Ok(Value::NIL)
    }
}

/// `(str-replace string old new)` -- replaces all occurrences of old with new.
fn native_str_replace(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-replace")?;
    let old = require_string_arg(&args, 1, "str-replace")?;
    let new = require_string_arg(&args, 2, "str-replace")?;
    Ok(Value::String(text.replace(&old, &new)))
}

/// `(str-substring string start end)` -- extracts substring (0-indexed, end exclusive).
fn native_str_substring(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-substring")?;
    let start = require_int_arg(&args, 1, "str-substring")? as usize;
    let end = require_int_arg(&args, 2, "str-substring")? as usize;
    if start > text.len() || end > text.len() || start > end {
        return Err(RuntimeError {
            msg: format!(
                "str-substring: indices [{}, {}) out of bounds for string of length {}",
                start,
                end,
                text.len()
            ),
        });
    }
    Ok(Value::String(text[start..end].to_string()))
}

/// `(str-trim string)` -- trims leading and trailing whitespace.
fn native_str_trim(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-trim")?;
    Ok(Value::String(text.trim().to_string()))
}

/// `(str-upper string)` -- converts string to uppercase.
fn native_str_upper(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-upper")?;
    Ok(Value::String(text.to_uppercase()))
}

/// `(str-lower string)` -- converts string to lowercase.
fn native_str_lower(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-lower")?;
    Ok(Value::String(text.to_lowercase()))
}

/// `(str-starts-with string prefix)` -- returns T if string starts with prefix, NIL otherwise.
fn native_str_starts_with(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-starts-with")?;
    let prefix = require_string_arg(&args, 1, "str-starts-with")?;
    if text.starts_with(&prefix) {
        Ok(Value::True)
    } else {
        Ok(Value::NIL)
    }
}

/// `(str-ends-with string suffix)` -- returns T if string ends with suffix, NIL otherwise.
fn native_str_ends_with(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-ends-with")?;
    let suffix = require_string_arg(&args, 1, "str-ends-with")?;
    if text.ends_with(&suffix) {
        Ok(Value::True)
    } else {
        Ok(Value::NIL)
    }
}

/// `(str-index-of string substring)` -- returns index of first occurrence, or -1 if not found.
fn native_str_index_of(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "str-index-of")?;
    let substring = require_string_arg(&args, 1, "str-index-of")?;
    match text.find(&substring) {
        Some(index) => Ok(Value::Int(index as i32)),
        None => Ok(Value::Int(-1)),
    }
}

/// `(str string1 string2 ...)` -- concatenates all string arguments (variadic).
fn native_str_variadic(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for (i, arg) in args.iter().enumerate() {
        match arg {
            Value::String(s) => result.push_str(s),
            other => {
                return Err(RuntimeError {
                    msg: format!("str: expected string as argument {}, got {}", i + 1, other),
                })
            }
        }
    }
    Ok(Value::String(result))
}

/// `(to-string value)` -- converts any value to its string representation.
fn native_to_string(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(value) => {
            let s = match value {
                Value::String(s) => s.clone(),
                Value::Int(n) => n.to_string(),
                Value::Float(f) => f.to_string(),
                Value::True => "T".to_string(),
                Value::False => "F".to_string(),
                Value::List(List::NIL) => "NIL".to_string(),
                other => format!("{}", other),
            };
            Ok(Value::String(s))
        }
        None => Err(RuntimeError {
            msg: "to-string: expected 1 argument, got 0".to_string(),
        }),
    }
}

/// `(parse-int string)` -- parses a string as an integer.
fn native_parse_int(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let text = require_string_arg(&args, 0, "parse-int")?;
    match text.trim().parse::<i32>() {
        Ok(n) => Ok(Value::Int(n)),
        Err(e) => Err(RuntimeError {
            msg: format!("parse-int: cannot parse \"{}\" as integer: {}", text, e),
        }),
    }
}

// ---------------------------------------------------------------------------
// Argument extraction helpers for string primitives
// ---------------------------------------------------------------------------

/// Extracts a required string argument at the given index.
fn require_string_arg(args: &[Value], index: usize, fn_name: &str) -> Result<String, RuntimeError> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(RuntimeError {
            msg: format!(
                "{}: expected string as argument {}, got {}",
                fn_name,
                index + 1,
                other
            ),
        }),
        None => Err(RuntimeError {
            msg: format!(
                "{}: expected argument {}, got only {}",
                fn_name,
                index + 1,
                args.len()
            ),
        }),
    }
}

/// Extracts a required integer argument at the given index.
fn require_int_arg(args: &[Value], index: usize, fn_name: &str) -> Result<i32, RuntimeError> {
    match args.get(index) {
        Some(Value::Int(n)) => Ok(*n),
        Some(other) => Err(RuntimeError {
            msg: format!(
                "{}: expected integer as argument {}, got {}",
                fn_name,
                index + 1,
                other
            ),
        }),
        None => Err(RuntimeError {
            msg: format!(
                "{}: expected argument {}, got only {}",
                fn_name,
                index + 1,
                args.len()
            ),
        }),
    }
}

// ---------------------------------------------------------------------------
// List primitive implementations (pure functions, no captured state)
// ---------------------------------------------------------------------------

/// Registers all list manipulation and type-checking primitives into the Lisp runtime.
///
/// These are pure functions (no EditorState access) registered as `NativeFunc`
/// function pointers. After calling this, the following Lisp functions become available:
///
/// - `(length list)` -- return the number of elements in a list
/// - `(nth n list)` -- return element at index n (0-based), NIL if out of bounds
/// - `(first list)` -- return first element, NIL if empty
/// - `(rest list)` -- return list without first element
/// - `(cons element list)` -- prepend element to list
/// - `(append list1 list2)` -- concatenate two lists
/// - `(reverse list)` -- reverse a list
/// - `(range start end)` -- generate list of integers from start to end-1
/// - `(map fn list)` -- apply fn to each element, return new list
/// - `(filter fn list)` -- keep elements where fn returns truthy
/// - `(reduce fn init list)` -- fold left with accumulator
/// - `(for-each fn list)` -- call fn on each element for side effects, returns NIL
/// - `(list? value)` -- T if value is a list, NIL otherwise
/// - `(string? value)` -- T if value is a string, NIL otherwise
/// - `(number? value)` -- T if value is an integer, NIL otherwise
/// - `(nil? value)` -- T if value is NIL, NIL otherwise
pub fn register_list_primitives(runtime: &LispRuntime) {
    let env = runtime.env();

    env.borrow_mut().define(
        Symbol("length".to_string()),
        Value::NativeFunc(native_length),
    );
    env.borrow_mut()
        .define(Symbol("nth".to_string()), Value::NativeFunc(native_nth));
    env.borrow_mut()
        .define(Symbol("first".to_string()), Value::NativeFunc(native_first));
    env.borrow_mut()
        .define(Symbol("rest".to_string()), Value::NativeFunc(native_rest));
    env.borrow_mut()
        .define(Symbol("cons".to_string()), Value::NativeFunc(native_cons));
    env.borrow_mut().define(
        Symbol("append".to_string()),
        Value::NativeFunc(native_append),
    );
    env.borrow_mut().define(
        Symbol("reverse".to_string()),
        Value::NativeFunc(native_reverse),
    );
    env.borrow_mut()
        .define(Symbol("range".to_string()), Value::NativeFunc(native_range));
    env.borrow_mut()
        .define(Symbol("map".to_string()), Value::NativeFunc(native_map));
    env.borrow_mut().define(
        Symbol("filter".to_string()),
        Value::NativeFunc(native_filter),
    );
    env.borrow_mut().define(
        Symbol("reduce".to_string()),
        Value::NativeFunc(native_reduce),
    );
    env.borrow_mut().define(
        Symbol("for-each".to_string()),
        Value::NativeFunc(native_for_each),
    );
    env.borrow_mut().define(
        Symbol("list?".to_string()),
        Value::NativeFunc(native_is_list),
    );
    env.borrow_mut().define(
        Symbol("string?".to_string()),
        Value::NativeFunc(native_is_string),
    );
    env.borrow_mut().define(
        Symbol("number?".to_string()),
        Value::NativeFunc(native_is_number),
    );
    env.borrow_mut()
        .define(Symbol("nil?".to_string()), Value::NativeFunc(native_is_nil));

    // Register '=' as an alias for '==' (rust_lisp built-in).
    // Standard Lisp/Scheme uses '=' for numeric equality, which is what
    // rust_lisp's '==' provides. This alias lets plugins use the familiar '='.
    env.borrow_mut().define(
        Symbol("=".to_string()),
        Value::NativeFunc(|_env, args| {
            let a = args
                .first()
                .cloned()
                .ok_or_else(|| rust_lisp::model::RuntimeError {
                    msg: "= requires 2 arguments".to_string(),
                })?;
            let b = args
                .get(1)
                .cloned()
                .ok_or_else(|| rust_lisp::model::RuntimeError {
                    msg: "= requires 2 arguments".to_string(),
                })?;
            Ok(if a == b { Value::True } else { Value::False })
        }),
    );
}

/// Extracts a required list argument at the given index.
fn require_list_arg(args: &[Value], index: usize, fn_name: &str) -> Result<List, RuntimeError> {
    match args.get(index) {
        Some(Value::List(l)) => Ok(l.clone()),
        Some(other) => Err(RuntimeError {
            msg: format!(
                "{}: expected list as argument {}, got {}",
                fn_name,
                index + 1,
                other
            ),
        }),
        None => Err(RuntimeError {
            msg: format!(
                "{}: expected argument {}, got only {}",
                fn_name,
                index + 1,
                args.len()
            ),
        }),
    }
}

/// Evaluates a function call expression `(func arg)` in the given environment.
///
/// List arguments are wrapped in `(quote ...)` to prevent the evaluator from
/// treating them as function calls. Without quoting, a list like `(0 1 "color")`
/// would be evaluated as a call to `0`, producing "0 is not callable".
fn call_lisp_function(
    env: Rc<RefCell<Env>>,
    func: &Value,
    arg: &Value,
) -> Result<Value, RuntimeError> {
    let safe_arg = quote_if_list(arg);
    let call_list: List = vec![func.clone(), safe_arg].into_iter().collect();
    let call_expr = Value::List(call_list);
    rust_lisp::interpreter::eval(env, &call_expr)
}

/// Wraps a Value in `(quote <value>)` if it is a non-empty list, preventing
/// the evaluator from interpreting it as a function call.
fn quote_if_list(value: &Value) -> Value {
    match value {
        Value::List(list) if *list != List::NIL => {
            let quote_list: List = vec![Value::Symbol(Symbol("quote".to_string())), value.clone()]
                .into_iter()
                .collect();
            Value::List(quote_list)
        }
        _ => value.clone(),
    }
}

/// `(length list)` -- returns the number of elements in a list.
fn native_length(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = require_list_arg(&args, 0, "length")?;
    let count = list.into_iter().count() as i32;
    Ok(Value::Int(count))
}

/// `(nth n list)` -- returns element at index n (0-based), NIL if out of bounds.
fn native_nth(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let index = require_int_arg(&args, 0, "nth")?;
    let list = require_list_arg(&args, 1, "nth")?;

    if index < 0 {
        return Ok(Value::NIL);
    }

    match list.into_iter().nth(index as usize) {
        Some(value) => Ok(value),
        None => Ok(Value::NIL),
    }
}

/// `(first list)` -- returns first element, NIL if empty.
fn native_first(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = require_list_arg(&args, 0, "first")?;
    match list.car() {
        Ok(value) => Ok(value),
        Err(_) => Ok(Value::NIL),
    }
}

/// `(rest list)` -- returns list without first element.
fn native_rest(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = require_list_arg(&args, 0, "rest")?;
    Ok(Value::List(list.cdr()))
}

/// `(cons element list)` -- prepends element to list.
fn native_cons(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let element = args.first().ok_or_else(|| RuntimeError {
        msg: "cons: expected 2 arguments, got 0".to_string(),
    })?;
    let list = require_list_arg(&args, 1, "cons")?;
    Ok(Value::List(list.cons(element.clone())))
}

/// `(append list1 list2)` -- concatenates two lists.
fn native_append(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list1 = require_list_arg(&args, 0, "append")?;
    let list2 = require_list_arg(&args, 1, "append")?;
    let combined: List = list1.into_iter().chain(&list2).collect();
    Ok(Value::List(combined))
}

/// `(reverse list)` -- reverses a list.
fn native_reverse(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let list = require_list_arg(&args, 0, "reverse")?;
    let items: Vec<Value> = list.into_iter().collect();
    let reversed: List = items.into_iter().rev().collect();
    Ok(Value::List(reversed))
}

/// `(range start end)` -- generates a list of integers from start to end-1.
fn native_range(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let start = require_int_arg(&args, 0, "range")?;
    let end = require_int_arg(&args, 1, "range")?;
    let list: List = (start..end).map(Value::Int).collect();
    Ok(Value::List(list))
}

/// `(map fn list)` -- applies fn to each element, returns new list.
fn native_map(env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let func = args.first().ok_or_else(|| RuntimeError {
        msg: "map: expected 2 arguments (fn, list), got 0".to_string(),
    })?;
    let list = require_list_arg(&args, 1, "map")?;

    let results: Result<Vec<Value>, RuntimeError> = list
        .into_iter()
        .map(|item| call_lisp_function(env.clone(), func, &item))
        .collect();

    let result_list: List = results?.into_iter().collect();
    Ok(Value::List(result_list))
}

/// `(filter fn list)` -- keeps elements where fn returns truthy.
fn native_filter(env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let func = args.first().ok_or_else(|| RuntimeError {
        msg: "filter: expected 2 arguments (fn, list), got 0".to_string(),
    })?;
    let list = require_list_arg(&args, 1, "filter")?;

    let mut kept = Vec::new();
    for item in &list {
        let result = call_lisp_function(env.clone(), func, &item)?;
        let is_truthy: bool = (&result).into();
        if is_truthy {
            kept.push(item);
        }
    }

    let result_list: List = kept.into_iter().collect();
    Ok(Value::List(result_list))
}

/// `(reduce fn init list)` -- folds left with accumulator.
fn native_reduce(env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let func = args.first().ok_or_else(|| RuntimeError {
        msg: "reduce: expected 3 arguments (fn, init, list), got 0".to_string(),
    })?;
    let init = args.get(1).ok_or_else(|| RuntimeError {
        msg: "reduce: expected 3 arguments (fn, init, list), got 1".to_string(),
    })?;
    let list = require_list_arg(&args, 2, "reduce")?;

    let mut accumulator = init.clone();
    for item in &list {
        // Build a call expression: (func accumulator item)
        let call_list: List = vec![func.clone(), accumulator, item].into_iter().collect();
        let call_expr = Value::List(call_list);
        accumulator = rust_lisp::interpreter::eval(env.clone(), &call_expr)?;
    }

    Ok(accumulator)
}

/// `(for-each fn list)` -- calls fn on each element for side effects, returns NIL.
fn native_for_each(env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let func = args.first().ok_or_else(|| RuntimeError {
        msg: "for-each: expected 2 arguments (fn, list), got 0".to_string(),
    })?;
    let list = require_list_arg(&args, 1, "for-each")?;

    for item in &list {
        call_lisp_function(env.clone(), func, &item)?;
    }

    Ok(Value::NIL)
}

/// `(list? value)` -- returns T if value is a list (including NIL), NIL otherwise.
fn native_is_list(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(Value::List(_)) => Ok(Value::True),
        Some(_) => Ok(Value::NIL),
        None => Err(RuntimeError {
            msg: "list?: expected 1 argument, got 0".to_string(),
        }),
    }
}

/// `(string? value)` -- returns T if value is a string, NIL otherwise.
fn native_is_string(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(Value::String(_)) => Ok(Value::True),
        Some(_) => Ok(Value::NIL),
        None => Err(RuntimeError {
            msg: "string?: expected 1 argument, got 0".to_string(),
        }),
    }
}

/// `(number? value)` -- returns T if value is an integer, NIL otherwise.
fn native_is_number(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(Value::Int(_)) => Ok(Value::True),
        Some(_) => Ok(Value::NIL),
        None => Err(RuntimeError {
            msg: "number?: expected 1 argument, got 0".to_string(),
        }),
    }
}

/// `(nil? value)` -- returns T if value is NIL, NIL otherwise.
fn native_is_nil(_env: Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.first() {
        Some(Value::List(List::NIL)) => Ok(Value::True),
        Some(_) => Ok(Value::NIL),
        None => Err(RuntimeError {
            msg: "nil?: expected 1 argument, got 0".to_string(),
        }),
    }
}

/// Registers filesystem primitives for the folder browser plugin.
///
/// After calling this, the following Lisp functions become available:
/// - `(list-dir path)` -- list directory entries as `((name type) ...)`
/// - `(is-dir? path)` -- returns `#t` if path is a directory
/// - `(path-join base child)` -- joins two path components
/// - `(path-parent path)` -- returns the parent directory of path
/// - `(cli-argument)` -- returns the CLI argument Alfred was started with
/// - `(open-file path)` -- opens a file into the editor buffer
pub fn register_filesystem_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_list_dir(env.clone());
    register_is_dir(env.clone());
    register_path_join(env.clone());
    register_path_parent(env.clone());
    register_cli_argument(env.clone(), state.clone());
    register_open_file(env, state);
}

/// Registers `list-dir`: lists directory entries sorted dirs-first then files.
///
/// Usage: `(list-dir "/some/path")`
///
/// Returns a list of `(name type)` pairs where type is `"file"`, `"dir"`, or `"symlink"`.
/// Directories appear first (alphabetical), then files (alphabetical).
/// Returns empty list on error (permission denied, not a directory, etc.).
fn register_list_dir(env: Rc<RefCell<Env>>) {
    define_native_closure(&env, "list-dir", move |_env, args| {
        let path_str = extract_string_arg(&args, "list-dir")?;
        let path = std::path::Path::new(&path_str);

        let read_dir = match std::fs::read_dir(path) {
            Ok(rd) => rd,
            Err(_) => return Ok(Value::List(List::NIL)),
        };

        let mut dirs: Vec<(String, String)> = Vec::new();
        let mut files: Vec<(String, String)> = Vec::new();

        for entry_result in read_dir {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().ok();
            let type_label = match file_type {
                Some(ft) if ft.is_dir() => "dir",
                Some(ft) if ft.is_symlink() => "symlink",
                _ => "file",
            };

            if type_label == "dir" {
                dirs.push((name, type_label.to_string()));
            } else {
                files.push((name, type_label.to_string()));
            }
        }

        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut all_entries: Vec<Value> = Vec::with_capacity(dirs.len() + files.len());
        for (name, type_label) in dirs.into_iter().chain(files.into_iter()) {
            let pair: List = vec![Value::String(name), Value::String(type_label)]
                .into_iter()
                .collect();
            all_entries.push(Value::List(pair));
        }

        let result_list: List = all_entries.into_iter().collect();
        Ok(Value::List(result_list))
    });
}

/// Registers `is-dir?`: checks if a path is a directory.
///
/// Usage: `(is-dir? "/some/path")`
///
/// Returns `#t` if the path is a directory, `#f` (NIL) otherwise.
fn register_is_dir(env: Rc<RefCell<Env>>) {
    define_native_closure(&env, "is-dir?", move |_env, args| {
        let path_str = extract_string_arg(&args, "is-dir?")?;
        let path = std::path::Path::new(&path_str);
        if path.is_dir() {
            Ok(Value::True)
        } else {
            Ok(Value::NIL)
        }
    });
}

/// Registers `path-join`: joins two path components.
///
/// Usage: `(path-join "/home" "user")` -> `"/home/user"`
fn register_path_join(env: Rc<RefCell<Env>>) {
    define_native_closure(&env, "path-join", move |_env, args| {
        let base = extract_string_arg_at(&args, 0, "path-join", "base")?;
        let child = extract_string_arg_at(&args, 1, "path-join", "child")?;
        let joined = std::path::Path::new(&base).join(&child);
        Ok(Value::String(joined.to_string_lossy().to_string()))
    });
}

/// Registers `path-parent`: returns the parent directory of a path.
///
/// Usage: `(path-parent "/home/user")` -> `"/home"`
///
/// Returns empty string if the path has no parent (e.g., root `/`).
fn register_path_parent(env: Rc<RefCell<Env>>) {
    define_native_closure(&env, "path-parent", move |_env, args| {
        let path_str = extract_string_arg(&args, "path-parent")?;
        let path = std::path::Path::new(&path_str);
        match path.parent() {
            Some(parent) => Ok(Value::String(parent.to_string_lossy().to_string())),
            None => Ok(Value::String(String::new())),
        }
    });
}

/// Registers `cli-argument`: returns the CLI argument Alfred was started with.
///
/// Usage: `(cli-argument)` -> `"/path/to/file"` or `""`
fn register_cli_argument(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "cli-argument", move |_env, _args| {
        let editor = state.borrow();
        match &editor.cli_argument {
            Some(arg) => Ok(Value::String(arg.clone())),
            None => Ok(Value::String(String::new())),
        }
    });
}

/// Registers `open-file`: opens a file into the editor buffer.
///
/// Usage: `(open-file "/path/to/file.txt")`
///
/// Replaces the current buffer, resets cursor to (0,0), resets viewport,
/// sets mode to "normal" with active keymaps `["normal-mode"]`,
/// and sets message to the filename.
/// On error, sets message to the error text and returns NIL.
fn register_open_file(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "open-file", move |_env, args| {
        let path_str = extract_string_arg(&args, "open-file")?;
        let path = std::path::Path::new(&path_str);

        match alfred_core::buffer::Buffer::from_file(path) {
            Ok(new_buffer) => {
                let filename = new_buffer.filename().unwrap_or(&path_str).to_string();
                let mut editor = state.borrow_mut();
                editor.buffer = new_buffer;
                editor.line_styles.clear();
                editor.line_backgrounds.clear();
                editor.cursor = cursor::new(0, 0);
                editor.viewport.top_line = 0;
                editor.mode = "normal".to_string();
                editor.active_keymaps = vec!["normal-mode".to_string()];
                editor.focused_panel = None;
                editor.message = Some(filename);
                Ok(Value::NIL)
            }
            Err(e) => {
                let mut editor = state.borrow_mut();
                editor.message = Some(format!("{}", e));
                Ok(Value::NIL)
            }
        }
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
    // Bug reproduction: define-command callback execution
    // -----------------------------------------------------------------------

    /// Helper: execute a Dynamic command through the ClonedHandler path,
    /// avoiding RefCell double-borrow (mirrors the TUI dispatch logic).
    fn execute_dynamic_command(
        state: &Rc<RefCell<EditorState>>,
        name: &str,
    ) -> alfred_core::error::Result<()> {
        let handler = state.borrow().commands.extract_handler(name);
        match handler {
            Some(alfred_core::command::ClonedHandler::Dynamic(f)) => {
                // Dynamic handlers capture their own Rc<RefCell<EditorState>>.
                // We must NOT hold a borrow here. Pass a dummy state.
                let mut dummy = editor_state::new(1, 1);
                f(&mut dummy)
            }
            Some(alfred_core::command::ClonedHandler::Native(f)) => f(&mut state.borrow_mut()),
            None => Err(alfred_core::error::AlfredError::CommandNotFound {
                name: name.to_string(),
            }),
        }
    }

    #[test]
    fn given_define_command_with_simple_lambda_when_executed_then_callback_runs() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());

        runtime
            .eval(r#"(define-command "simple-msg" (lambda () (message "direct")))"#)
            .unwrap();

        // Execute through the Dynamic-safe path (no RefCell double-borrow)
        execute_dynamic_command(&state, "simple-msg").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.message, Some("direct".to_string()));
    }

    // -----------------------------------------------------------------------
    // Bug reproduction: define-command env scoping (define then define-command)
    // -----------------------------------------------------------------------

    #[test]
    fn given_define_then_define_command_when_executed_then_can_access_defined_variable() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());

        // Define a variable, then a command that reads it
        runtime
            .eval(
                r#"
            (define greeting "hi there")
            (define-command "test-greet" (lambda () (message greeting)))
        "#,
            )
            .unwrap();

        // Execute through the Dynamic-safe path (no RefCell double-borrow)
        execute_dynamic_command(&state, "test-greet").unwrap();

        // Verify the message was set using the defined variable
        let editor = state.borrow();
        assert_eq!(editor.message, Some("hi there".to_string()));
    }

    #[test]
    fn given_define_then_define_command_when_executed_then_can_access_defined_function() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());
        register_string_primitives(&runtime);
        register_list_primitives(&runtime);

        // Define a helper function, then a command that calls it
        runtime
            .eval(
                r#"
            (define greet (lambda (name) (str-join (list "Hello" name) " ")))
            (define-command "test-greet" (lambda () (message (greet "World"))))
        "#,
            )
            .unwrap();

        // Execute through the Dynamic-safe path (no RefCell double-borrow)
        execute_dynamic_command(&state, "test-greet").unwrap();

        // Verify the message was set by the helper function
        let editor = state.borrow();
        assert_eq!(editor.message, Some("Hello World".to_string()));
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
            Some(&"enter-operator-delete".to_string()),
            "d should be bound to enter-operator-delete"
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

    // -----------------------------------------------------------------------
    // Acceptance test (10-03): set-theme-color then get-theme-color round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn given_theme_primitives_when_set_then_get_theme_color_then_returns_stored_value() {
        // Given: an editor state with theme primitives registered
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // When: set-theme-color is called, then get-theme-color retrieves it
        runtime
            .eval("(set-theme-color \"status-bar-bg\" \"#3c3836\")")
            .unwrap();
        let result = runtime.eval("(get-theme-color \"status-bar-bg\")").unwrap();

        // Then: the retrieved color matches what was set
        assert_eq!(result.as_string(), Some("#3c3836".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests (10-03): theme primitives
    // Test Budget: 6 behaviors x 2 = 12 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_theme_primitives_when_set_theme_color_with_hex_then_stores_in_theme() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        runtime
            .eval("(set-theme-color \"text-fg\" \"#ff5733\")")
            .unwrap();

        let editor = state.borrow();
        assert_eq!(
            editor.theme.get("text-fg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(255, 87, 51))
        );
    }

    #[test]
    fn given_theme_primitives_when_set_theme_color_with_named_then_stores_in_theme() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        runtime
            .eval("(set-theme-color \"gutter-fg\" \"cyan\")")
            .unwrap();

        let editor = state.borrow();
        assert_eq!(
            editor.theme.get("gutter-fg"),
            Some(&alfred_core::theme::ThemeColor::Named(
                alfred_core::theme::NamedColor::Cyan
            ))
        );
    }

    #[test]
    fn given_theme_primitives_when_set_theme_color_with_invalid_color_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-theme-color \"text-fg\" \"not-a-color\")");

        assert!(result.is_err());
    }

    #[test]
    fn given_theme_primitives_when_get_theme_color_missing_key_then_returns_nil() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-theme-color \"nonexistent\")").unwrap();

        assert_eq!(*result.inner(), Value::NIL);
    }

    #[test]
    fn given_theme_primitives_when_define_theme_with_pairs_then_stores_named_theme() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        runtime
            .eval("(define-theme \"gruvbox\" \"text-fg\" \"#ebdbb2\" \"text-bg\" \"#282828\")")
            .unwrap();

        let editor = state.borrow();
        let theme = editor
            .named_themes
            .get("gruvbox")
            .expect("theme should exist");
        assert_eq!(
            theme.get("text-fg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(235, 219, 178))
        );
        assert_eq!(
            theme.get("text-bg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(40, 40, 40))
        );
    }

    #[test]
    fn given_theme_primitives_when_define_theme_with_odd_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // Odd number of key-value arguments (name + 3 args = odd pairs)
        let result = runtime.eval("(define-theme \"bad\" \"key1\" \"#ff0000\" \"orphan\")");

        assert!(result.is_err());
    }

    #[test]
    fn given_theme_primitives_when_define_theme_with_invalid_color_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        let result = runtime.eval("(define-theme \"bad\" \"key1\" \"not-a-color\")");

        assert!(result.is_err());
    }

    #[test]
    fn given_defined_theme_when_load_theme_then_copies_colors_to_active_theme() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // Define a theme, then load it
        runtime
            .eval("(define-theme \"dracula\" \"text-fg\" \"#f8f8f2\" \"text-bg\" \"#282a36\")")
            .unwrap();
        runtime.eval("(load-theme \"dracula\")").unwrap();

        // Active theme should now contain the dracula colors
        let editor = state.borrow();
        assert_eq!(
            editor.theme.get("text-fg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(248, 248, 242))
        );
        assert_eq!(
            editor.theme.get("text-bg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(40, 42, 54))
        );
    }

    #[test]
    fn given_theme_primitives_when_load_theme_nonexistent_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        let result = runtime.eval("(load-theme \"nonexistent\")");

        assert!(result.is_err());
    }

    #[test]
    fn given_active_theme_with_colors_when_load_theme_then_overwrites_matching_keys() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // Set a color in active theme first
        runtime
            .eval("(set-theme-color \"text-fg\" \"#000000\")")
            .unwrap();

        // Define and load a theme that overrides text-fg
        runtime
            .eval("(define-theme \"override\" \"text-fg\" \"#ffffff\")")
            .unwrap();
        runtime.eval("(load-theme \"override\")").unwrap();

        // The active theme text-fg should be overridden
        let editor = state.borrow();
        assert_eq!(
            editor.theme.get("text-fg"),
            Some(&alfred_core::theme::ThemeColor::Rgb(255, 255, 255))
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (10-04): loading default-theme plugin sets all standard
    // color slots on EditorState
    // -----------------------------------------------------------------------

    #[test]
    fn given_default_theme_plugin_when_loaded_then_all_standard_color_slots_are_set() {
        // Given: an editor state with theme primitives registered
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // When: the default-theme plugin source is evaluated
        let plugin_source = std::fs::read_to_string("../../plugins/default-theme/init.lisp")
            .expect("default-theme plugin should exist at plugins/default-theme/init.lisp");

        // Filter out comment lines to only eval Lisp forms
        let lisp_forms: String = plugin_source
            .lines()
            .filter(|line| !line.trim_start().starts_with(';'))
            .collect::<Vec<_>>()
            .join("\n");

        for line in lisp_forms.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                runtime.eval(trimmed).unwrap();
            }
        }

        // Then: color slots with explicit values are set on the theme
        let editor = state.borrow();

        // Status bar has distinct background color (RGB, not "default")
        assert!(
            editor.theme.contains_key("status-bar-bg"),
            "status-bar-bg should be set to an explicit color"
        );
        assert!(
            editor.theme.contains_key("status-bar-fg"),
            "status-bar-fg should be set to an explicit color"
        );

        // Gutter has distinct foreground color
        assert!(
            editor.theme.contains_key("gutter-fg"),
            "gutter-fg should be set to an explicit color"
        );

        // "default" color values are removed from theme (terminal defaults)
        // text-fg, text-bg, gutter-bg, message-fg, message-bg are all "default"
        assert!(
            !editor.theme.contains_key("text-fg"),
            "text-fg with 'default' value should not be in theme"
        );
        assert!(
            !editor.theme.contains_key("message-fg"),
            "message-fg with 'default' value should not be in theme"
        );

        // Verify status-bar-bg is an RGB color for a distinct background
        let status_bg = editor.theme.get("status-bar-bg").unwrap();
        assert!(
            matches!(status_bg, alfred_core::theme::ThemeColor::Rgb(_, _, _)),
            "status-bar-bg should be an RGB color for a distinct background, got: {:?}",
            status_bg
        );

        // Verify gutter-fg is a specific color
        let gutter_fg = editor.theme.get("gutter-fg").unwrap();
        assert!(
            matches!(
                gutter_fg,
                alfred_core::theme::ThemeColor::Rgb(_, _, _)
                    | alfred_core::theme::ThemeColor::Named(_)
            ),
            "gutter-fg should be a specific color, got: {:?}",
            gutter_fg
        );

        // Verify syntax highlight color slots are set
        let syntax_slots = [
            "syntax-keyword",
            "syntax-function",
            "syntax-string",
            "syntax-comment",
            "syntax-type",
            "syntax-variable",
            "syntax-operator",
            "syntax-number",
            "syntax-punctuation",
            "syntax-property",
            "syntax-attribute",
            "syntax-constant",
            "syntax-constructor",
        ];
        for slot in &syntax_slots {
            assert!(
                editor.theme.contains_key(*slot),
                "syntax slot '{}' should be set by default-theme plugin",
                slot
            );
            let color = editor.theme.get(*slot).unwrap();
            assert!(
                matches!(color, alfred_core::theme::ThemeColor::Rgb(_, _, _)),
                "syntax slot '{}' should be an RGB color, got: {:?}",
                slot,
                color
            );
        }
    }

    // -----------------------------------------------------------------------
    // Unit test (10-04): empty theme falls back to terminal defaults when no
    // plugin loaded
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_theme_plugin_loaded_when_theme_queried_then_all_slots_empty() {
        // Given: an editor state with theme primitives but no plugin loaded
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_theme_primitives(&runtime, state.clone());

        // Then: the theme is empty (no color slots set)
        let editor = state.borrow();
        assert!(
            editor.theme.is_empty(),
            "Theme should be empty without plugin, but has {} entries",
            editor.theme.len()
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test: set-cursor-shape + get-cursor-shape round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_when_set_cursor_shape_and_get_cursor_shape_then_round_trips() {
        // Given: an editor state with core primitives registered
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        // When: set-cursor-shape is called for insert mode with "blinking-bar"
        runtime
            .eval("(set-cursor-shape \"insert\" \"blinking-bar\")")
            .unwrap();

        // Then: get-cursor-shape returns "blinking-bar" for insert mode
        let result = runtime.eval("(get-cursor-shape \"insert\")").unwrap();
        assert_eq!(result.as_string(), Some("blinking-bar".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests: set-cursor-shape primitive
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_runtime_when_set_cursor_shape_with_valid_shape_then_updates_state() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime
            .eval("(set-cursor-shape \"normal\" \"blinking-block\")")
            .unwrap();

        let editor = state.borrow();
        assert_eq!(
            editor.cursor_shapes.get("normal"),
            Some(&"blinking-block".to_string())
        );
    }

    #[test]
    fn given_runtime_when_set_cursor_shape_with_invalid_shape_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-cursor-shape \"normal\" \"triangle\")");
        assert!(result.is_err(), "Invalid shape name should return error");
    }

    #[test]
    fn given_runtime_when_set_cursor_shape_with_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-cursor-shape)");
        assert!(result.is_err(), "No args should return error");
    }

    #[test]
    fn given_runtime_when_set_cursor_shape_with_one_arg_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-cursor-shape \"normal\")");
        assert!(result.is_err(), "One arg should return error");
    }

    // -----------------------------------------------------------------------
    // Unit tests: get-cursor-shape primitive
    // -----------------------------------------------------------------------

    #[test]
    fn given_default_state_when_get_cursor_shape_for_normal_then_returns_block() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-cursor-shape \"normal\")").unwrap();
        assert_eq!(result.as_string(), Some("block".to_string()));
    }

    #[test]
    fn given_default_state_when_get_cursor_shape_for_insert_then_returns_bar() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-cursor-shape \"insert\")").unwrap();
        assert_eq!(result.as_string(), Some("bar".to_string()));
    }

    #[test]
    fn given_runtime_when_get_cursor_shape_for_unknown_mode_then_returns_nil() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-cursor-shape \"unknown-mode\")").unwrap();
        assert_eq!(result.inner().clone(), Value::NIL);
    }

    #[test]
    fn given_runtime_when_get_cursor_shape_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-cursor-shape)");
        assert!(result.is_err(), "No args should return error");
    }

    #[test]
    fn given_runtime_when_set_cursor_shape_for_custom_mode_then_get_returns_it() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime
            .eval("(set-cursor-shape \"visual\" \"underline\")")
            .unwrap();

        let result = runtime.eval("(get-cursor-shape \"visual\")").unwrap();
        assert_eq!(result.as_string(), Some("underline".to_string()));
    }

    // -----------------------------------------------------------------------
    // Unit tests: set-tab-width and get-tab-width primitives
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_default_state_when_get_tab_width_then_returns_4() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(get-tab-width)").unwrap();
        assert_eq!(
            result.as_integer(),
            Some(4),
            "Default tab width should be 4"
        );
    }

    #[test]
    fn given_runtime_when_set_tab_width_to_2_then_get_returns_2() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(set-tab-width 2)").unwrap();

        let result = runtime.eval("(get-tab-width)").unwrap();
        assert_eq!(
            result.as_integer(),
            Some(2),
            "Tab width should be 2 after set"
        );
    }

    #[test]
    fn given_runtime_when_set_tab_width_to_8_then_state_reflects_change() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        runtime.eval("(set-tab-width 8)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.tab_width, 8, "EditorState.tab_width should be 8");
    }

    #[test]
    fn given_runtime_when_set_tab_width_to_zero_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-tab-width 0)");
        assert!(
            result.is_err(),
            "set-tab-width with 0 should return an error"
        );
    }

    #[test]
    fn given_runtime_when_set_tab_width_to_negative_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-tab-width -1)");
        assert!(
            result.is_err(),
            "set-tab-width with negative value should return an error"
        );
    }

    #[test]
    fn given_runtime_when_set_tab_width_with_string_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-tab-width \"four\")");
        assert!(
            result.is_err(),
            "set-tab-width with string argument should return an error"
        );
    }

    #[test]
    fn given_runtime_when_set_tab_width_with_no_args_then_returns_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());

        let result = runtime.eval("(set-tab-width)");
        assert!(
            result.is_err(),
            "set-tab-width with no args should return an error"
        );
    }

    // -----------------------------------------------------------------------
    // Buffer style primitives: clear-line-styles, set-line-style,
    // buffer-line-count, buffer-get-line
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_styled_buffer_when_clear_line_styles_evaluated_then_line_styles_empty() {
        // Given: an editor state with line styles applied
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("a,b,c");
        }

        let runtime = LispRuntime::new();
        register_buffer_style_primitives(&runtime, state.clone());

        // Add a style segment via the primitive
        runtime.eval("(set-line-style 0 0 1 \"#ff6b6b\")").unwrap();
        assert!(!state.borrow().line_styles.is_empty());

        // When: clear-line-styles is evaluated
        runtime.eval("(clear-line-styles)").unwrap();

        // Then: line_styles is empty
        assert!(state.borrow().line_styles.is_empty());
    }

    #[test]
    fn given_buffer_when_buffer_line_count_evaluated_then_returns_correct_count() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("a\nb\nc");
        }

        let runtime = LispRuntime::new();
        register_buffer_style_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-line-count)").unwrap();
        assert_eq!(*result.inner(), Value::Int(3));
    }

    #[test]
    fn given_buffer_when_buffer_get_line_evaluated_then_returns_line_content() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("hello\nworld");
        }

        let runtime = LispRuntime::new();
        register_buffer_style_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-get-line 0)").unwrap();
        assert_eq!(*result.inner(), Value::String("hello".to_string()));

        let result = runtime.eval("(buffer-get-line 1)").unwrap();
        assert_eq!(*result.inner(), Value::String("world".to_string()));
    }

    // -----------------------------------------------------------------------
    // String primitives tests
    // Test Budget: 15 primitives x 2 (happy + edge) = 30 max
    // -----------------------------------------------------------------------

    /// Helper: creates a LispRuntime with string primitives registered.
    fn runtime_with_string_primitives() -> LispRuntime {
        let runtime = LispRuntime::new();
        register_string_primitives(&runtime);
        runtime
    }

    // -- str-split --

    #[test]
    fn given_delimited_string_when_str_split_then_returns_list_of_parts() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-split \"a,b,c\" \",\")").unwrap();
        match result.inner() {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::String("a".to_string()));
                assert_eq!(items[1], Value::String("b".to_string()));
                assert_eq!(items[2], Value::String("c".to_string()));
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn given_string_without_delimiter_when_str_split_then_returns_single_element_list() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-split \"hello\" \",\")").unwrap();
        match result.inner() {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], Value::String("hello".to_string()));
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    // -- str-join --

    #[test]
    fn given_list_of_strings_when_str_join_then_returns_joined_string() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-join (list \"a\" \"b\") \",\")").unwrap();
        assert_eq!(result.as_string(), Some("a,b".to_string()));
    }

    #[test]
    fn given_empty_list_when_str_join_then_returns_empty_string() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-join (list) \",\")").unwrap();
        assert_eq!(result.as_string(), Some("".to_string()));
    }

    // -- str-length --

    #[test]
    fn given_nonempty_string_when_str_length_then_returns_correct_length() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-length \"hello\")").unwrap();
        assert_eq!(result.as_integer(), Some(5));
    }

    #[test]
    fn given_empty_string_when_str_length_then_returns_zero() {
        let runtime = runtime_with_string_primitives();
        // rust_lisp parser does not support empty string literals "",
        // so we create one via str-substring
        let result = runtime
            .eval("(str-length (str-substring \"x\" 0 0))")
            .unwrap();
        assert_eq!(result.as_integer(), Some(0));
    }

    // -- str-contains --

    #[test]
    fn given_string_containing_substring_when_str_contains_then_returns_true() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-contains \"hello\" \"ell\")").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_string_not_containing_substring_when_str_contains_then_returns_nil() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-contains \"hello\" \"xyz\")").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- str-replace --

    #[test]
    fn given_string_with_pattern_when_str_replace_then_replaces_all_occurrences() {
        let runtime = runtime_with_string_primitives();
        let result = runtime
            .eval("(str-replace \"aabbcc\" \"bb\" \"XX\")")
            .unwrap();
        assert_eq!(result.as_string(), Some("aaXXcc".to_string()));
    }

    #[test]
    fn given_string_without_pattern_when_str_replace_then_returns_original() {
        let runtime = runtime_with_string_primitives();
        let result = runtime
            .eval("(str-replace \"hello\" \"xyz\" \"ABC\")")
            .unwrap();
        assert_eq!(result.as_string(), Some("hello".to_string()));
    }

    // -- str-substring --

    #[test]
    fn given_valid_indices_when_str_substring_then_returns_substring() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-substring \"hello\" 1 3)").unwrap();
        assert_eq!(result.as_string(), Some("el".to_string()));
    }

    #[test]
    fn given_out_of_bounds_indices_when_str_substring_then_returns_error() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-substring \"hi\" 0 10)");
        assert!(result.is_err(), "out-of-bounds should return error");
    }

    // -- str-trim --

    #[test]
    fn given_string_with_whitespace_when_str_trim_then_returns_trimmed() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-trim \"  hi  \")").unwrap();
        assert_eq!(result.as_string(), Some("hi".to_string()));
    }

    #[test]
    fn given_already_trimmed_string_when_str_trim_then_returns_same() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-trim \"hello\")").unwrap();
        assert_eq!(result.as_string(), Some("hello".to_string()));
    }

    // -- str-upper --

    #[test]
    fn given_lowercase_string_when_str_upper_then_returns_uppercase() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-upper \"hello\")").unwrap();
        assert_eq!(result.as_string(), Some("HELLO".to_string()));
    }

    #[test]
    fn given_empty_string_when_str_upper_then_returns_empty() {
        let runtime = runtime_with_string_primitives();
        // rust_lisp parser does not support empty string literals "",
        // so we create one via str-substring
        let result = runtime
            .eval("(str-upper (str-substring \"x\" 0 0))")
            .unwrap();
        assert_eq!(result.as_string(), Some("".to_string()));
    }

    // -- str-lower --

    #[test]
    fn given_uppercase_string_when_str_lower_then_returns_lowercase() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-lower \"HELLO\")").unwrap();
        assert_eq!(result.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn given_empty_string_when_str_lower_then_returns_empty() {
        let runtime = runtime_with_string_primitives();
        // rust_lisp parser does not support empty string literals "",
        // so we create one via str-substring
        let result = runtime
            .eval("(str-lower (str-substring \"x\" 0 0))")
            .unwrap();
        assert_eq!(result.as_string(), Some("".to_string()));
    }

    // -- str-starts-with --

    #[test]
    fn given_matching_prefix_when_str_starts_with_then_returns_true() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-starts-with \"hello\" \"he\")").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_non_matching_prefix_when_str_starts_with_then_returns_nil() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-starts-with \"hello\" \"lo\")").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- str-ends-with --

    #[test]
    fn given_matching_suffix_when_str_ends_with_then_returns_true() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-ends-with \"hello\" \"lo\")").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_non_matching_suffix_when_str_ends_with_then_returns_nil() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-ends-with \"hello\" \"he\")").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- str-index-of --

    #[test]
    fn given_string_containing_substring_when_str_index_of_then_returns_index() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-index-of \"hello\" \"ll\")").unwrap();
        assert_eq!(result.as_integer(), Some(2));
    }

    #[test]
    fn given_string_not_containing_substring_when_str_index_of_then_returns_negative_one() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str-index-of \"hello\" \"xyz\")").unwrap();
        assert_eq!(result.as_integer(), Some(-1));
    }

    // -- str (concatenation) --

    #[test]
    fn given_multiple_strings_when_str_then_returns_concatenation() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str \"a\" \"b\" \"c\")").unwrap();
        assert_eq!(result.as_string(), Some("abc".to_string()));
    }

    #[test]
    fn given_no_args_when_str_then_returns_empty_string() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(str)").unwrap();
        assert_eq!(result.as_string(), Some("".to_string()));
    }

    // -- to-string --

    #[test]
    fn given_integer_when_to_string_then_returns_string_representation() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(to-string 42)").unwrap();
        assert_eq!(result.as_string(), Some("42".to_string()));
    }

    #[test]
    fn given_string_when_to_string_then_returns_same_string() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(to-string \"hello\")").unwrap();
        assert_eq!(result.as_string(), Some("hello".to_string()));
    }

    // -- parse-int --

    #[test]
    fn given_valid_integer_string_when_parse_int_then_returns_integer() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(parse-int \"42\")").unwrap();
        assert_eq!(result.as_integer(), Some(42));
    }

    #[test]
    fn given_non_numeric_string_when_parse_int_then_returns_error() {
        let runtime = runtime_with_string_primitives();
        let result = runtime.eval("(parse-int \"abc\")");
        assert!(result.is_err(), "parse-int with non-numeric should error");
    }

    // -----------------------------------------------------------------------
    // List primitives tests
    // -----------------------------------------------------------------------

    /// Creates a runtime with list primitives registered.
    fn runtime_with_list_primitives() -> LispRuntime {
        let runtime = LispRuntime::new();
        register_list_primitives(&runtime);
        runtime
    }

    // -- length --

    #[test]
    fn given_non_empty_list_when_length_then_returns_count() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(length '(1 2 3))").unwrap();
        assert_eq!(result.as_integer(), Some(3));
    }

    #[test]
    fn given_empty_list_when_length_then_returns_zero() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(length '())").unwrap();
        assert_eq!(result.as_integer(), Some(0));
    }

    // -- nth --

    #[test]
    fn given_valid_index_when_nth_then_returns_element() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(nth 1 '(\"a\" \"b\" \"c\"))").unwrap();
        assert_eq!(result.as_string(), Some("b".to_string()));
    }

    #[test]
    fn given_out_of_bounds_index_when_nth_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(nth 5 '(1 2))").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- first --

    #[test]
    fn given_non_empty_list_when_first_then_returns_first_element() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(first '(10 20))").unwrap();
        assert_eq!(result.as_integer(), Some(10));
    }

    #[test]
    fn given_empty_list_when_first_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(first '())").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- rest --

    #[test]
    fn given_non_empty_list_when_rest_then_returns_tail() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(rest '(1 2 3))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(2), Value::Int(3)]);
            }
            _ => panic!("rest should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_single_element_list_when_rest_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(rest '(1))").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- cons --

    #[test]
    fn given_element_and_list_when_cons_then_returns_prepended_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(cons 0 '(1 2))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(0), Value::Int(1), Value::Int(2)]);
            }
            _ => panic!("cons should return a list, got {:?}", inner),
        }
    }

    // -- append --

    #[test]
    fn given_two_lists_when_append_then_returns_concatenated_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(append '(1 2) '(3 4))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(
                    items,
                    vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]
                );
            }
            _ => panic!("append should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_empty_and_non_empty_list_when_append_then_returns_second_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(append '() '(1 2))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(1), Value::Int(2)]);
            }
            _ => panic!("append should return a list, got {:?}", inner),
        }
    }

    // -- reverse --

    #[test]
    fn given_non_empty_list_when_reverse_then_returns_reversed_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(reverse '(1 2 3))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(3), Value::Int(2), Value::Int(1)]);
            }
            _ => panic!("reverse should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_empty_list_when_reverse_then_returns_empty_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(reverse '())").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- range --

    #[test]
    fn given_start_and_end_when_range_then_returns_integer_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(range 0 3)").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(0), Value::Int(1), Value::Int(2)]);
            }
            _ => panic!("range should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_equal_start_and_end_when_range_then_returns_empty_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(range 5 5)").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- map --

    #[test]
    fn given_lambda_and_list_when_map_then_returns_transformed_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(map (lambda (x) (+ x 1)) '(1 2 3))").unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(2), Value::Int(3), Value::Int(4)]);
            }
            _ => panic!("map should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_lambda_and_empty_list_when_map_then_returns_empty_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(map (lambda (x) (+ x 1)) '())").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- filter --

    #[test]
    fn given_predicate_and_list_when_filter_then_returns_matching_elements() {
        let runtime = runtime_with_list_primitives();
        let result = runtime
            .eval("(filter (lambda (x) (> x 2)) '(1 2 3 4))")
            .unwrap();
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items, vec![Value::Int(3), Value::Int(4)]);
            }
            _ => panic!("filter should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_predicate_matching_nothing_when_filter_then_returns_empty_list() {
        let runtime = runtime_with_list_primitives();
        let result = runtime
            .eval("(filter (lambda (x) (> x 100)) '(1 2 3))")
            .unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- reduce --

    #[test]
    fn given_plus_and_list_when_reduce_then_returns_sum() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(reduce + 0 '(1 2 3))").unwrap();
        assert_eq!(result.as_integer(), Some(6));
    }

    #[test]
    fn given_empty_list_when_reduce_then_returns_initial_value() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(reduce + 0 '())").unwrap();
        assert_eq!(result.as_integer(), Some(0));
    }

    // -- for-each --

    #[test]
    fn given_lambda_and_list_when_for_each_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        // for-each should execute without error and return NIL
        let result = runtime
            .eval("(for-each (lambda (x) (+ x 1)) '(1 2 3))")
            .unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- type predicates --

    #[test]
    fn given_string_when_string_predicate_then_returns_true() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(string? \"hello\")").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_integer_when_string_predicate_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(string? 42)").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    #[test]
    fn given_integer_when_number_predicate_then_returns_true() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(number? 42)").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_string_when_number_predicate_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(number? \"hello\")").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    #[test]
    fn given_list_when_list_predicate_then_returns_true() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(list? '(1 2))").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_integer_when_list_predicate_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(list? 42)").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    #[test]
    fn given_nil_when_nil_predicate_then_returns_true() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(nil? '())").unwrap();
        assert_eq!(*result.inner(), Value::True);
    }

    #[test]
    fn given_non_nil_when_nil_predicate_then_returns_nil() {
        let runtime = runtime_with_list_primitives();
        let result = runtime.eval("(nil? 42)").unwrap();
        assert_eq!(*result.inner(), Value::NIL);
    }

    // -- buffer-line-count (already registered via rainbow_csv_primitives) --

    #[test]
    fn given_multiline_buffer_when_buffer_line_count_then_returns_correct_count() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("Line 1\nLine 2\nLine 3");
        }

        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_buffer_style_primitives(&runtime, state.clone());

        let result = runtime.eval("(buffer-line-count)").unwrap();
        assert_eq!(result.as_integer(), Some(3));
    }

    // -- composition: map + filter + reduce pipeline --

    #[test]
    fn given_list_when_map_filter_reduce_composed_then_returns_correct_result() {
        let runtime = runtime_with_list_primitives();
        // (reduce + 0 (filter (lambda (x) (> x 2)) (map (lambda (x) (+ x 1)) '(1 2 3 4))))
        // map +1: (2 3 4 5), filter >2: (3 4 5), reduce +: 12
        let result = runtime
            .eval(
                "(reduce + 0 (filter (lambda (x) (> x 2)) (map (lambda (x) (+ x 1)) '(1 2 3 4))))",
            )
            .unwrap();
        assert_eq!(result.as_integer(), Some(12));
    }

    // -----------------------------------------------------------------------
    // Integration test: load actual rainbow-csv plugin file and execute command
    // -----------------------------------------------------------------------

    #[test]
    fn given_csv_buffer_when_rainbow_csv_plugin_loaded_and_command_executed_then_line_styles_populated(
    ) {
        // Given: an editor state with a CSV buffer
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("a,b,c\n1,2,3");
        }

        // And: a runtime with ALL primitives registered
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_define_command(&runtime, state.clone());
        register_string_primitives(&runtime);
        register_list_primitives(&runtime);
        register_hook_primitives(&runtime, state.clone());
        register_keymap_primitives(&runtime, state.clone());
        register_theme_primitives(&runtime, state.clone());
        register_buffer_style_primitives(&runtime, state.clone());

        // And: the actual rainbow-csv plugin is loaded from disk
        let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/rainbow-csv/init.lisp");
        runtime
            .eval_file(&plugin_path)
            .expect("rainbow-csv plugin should load without errors");

        // When: the "rainbow-csv" command is executed
        execute_dynamic_command(&state, "rainbow-csv")
            .expect("rainbow-csv command should execute without errors");

        // Then: line_styles has entries for the CSV lines
        let editor = state.borrow();
        assert!(
            !editor.line_styles.is_empty(),
            "line_styles should have entries after rainbow-csv command, but was empty"
        );
        // Specifically, both lines should have style entries
        assert!(
            editor.line_styles.contains_key(&0),
            "line 0 should have style entries"
        );
        assert!(
            editor.line_styles.contains_key(&1),
            "line 1 should have style entries"
        );
        // Each line has 3 CSV fields, so 3 style segments per line
        assert_eq!(
            editor.line_styles.get(&0).unwrap().len(),
            3,
            "line 0 should have 3 style segments (one per CSV field)"
        );
        assert_eq!(
            editor.line_styles.get(&1).unwrap().len(),
            3,
            "line 1 should have 3 style segments (one per CSV field)"
        );
    }

    // -----------------------------------------------------------------------
    // Rendering primitives: set-status-bar, set-gutter-line, set-gutter-width,
    // viewport-top-line, viewport-height
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    fn create_rendering_test_runtime() -> (LispRuntime, Rc<RefCell<EditorState>>) {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer =
                alfred_core::buffer::Buffer::from_string("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");
            editor.cursor = cursor::new(0, 0);
            editor.viewport.top_line = 3;
        }
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_rendering_primitives(&runtime, state.clone());
        (runtime, state)
    }

    #[test]
    fn given_viewport_at_line_3_when_viewport_top_line_evaluated_then_returns_3() {
        let (runtime, _state) = create_rendering_test_runtime();

        let result = runtime.eval("(viewport-top-line)").unwrap();
        assert_eq!(result.as_integer(), Some(3));
    }

    #[test]
    fn given_viewport_height_24_when_viewport_height_evaluated_then_returns_24() {
        let (runtime, _state) = create_rendering_test_runtime();

        let result = runtime.eval("(viewport-height)").unwrap();
        assert_eq!(result.as_integer(), Some(24));
    }

    // -----------------------------------------------------------------------
    // Panel primitives: define-panel, set-panel-content, set-panel-line,
    // set-panel-style, set-panel-size, remove-panel
    // Test Budget: 7 behaviors x 2 = 14 max
    // -----------------------------------------------------------------------

    fn create_panel_test_runtime() -> (LispRuntime, Rc<RefCell<EditorState>>) {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.viewport.top_line = 5;
        }
        let runtime = LispRuntime::new();
        register_core_primitives(&runtime, state.clone());
        register_panel_primitives(&runtime, state.clone());
        register_panel_focus_primitives(&runtime, state.clone());
        register_rendering_primitives(&runtime, state.clone());
        (runtime, state)
    }

    #[test]
    fn given_empty_registry_when_define_panel_evaluated_then_panel_exists() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"status\" \"bottom\" 1)")
            .unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "status");
        assert!(
            panel.is_some(),
            "Panel 'status' should exist after define-panel"
        );
        let panel = panel.unwrap();
        assert_eq!(panel.position, alfred_core::panel::PanelPosition::Bottom);
        assert_eq!(panel.size, 1);
    }

    #[test]
    fn given_panel_when_set_panel_content_evaluated_then_content_updated() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"status\" \"bottom\" 1)")
            .unwrap();
        runtime
            .eval("(set-panel-content \"status\" \" file.txt  Ln 1, Col 1 \")")
            .unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "status").unwrap();
        assert_eq!(panel.content, " file.txt  Ln 1, Col 1 ");
    }

    #[test]
    fn given_panel_when_set_panel_line_evaluated_then_line_content_stored() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"gutter\" \"left\" 4)")
            .unwrap();
        runtime
            .eval("(set-panel-line \"gutter\" 0 \"  1 \")")
            .unwrap();
        runtime
            .eval("(set-panel-line \"gutter\" 1 \"  2 \")")
            .unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "gutter").unwrap();
        assert_eq!(panel.lines.get(&0), Some(&"  1 ".to_string()));
        assert_eq!(panel.lines.get(&1), Some(&"  2 ".to_string()));
    }

    #[test]
    fn given_panel_when_set_panel_style_evaluated_then_colors_updated() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"status\" \"bottom\" 1)")
            .unwrap();
        runtime
            .eval("(set-panel-style \"status\" \"#cdd6f4\" \"#313244\")")
            .unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "status").unwrap();
        assert_eq!(panel.fg_color, Some("#cdd6f4".to_string()));
        assert_eq!(panel.bg_color, Some("#313244".to_string()));
    }

    #[test]
    fn given_panel_when_set_panel_style_with_default_then_colors_cleared() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"status\" \"bottom\" 1)")
            .unwrap();
        runtime
            .eval("(set-panel-style \"status\" \"#cdd6f4\" \"#313244\")")
            .unwrap();
        runtime
            .eval("(set-panel-style \"status\" \"default\" \"default\")")
            .unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "status").unwrap();
        assert_eq!(panel.fg_color, None);
        assert_eq!(panel.bg_color, None);
    }

    #[test]
    fn given_panel_when_remove_panel_evaluated_then_panel_gone() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"status\" \"bottom\" 1)")
            .unwrap();
        assert!(
            alfred_core::panel::get(&state.borrow().panels, "status").is_some(),
            "Panel should exist before removal"
        );

        runtime.eval("(remove-panel \"status\")").unwrap();

        let editor = state.borrow();
        assert!(
            alfred_core::panel::get(&editor.panels, "status").is_none(),
            "Panel should be gone after remove-panel"
        );
    }

    #[test]
    fn given_panel_when_set_panel_size_evaluated_then_size_updated() {
        let (runtime, state) = create_panel_test_runtime();

        runtime
            .eval("(define-panel \"gutter\" \"left\" 4)")
            .unwrap();
        runtime.eval("(set-panel-size \"gutter\" 6)").unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "gutter").unwrap();
        assert_eq!(panel.size, 6);
    }

    // -----------------------------------------------------------------------
    // Panel focus primitives
    // -----------------------------------------------------------------------

    #[test]
    fn given_panel_when_focus_panel_then_focused_panel_set() {
        let (runtime, state) = create_panel_test_runtime();
        runtime
            .eval("(define-panel \"sidebar\" \"left\" 20)")
            .unwrap();

        runtime.eval("(focus-panel \"sidebar\")").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.focused_panel, Some("sidebar".to_string()));
    }

    #[test]
    fn given_nonexistent_panel_when_focus_panel_then_error() {
        let (runtime, _state) = create_panel_test_runtime();

        let result = runtime.eval("(focus-panel \"nope\")");
        assert!(
            result.is_err(),
            "focus-panel on nonexistent panel should fail"
        );
    }

    #[test]
    fn given_focused_panel_when_unfocus_panel_then_focused_panel_cleared() {
        let (runtime, state) = create_panel_test_runtime();
        runtime
            .eval("(define-panel \"sidebar\" \"left\" 20)")
            .unwrap();
        runtime.eval("(focus-panel \"sidebar\")").unwrap();

        runtime.eval("(unfocus-panel)").unwrap();

        let editor = state.borrow();
        assert_eq!(editor.focused_panel, None);
    }

    #[test]
    fn given_panel_with_lines_when_panel_cursor_down_then_cursor_advances() {
        let (runtime, state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();

        runtime.eval("(panel-cursor-down \"tree\")").unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "tree").unwrap();
        assert_eq!(panel.cursor_line, 1);
    }

    #[test]
    fn given_panel_with_cursor_at_1_when_panel_cursor_up_then_cursor_at_0() {
        let (runtime, state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();
        runtime.eval("(panel-cursor-down \"tree\")").unwrap();

        runtime.eval("(panel-cursor-up \"tree\")").unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "tree").unwrap();
        assert_eq!(panel.cursor_line, 0);
    }

    #[test]
    fn given_panel_with_lines_when_panel_cursor_line_then_returns_position() {
        let (runtime, _state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();
        runtime.eval("(panel-cursor-down \"tree\")").unwrap();

        let result = runtime.eval("(panel-cursor-line \"tree\")").unwrap();
        assert_eq!(result.as_integer(), Some(1));
    }

    #[test]
    fn given_panel_with_3_lines_when_panel_entry_count_then_returns_3() {
        let (runtime, _state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 2 \"c\")").unwrap();

        let result = runtime.eval("(panel-entry-count \"tree\")").unwrap();
        assert_eq!(result.as_integer(), Some(3));
    }

    #[test]
    fn given_viewport_at_line_5_when_viewport_top_line_evaluated_via_panel_runtime_then_returns_5()
    {
        let (runtime, _state) = create_panel_test_runtime();

        let result = runtime.eval("(viewport-top-line)").unwrap();
        assert_eq!(result.as_integer(), Some(5));
    }

    #[test]
    fn given_viewport_height_24_when_viewport_height_evaluated_via_panel_runtime_then_returns_24() {
        let (runtime, _state) = create_panel_test_runtime();

        let result = runtime.eval("(viewport-height)").unwrap();
        assert_eq!(result.as_integer(), Some(24));
    }

    // -----------------------------------------------------------------------
    // clear-panel-lines primitive
    // -----------------------------------------------------------------------

    #[test]
    fn given_panel_with_lines_when_clear_panel_lines_then_lines_empty() {
        let (runtime, state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();

        runtime.eval("(clear-panel-lines \"tree\")").unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "tree").unwrap();
        assert!(panel.lines.is_empty());
        assert_eq!(panel.cursor_line, 0);
    }

    #[test]
    fn given_nonexistent_panel_when_clear_panel_lines_then_error() {
        let (runtime, _state) = create_panel_test_runtime();
        let result = runtime.eval("(clear-panel-lines \"nope\")");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // panel-set-cursor primitive
    // -----------------------------------------------------------------------

    #[test]
    fn given_panel_when_panel_set_cursor_then_cursor_at_target() {
        let (runtime, state) = create_panel_test_runtime();
        runtime.eval("(define-panel \"tree\" \"left\" 20)").unwrap();
        runtime.eval("(set-panel-line \"tree\" 0 \"a\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 1 \"b\")").unwrap();
        runtime.eval("(set-panel-line \"tree\" 2 \"c\")").unwrap();

        runtime.eval("(panel-set-cursor \"tree\" 2)").unwrap();

        let editor = state.borrow();
        let panel = alfred_core::panel::get(&editor.panels, "tree").unwrap();
        assert_eq!(panel.cursor_line, 2);
    }

    #[test]
    fn given_nonexistent_panel_when_panel_set_cursor_then_error() {
        let (runtime, _state) = create_panel_test_runtime();
        let result = runtime.eval("(panel-set-cursor \"nope\" 0)");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: list-dir
    // -----------------------------------------------------------------------

    #[test]
    fn given_directory_with_files_and_dirs_when_list_dir_then_returns_sorted_entries_dirs_first() {
        let dir = tempfile::tempdir().unwrap();
        // Create files and directories
        std::fs::write(dir.path().join("banana.txt"), "content").unwrap();
        std::fs::write(dir.path().join("apple.txt"), "content").unwrap();
        std::fs::create_dir(dir.path().join("zulu_dir")).unwrap();
        std::fs::create_dir(dir.path().join("alpha_dir")).unwrap();

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let dir_path = dir.path().to_string_lossy().to_string();
        let result = runtime
            .eval(&format!("(list-dir \"{}\")", dir_path))
            .unwrap();

        // Parse result: should be ((alpha_dir dir) (zulu_dir dir) (apple.txt file) (banana.txt file))
        let inner = result.inner().clone();
        match inner {
            Value::List(list) => {
                let items: Vec<Value> = list.into_iter().collect();
                assert_eq!(items.len(), 4, "Should have 4 entries");

                // First two should be directories (alphabetical)
                let first_pair: Vec<Value> = match &items[0] {
                    Value::List(l) => l.clone().into_iter().collect(),
                    other => panic!("Expected list, got {:?}", other),
                };
                assert_eq!(first_pair[0], Value::String("alpha_dir".to_string()));
                assert_eq!(first_pair[1], Value::String("dir".to_string()));

                let second_pair: Vec<Value> = match &items[1] {
                    Value::List(l) => l.clone().into_iter().collect(),
                    other => panic!("Expected list, got {:?}", other),
                };
                assert_eq!(second_pair[0], Value::String("zulu_dir".to_string()));
                assert_eq!(second_pair[1], Value::String("dir".to_string()));

                // Last two should be files (alphabetical)
                let third_pair: Vec<Value> = match &items[2] {
                    Value::List(l) => l.clone().into_iter().collect(),
                    other => panic!("Expected list, got {:?}", other),
                };
                assert_eq!(third_pair[0], Value::String("apple.txt".to_string()));
                assert_eq!(third_pair[1], Value::String("file".to_string()));

                let fourth_pair: Vec<Value> = match &items[3] {
                    Value::List(l) => l.clone().into_iter().collect(),
                    other => panic!("Expected list, got {:?}", other),
                };
                assert_eq!(fourth_pair[0], Value::String("banana.txt".to_string()));
                assert_eq!(fourth_pair[1], Value::String("file".to_string()));
            }
            _ => panic!("list-dir should return a list, got {:?}", inner),
        }
    }

    #[test]
    fn given_nonexistent_path_when_list_dir_then_returns_empty_list() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let result = runtime
            .eval("(list-dir \"/nonexistent/path/that/does/not/exist\")")
            .unwrap();

        let inner = result.inner().clone();
        match inner {
            Value::List(List::NIL) => {} // expected empty list
            _ => panic!(
                "list-dir on nonexistent path should return empty list, got {:?}",
                inner
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: is-dir?
    // -----------------------------------------------------------------------

    #[test]
    fn given_directory_when_is_dir_then_returns_true() {
        let dir = tempfile::tempdir().unwrap();

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let dir_path = dir.path().to_string_lossy().to_string();
        let result = runtime
            .eval(&format!("(is-dir? \"{}\")", dir_path))
            .unwrap();

        assert_eq!(result.inner().clone(), Value::True);
    }

    #[test]
    fn given_file_when_is_dir_then_returns_nil() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let path_str = file_path.to_string_lossy().to_string();
        let result = runtime
            .eval(&format!("(is-dir? \"{}\")", path_str))
            .unwrap();

        let inner = result.inner().clone();
        match inner {
            Value::List(List::NIL) => {} // NIL = false
            _ => panic!("is-dir? on a file should return NIL, got {:?}", inner),
        }
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: path-join
    // -----------------------------------------------------------------------

    #[test]
    fn given_base_and_child_when_path_join_then_returns_joined_path() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let result = runtime.eval("(path-join \"/home\" \"user\")").unwrap();

        assert_eq!(result.as_string(), Some("/home/user".to_string()));
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: path-parent
    // -----------------------------------------------------------------------

    #[test]
    fn given_path_when_path_parent_then_returns_parent_directory() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let result = runtime.eval("(path-parent \"/home/user\")").unwrap();

        assert_eq!(result.as_string(), Some("/home".to_string()));
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: cli-argument
    // -----------------------------------------------------------------------

    #[test]
    fn given_cli_argument_set_when_cli_argument_evaluated_then_returns_argument() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        state.borrow_mut().cli_argument = Some("/tmp/myfile.txt".to_string());

        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let result = runtime.eval("(cli-argument)").unwrap();

        assert_eq!(result.as_string(), Some("/tmp/myfile.txt".to_string()));
    }

    #[test]
    fn given_no_cli_argument_when_cli_argument_evaluated_then_returns_empty_string() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        // cli_argument is None by default

        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let result = runtime.eval("(cli-argument)").unwrap();

        assert_eq!(result.as_string(), Some("".to_string()));
    }

    // -----------------------------------------------------------------------
    // Filesystem primitives: open-file
    // -----------------------------------------------------------------------

    #[test]
    fn given_valid_file_when_open_file_then_buffer_contains_file_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello from file").unwrap();

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        // Start with some content in the buffer to verify it gets replaced
        {
            let mut editor = state.borrow_mut();
            editor.buffer = alfred_core::buffer::Buffer::from_string("original content");
            editor.cursor = cursor::new(5, 10);
        }

        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        let path_str = file_path.to_string_lossy().to_string();
        runtime
            .eval(&format!("(open-file \"{}\")", path_str))
            .unwrap();

        let editor = state.borrow();
        assert_eq!(buffer::content(&editor.buffer), "Hello from file");
        assert_eq!(editor.cursor.line, 0);
        assert_eq!(editor.cursor.column, 0);
        assert_eq!(editor.viewport.top_line, 0);
        assert_eq!(editor.mode, "normal");
        assert_eq!(editor.active_keymaps, vec!["normal-mode".to_string()]);
        assert_eq!(editor.message, Some("test.txt".to_string()));
    }

    #[test]
    fn given_nonexistent_file_when_open_file_then_message_contains_error() {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));

        let runtime = LispRuntime::new();
        register_filesystem_primitives(&runtime, state.clone());

        runtime
            .eval("(open-file \"/nonexistent/path/to/file.txt\")")
            .unwrap();

        let editor = state.borrow();
        assert!(
            editor.message.is_some(),
            "Message should be set with error text"
        );
        let msg = editor.message.as_ref().unwrap();
        assert!(
            msg.contains("nonexistent") || msg.contains("No such file") || msg.contains("error"),
            "Error message should mention the problem, got: {}",
            msg
        );
    }
}
