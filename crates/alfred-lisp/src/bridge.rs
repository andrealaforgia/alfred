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
pub fn register_core_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_buffer_insert(env.clone(), state.clone());
    register_buffer_delete(env.clone(), state.clone());
    register_buffer_content(env.clone(), state.clone());
    register_cursor_position(env.clone(), state.clone());
    register_cursor_move(env.clone(), state.clone());
    register_message(env.clone(), state.clone());
    register_current_mode(env, state);
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
}
