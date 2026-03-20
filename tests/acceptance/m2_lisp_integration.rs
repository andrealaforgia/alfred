//! M2 Acceptance Tests: Lisp Integration -- Evaluate Expressions, Call Rust Primitives
//!
//! What M2 proves: Can evaluate Lisp expressions that call Rust primitives
//! and produce observable changes in editor state.
//!
//! Driving ports exercised:
//!   - LispRuntime::eval() — evaluate Lisp source strings
//!   - EditorState — observe buffer/cursor/message changes after evaluation
//!   - bridge::register_core_primitives() — set up FFI at startup
//!
//! Tests verify that Lisp evaluation produces correct state changes.
//! They do NOT test the Lisp parser or interpreter internals.

mod helpers;

// ---------------------------------------------------------------------------
// Walking Skeleton
// ---------------------------------------------------------------------------

/// WS-2: Lisp expression modifies the buffer through a Rust primitive.
///
/// Given the editor has a buffer with text "Hello"
/// When the expression (buffer-insert " World") is evaluated
/// Then the buffer content becomes "Hello World"
#[test]
#[ignore]
fn given_buffer_with_text_when_lisp_inserts_then_buffer_updated() {
    // Given: a buffer with text "Hello", cursor at end
    // let mut state = EditorState::new();
    // state.buffer = Buffer::from_str("Hello");
    // state.cursor = Cursor { line: 0, column: 5 };
    // let mut runtime = LispRuntime::new();
    // bridge::register_core_primitives(&mut runtime, &mut state).unwrap();

    // When: the expression (buffer-insert " World") is evaluated
    // runtime.eval("(buffer-insert \" World\")").unwrap();

    // Then: the buffer content becomes "Hello World"
    // assert_eq!(state.buffer.content(), "Hello World");

    todo!("Implement when alfred-lisp crate exists");
}

// ---------------------------------------------------------------------------
// Happy Path
// ---------------------------------------------------------------------------

/// M2-H1: Lisp expression moves the cursor.
///
/// Given a buffer with multiple lines
/// When (cursor-move :down 5) is evaluated
/// Then the cursor moves down five lines
#[test]
#[ignore]
fn given_multiline_buffer_when_lisp_moves_cursor_down_then_cursor_at_line_5() {
    // Given: a buffer with 10 lines, cursor at (0, 0)
    let lines: Vec<String> = (1..=10).map(|i| format!("Line {}", i)).collect();
    let content = lines.join("\n");
    let (_dir, _file_path) = helpers::create_temp_file("test.txt", &content);

    // When: (cursor-move :down 5) is evaluated
    // runtime.eval("(cursor-move :down 5)").unwrap();

    // Then: the cursor is at line 5
    // assert_eq!(state.cursor.line, 5);

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-H2: Lisp expression reads cursor position.
///
/// Given a buffer with cursor at a known position
/// When (cursor-position) is evaluated
/// Then it returns the current line and column
#[test]
#[ignore]
fn given_cursor_at_known_position_when_lisp_reads_position_then_returns_correct_values() {
    // Given: cursor at (3, 7)
    // state.cursor = Cursor { line: 3, column: 7 };

    // When: (cursor-position) is evaluated
    // let result = runtime.eval("(cursor-position)").unwrap();

    // Then: result contains line 3, column 7
    // The Lisp value should represent (3 7) or equivalent
    // assert_eq!(result, LispValue::list(vec![LispValue::Int(3), LispValue::Int(7)]));

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-H3: Lisp expression sets a message.
///
/// Given the editor is running
/// When (message "test") is evaluated
/// Then the message area contains "test"
#[test]
#[ignore]
fn given_editor_running_when_lisp_sets_message_then_message_area_shows_text() {
    // Given: editor state with empty message
    // state.message = None;

    // When: (message "test") is evaluated
    // runtime.eval("(message \"test\")").unwrap();

    // Then: the message area contains "test"
    // assert_eq!(state.message, Some("test".to_string()));

    todo!("Implement when alfred-lisp crate exists");
}

// ---------------------------------------------------------------------------
// Error Path
// ---------------------------------------------------------------------------

/// M2-E1: Syntax error in Lisp does not crash the editor.
///
/// Given the editor is running
/// When a Lisp expression with a syntax error is evaluated
/// Then an error message is displayed and the editor remains stable
#[test]
#[ignore]
fn given_editor_running_when_lisp_syntax_error_then_error_message_and_stable() {
    // Given: editor is running
    // let mut state = EditorState::new();
    // let mut runtime = LispRuntime::new();

    // When: expression with syntax error
    // let result = runtime.eval("(buffer-insert");

    // Then: error is returned, editor state intact
    // assert!(result.is_err());
    // assert!(state.running); // editor still running, not crashed

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-E2: Calling an undefined function shows an error.
///
/// Given the editor is running
/// When a Lisp expression calls an undefined function
/// Then an error message is displayed and the editor remains stable
#[test]
#[ignore]
fn given_editor_running_when_lisp_calls_undefined_function_then_error_and_stable() {
    // Given: editor is running

    // When: calling an undefined function
    // let result = runtime.eval("(nonexistent-function 42)");

    // Then: error returned, editor stable
    // assert!(result.is_err());
    // assert!(state.running);

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-E3: Wrong argument types to a primitive show an error.
///
/// Given the editor is running
/// When a primitive receives wrong argument types
/// Then an error message is displayed and the buffer is unchanged
#[test]
#[ignore]
fn given_editor_running_when_primitive_gets_wrong_types_then_error_and_buffer_unchanged() {
    // Given: buffer with known content
    // state.buffer = Buffer::from_str("Original");

    // When: cursor-move receives wrong types
    // let result = runtime.eval("(cursor-move \"not-a-direction\" \"not-a-number\")");

    // Then: error returned, buffer unchanged
    // assert!(result.is_err());
    // assert_eq!(state.buffer.content(), "Original");

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-E4: Too few arguments to a primitive show an error.
///
/// Given the editor is running
/// When a primitive receives too few arguments
/// Then an error message is displayed and the editor remains stable
#[test]
#[ignore]
fn given_editor_running_when_primitive_gets_too_few_args_then_error_and_stable() {
    // Given: editor is running

    // When: cursor-move called with no arguments
    // let result = runtime.eval("(cursor-move)");

    // Then: error returned, editor stable
    // assert!(result.is_err());
    // assert!(state.running);

    todo!("Implement when alfred-lisp crate exists");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// M2-EC1: Inserting into an empty buffer.
///
/// Given an empty buffer
/// When (buffer-insert "text") is evaluated
/// Then the buffer contains "text"
#[test]
#[ignore]
fn given_empty_buffer_when_lisp_inserts_text_then_buffer_contains_text() {
    // Given: empty buffer
    // state.buffer = Buffer::from_str("");

    // When: insert text
    // runtime.eval("(buffer-insert \"text\")").unwrap();

    // Then: buffer has content
    // assert_eq!(state.buffer.content(), "text");

    todo!("Implement when alfred-lisp crate exists");
}

/// M2-EC2: Line count primitive returns correct count.
///
/// Given a buffer with 5 lines
/// When (buffer-line-count) is evaluated
/// Then it returns 5
#[test]
#[ignore]
fn given_buffer_with_five_lines_when_lisp_counts_lines_then_returns_five() {
    // Given: buffer with 5 lines
    // state.buffer = Buffer::from_str("A\nB\nC\nD\nE");

    // When: line count queried
    // let result = runtime.eval("(buffer-line-count)").unwrap();

    // Then: returns 5
    // assert_eq!(result, LispValue::Int(5));

    todo!("Implement when alfred-lisp crate exists");
}
