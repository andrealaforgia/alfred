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
/// - `(buffer-filename)` -- return the buffer's filename or empty string if unnamed
/// - `(buffer-modified?)` -- return T if buffer modified, F otherwise
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
    register_buffer_modified(env, state);
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

/// Registers `current-mode`: returns the current editor mode name as a string.
///
/// Usage: `(current-mode)` -- returns `"normal"` (or other mode name).
fn register_current_mode(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    define_native_closure(&env, "current-mode", move |_env, _args| {
        let editor = state.borrow();
        let mode_name = editor.mode.to_string();
        Ok(Value::String(mode_name))
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
}
