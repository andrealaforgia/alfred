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
use alfred_core::editor_state::EditorState;

use crate::runtime::LispRuntime;

/// Registers all core buffer and cursor primitives into the runtime.
///
/// After calling this, the following Lisp functions become available:
/// - `(buffer-insert text)` -- insert text at cursor position
/// - `(buffer-delete)` -- delete character at cursor position
/// - `(buffer-content)` -- return buffer text as string
/// - `(cursor-position)` -- return (line column) as a list
pub fn register_core_primitives(runtime: &LispRuntime, state: Rc<RefCell<EditorState>>) {
    let env = runtime.env();

    register_buffer_insert(env.clone(), state.clone());
    register_buffer_delete(env.clone(), state.clone());
    register_buffer_content(env.clone(), state.clone());
    register_cursor_position(env, state);
}

/// Registers `buffer-insert`: inserts text at the current cursor position.
fn register_buffer_insert(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    let closure = move |_env: Rc<RefCell<Env>>, args: Vec<Value>| -> Result<Value, RuntimeError> {
        let text = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(RuntimeError {
                    msg: format!("buffer-insert: expected string argument, got {}", other),
                });
            }
            None => {
                return Err(RuntimeError {
                    msg: "buffer-insert: expected 1 argument, got 0".to_string(),
                });
            }
        };

        let mut editor = state.borrow_mut();
        let cursor_line = editor.cursor.line;
        let cursor_column = editor.cursor.column;
        editor.buffer = buffer::insert_at(&editor.buffer, cursor_line, cursor_column, &text);

        Ok(Value::NIL)
    };

    env.borrow_mut().define(
        Symbol("buffer-insert".to_string()),
        Value::NativeClosure(Rc::new(RefCell::new(closure))),
    );
}

/// Registers `buffer-delete`: removes one character at the cursor position.
fn register_buffer_delete(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    let closure = move |_env: Rc<RefCell<Env>>, _args: Vec<Value>| -> Result<Value, RuntimeError> {
        let mut editor = state.borrow_mut();
        let cursor_line = editor.cursor.line;
        let cursor_column = editor.cursor.column;
        editor.buffer = buffer::delete_at(&editor.buffer, cursor_line, cursor_column);

        Ok(Value::NIL)
    };

    env.borrow_mut().define(
        Symbol("buffer-delete".to_string()),
        Value::NativeClosure(Rc::new(RefCell::new(closure))),
    );
}

/// Registers `buffer-content`: returns the entire buffer text as a string.
fn register_buffer_content(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    let closure = move |_env: Rc<RefCell<Env>>, _args: Vec<Value>| -> Result<Value, RuntimeError> {
        let editor = state.borrow();
        let text = buffer::content(&editor.buffer);
        Ok(Value::String(text))
    };

    env.borrow_mut().define(
        Symbol("buffer-content".to_string()),
        Value::NativeClosure(Rc::new(RefCell::new(closure))),
    );
}

/// Registers `cursor-position`: returns the cursor's (line column) as a list.
fn register_cursor_position(env: Rc<RefCell<Env>>, state: Rc<RefCell<EditorState>>) {
    let closure = move |_env: Rc<RefCell<Env>>, _args: Vec<Value>| -> Result<Value, RuntimeError> {
        let editor = state.borrow();
        let line = editor.cursor.line as i32;
        let column = editor.cursor.column as i32;
        let list: List = vec![Value::Int(line), Value::Int(column)]
            .into_iter()
            .collect();
        Ok(Value::List(list))
    };

    env.borrow_mut().define(
        Symbol("cursor-position".to_string()),
        Value::NativeClosure(Rc::new(RefCell::new(closure))),
    );
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
}
