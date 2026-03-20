//! M1 Acceptance Tests: Rust Kernel -- Buffer, Cursor, Viewport, Navigation
//!
//! What M1 proves: Can display a file and navigate it with arrow keys.
//!
//! Driving ports exercised:
//!   - EditorState (buffer content, cursor position, viewport, running flag)
//!   - Buffer (file loading, content access, line count)
//!   - Cursor (position, movement within buffer bounds)
//!   - Viewport (scroll tracking)
//!
//! These tests do NOT touch the terminal. They verify state transitions
//! through the public API of alfred-core.

mod helpers;

// ---------------------------------------------------------------------------
// Walking Skeleton
// ---------------------------------------------------------------------------

/// WS-1: The simplest end-to-end proof that Alfred boots and does something.
///
/// Given a file exists with known text content
/// When the user opens the file in Alfred
/// Then the buffer contains the file's full text
/// And the cursor is positioned at the beginning of the file
#[test]
#[ignore] // Enable first -- this is the walking skeleton
fn given_a_file_with_content_when_opened_then_buffer_contains_text_and_cursor_at_start() {
    // Given: a file exists with known text content
    let content = "Line one\nLine two\nLine three\n";
    let (_dir, file_path) = helpers::create_temp_file("test.txt", content);

    // When: the user opens the file in Alfred
    // Will be: let state = EditorState::open_file(&file_path).unwrap();
    let _ = file_path;

    // Then: the buffer contains the file's full text
    // assert_eq!(state.buffer.content(), content);

    // And: the cursor is positioned at the beginning of the file
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when alfred-core crate exists");
}

// ---------------------------------------------------------------------------
// Happy Path
// ---------------------------------------------------------------------------

/// M1-H1: Cursor moves down one line.
///
/// Given a file is open with multiple lines
/// When the user moves the cursor down
/// Then the cursor advances to the next line
#[test]
#[ignore]
fn given_file_open_when_cursor_moves_down_then_cursor_on_next_line() {
    // Given: a file is open with multiple lines
    let content = "Line one\nLine two\nLine three\n";
    let (_dir, file_path) = helpers::create_temp_file("test.txt", content);
    let _ = file_path;

    // When: the user moves the cursor down
    // state = cursor::move_cursor(state.cursor, Direction::Down, 1, &state.buffer);

    // Then: the cursor advances to the next line
    // assert_eq!(state.cursor.line, 1);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when alfred-core crate exists");
}

/// M1-H2: Viewport scrolls when cursor exits visible area.
///
/// Given the cursor is at the bottom of the viewport
/// When the cursor moves past the viewport boundary
/// Then the viewport scrolls to keep the cursor visible
#[test]
#[ignore]
fn given_cursor_at_viewport_bottom_when_moves_down_then_viewport_scrolls() {
    // Given: a file with more lines than the viewport height
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {}", i)).collect();
    let content = lines.join("\n");
    let (_dir, file_path) = helpers::create_temp_file("long.txt", &content);
    let _ = file_path;

    // When: the cursor moves past the viewport boundary
    // Set viewport height to 24, move cursor to line 25
    // for _ in 0..25 {
    //     state.cursor = cursor::move_cursor(state.cursor, Direction::Down, 1, &state.buffer);
    // }

    // Then: the viewport scrolls to keep the cursor visible
    // assert!(state.viewport.top_line > 0);
    // assert!(state.cursor.line >= state.viewport.top_line);
    // assert!(state.cursor.line < state.viewport.top_line + state.viewport.height as usize);

    todo!("Implement when alfred-core crate exists");
}

/// M1-H3: Editor exits cleanly on quit.
///
/// Given the user is viewing a file
/// When the user presses the quit key combination (Ctrl-Q)
/// Then the editor signals quit
#[test]
#[ignore]
fn given_file_open_when_quit_pressed_then_editor_signals_quit() {
    // Given: the user is viewing a file
    let (_dir, file_path) = helpers::create_temp_file("test.txt", "Hello");
    let _ = file_path;

    // When: the user presses Ctrl-Q
    // Process KeyEvent { code: KeyCode::Char('q'), modifiers: Modifiers::CTRL }

    // Then: the editor signals quit
    // assert!(!state.running);

    todo!("Implement when alfred-core crate exists");
}

/// M1-H4: Cursor moves right within a line.
///
/// Given a file is open with the cursor at column 0
/// When the user moves the cursor right
/// Then the cursor advances one column
#[test]
#[ignore]
fn given_cursor_at_column_zero_when_moves_right_then_cursor_advances_one_column() {
    // Given: a file is open with cursor at column 0
    let (_dir, file_path) = helpers::create_temp_file("test.txt", "Hello World");
    let _ = file_path;

    // When: the user moves the cursor right
    // state.cursor = cursor::move_cursor(state.cursor, Direction::Right, 1, &state.buffer);

    // Then: the cursor advances one column
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 1);

    todo!("Implement when alfred-core crate exists");
}

// ---------------------------------------------------------------------------
// Error Path
// ---------------------------------------------------------------------------

/// M1-E1: Cursor at last line does not move further down.
///
/// Given the cursor is at the last line of the buffer
/// When the user moves down
/// Then the cursor remains on the last line
#[test]
#[ignore]
fn given_cursor_at_last_line_when_moves_down_then_cursor_stays_on_last_line() {
    // Given: a file with 3 lines, cursor on line 2 (zero-indexed)
    let content = "Line one\nLine two\nLine three";
    let (_dir, file_path) = helpers::create_temp_file("test.txt", content);
    let _ = file_path;

    // Move cursor to last line
    // state.cursor = Cursor { line: 2, column: 0 };

    // When: the user moves down
    // state.cursor = cursor::move_cursor(state.cursor, Direction::Down, 1, &state.buffer);

    // Then: the cursor remains on line 2
    // assert_eq!(state.cursor.line, 2);

    todo!("Implement when alfred-core crate exists");
}

/// M1-E2: Cursor at line start does not move further left.
///
/// Given the cursor is at the start of a line
/// When the user moves left
/// Then the cursor remains at line start
#[test]
#[ignore]
fn given_cursor_at_line_start_when_moves_left_then_cursor_stays_at_start() {
    // Given: cursor at column 0
    let (_dir, file_path) = helpers::create_temp_file("test.txt", "Hello");
    let _ = file_path;

    // When: the user moves left
    // state.cursor = cursor::move_cursor(state.cursor, Direction::Left, 1, &state.buffer);

    // Then: cursor stays at column 0
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when alfred-core crate exists");
}

/// M1-E3: Cursor at line end does not move further right.
///
/// Given the cursor is at the end of a line
/// When the user moves right
/// Then the cursor remains at line end
#[test]
#[ignore]
fn given_cursor_at_line_end_when_moves_right_then_cursor_stays_at_end() {
    // Given: file with "Hello" (5 chars), cursor at column 5
    let (_dir, file_path) = helpers::create_temp_file("test.txt", "Hello");
    let _ = file_path;

    // Set cursor to end of line
    // state.cursor = Cursor { line: 0, column: 5 };

    // When: the user moves right
    // state.cursor = cursor::move_cursor(state.cursor, Direction::Right, 1, &state.buffer);

    // Then: cursor stays at column 5
    // assert_eq!(state.cursor.column, 5);

    todo!("Implement when alfred-core crate exists");
}

/// M1-E4: Editor starts with empty buffer when no file is provided.
///
/// Given no file argument is provided
/// When the editor starts
/// Then the editor opens with an empty buffer
#[test]
#[ignore]
fn given_no_file_argument_when_editor_starts_then_empty_buffer() {
    // Given: no file argument

    // When: the editor starts
    // let state = EditorState::new();

    // Then: the buffer is empty
    // assert_eq!(state.buffer.line_count(), 0);
    // OR assert_eq!(state.buffer.content(), "");
    // And cursor is at (0, 0)
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when alfred-core crate exists");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// M1-EC1: Empty file results in buffer with zero content.
///
/// Given an empty file
/// When opened
/// Then the buffer has zero lines of content and cursor is at (0, 0)
#[test]
#[ignore]
fn given_empty_file_when_opened_then_buffer_empty_and_cursor_at_origin() {
    // Given: an empty file
    let (_dir, file_path) = helpers::create_temp_file("empty.txt", "");
    let _ = file_path;

    // When: opened
    // let state = EditorState::open_file(&file_path).unwrap();

    // Then: buffer is empty
    // assert_eq!(state.buffer.content(), "");
    // assert_eq!(state.cursor.line, 0);
    // assert_eq!(state.cursor.column, 0);

    todo!("Implement when alfred-core crate exists");
}

/// M1-EC2: Long line is fully loaded into the buffer.
///
/// Given a file with one very long line (exceeding viewport width)
/// When opened
/// Then the buffer contains the full line content
#[test]
#[ignore]
fn given_file_with_long_line_when_opened_then_buffer_contains_full_line() {
    // Given: a file with a very long line
    let long_line = "x".repeat(500);
    let (_dir, file_path) = helpers::create_temp_file("long_line.txt", &long_line);
    let _ = file_path;

    // When: opened
    // let state = EditorState::open_file(&file_path).unwrap();

    // Then: buffer contains the full line
    // assert_eq!(state.buffer.get_line(0).unwrap(), &long_line);

    todo!("Implement when alfred-core crate exists");
}

/// M1-EC3: Cursor column clamps when moving to a shorter line.
///
/// Given the cursor is on a long line at a high column
/// When the cursor moves to a shorter line
/// Then the cursor column clamps to the shorter line's length
#[test]
#[ignore]
fn given_cursor_on_long_line_when_moves_to_short_line_then_column_clamps() {
    // Given: two lines, first is long, second is short
    let content = "Hello World (long line)\nHi";
    let (_dir, file_path) = helpers::create_temp_file("test.txt", content);
    let _ = file_path;

    // Place cursor at column 20 on line 0
    // state.cursor = Cursor { line: 0, column: 20 };

    // When: cursor moves down to line 1 (which is only 2 chars)
    // state.cursor = cursor::move_cursor(state.cursor, Direction::Down, 1, &state.buffer);

    // Then: column clamps to line 1's length
    // assert_eq!(state.cursor.line, 1);
    // assert_eq!(state.cursor.column, 2); // "Hi" is 2 chars

    todo!("Implement when alfred-core crate exists");
}
