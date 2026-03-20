//! Cursor: position within a buffer and pure movement functions.
//!
//! A Cursor represents a (line, column) position in a buffer. All movement
//! functions are pure: they take a Cursor and a &Buffer reference, returning
//! a new Cursor. Movement is always clamped to buffer boundaries.

use crate::buffer::{self, Buffer};

/// A position within a buffer, identified by zero-indexed line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub line: usize,
    pub column: usize,
}

/// Creates a new Cursor at the given line and column.
pub fn new(line: usize, column: usize) -> Cursor {
    Cursor { line, column }
}

/// Returns the display length of a line (character count excluding trailing newline).
fn line_length(buf: &Buffer, line_index: usize) -> usize {
    buffer::get_line(buf, line_index)
        .map(|line| line.trim_end_matches('\n').len())
        .unwrap_or(0)
}

/// Moves the cursor down by one line, clamping column to the target line length.
///
/// If already on the last line, the cursor stays unchanged.
pub fn move_down(cursor: Cursor, buf: &Buffer) -> Cursor {
    let total_lines = buffer::line_count(buf);
    let last_line = if total_lines > 0 { total_lines - 1 } else { 0 };
    let new_line = if cursor.line < last_line {
        cursor.line + 1
    } else {
        cursor.line
    };
    let max_column = line_length(buf, new_line);
    Cursor {
        line: new_line,
        column: cursor.column.min(max_column),
    }
}

/// Moves the cursor up by one line, clamping column to the target line length.
///
/// If already on the first line, the cursor stays unchanged.
pub fn move_up(cursor: Cursor, buf: &Buffer) -> Cursor {
    let new_line = cursor.line.saturating_sub(1);
    let max_column = line_length(buf, new_line);
    Cursor {
        line: new_line,
        column: cursor.column.min(max_column),
    }
}

/// Moves the cursor right by one column.
///
/// If at the end of a line and a next line exists, wraps to column 0 of the
/// next line. If at the end of the last line, the cursor stays unchanged.
pub fn move_right(cursor: Cursor, buf: &Buffer) -> Cursor {
    let current_line_len = line_length(buf, cursor.line);
    if cursor.column < current_line_len {
        Cursor {
            line: cursor.line,
            column: cursor.column + 1,
        }
    } else {
        let total_lines = buffer::line_count(buf);
        let last_line = if total_lines > 0 { total_lines - 1 } else { 0 };
        if cursor.line < last_line {
            Cursor {
                line: cursor.line + 1,
                column: 0,
            }
        } else {
            cursor
        }
    }
}

/// Moves the cursor left by one column.
///
/// If at the start of a line and a previous line exists, wraps to the end of
/// the previous line. If at the start of the first line, the cursor stays unchanged.
pub fn move_left(cursor: Cursor, buf: &Buffer) -> Cursor {
    if cursor.column > 0 {
        Cursor {
            line: cursor.line,
            column: cursor.column - 1,
        }
    } else if cursor.line > 0 {
        let prev_line = cursor.line - 1;
        let prev_line_len = line_length(buf, prev_line);
        Cursor {
            line: prev_line,
            column: prev_line_len,
        }
    } else {
        cursor
    }
}

/// Clamps a cursor so that it lies within the buffer boundaries.
///
/// If the line is beyond the buffer, clamps to the last line. If the column
/// is beyond the line length, clamps to the end of that line.
pub fn ensure_within_bounds(cursor: Cursor, buf: &Buffer) -> Cursor {
    let total_lines = buffer::line_count(buf);
    let last_line = if total_lines > 0 { total_lines - 1 } else { 0 };
    let clamped_line = cursor.line.min(last_line);
    let max_column = line_length(buf, clamped_line);
    Cursor {
        line: clamped_line,
        column: cursor.column.min(max_column),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Acceptance test: cursor moves correctly across a multi-line buffer
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_cursor_moves_then_position_updates_and_clamps_at_boundaries() {
        // Given: a buffer with 3 lines of varying length
        //   line 0: "Hello"       (5 chars)
        //   line 1: "World!"      (6 chars)
        //   line 2: "Hi"          (2 chars)
        let buf = Buffer::from_string("Hello\nWorld!\nHi");

        // Starting at (0, 0)
        let cursor = new(0, 0);

        // When: move_down from line 0
        let cursor = move_down(cursor, &buf);
        // Then: cursor is at line 1, column 0
        assert_eq!(cursor, new(1, 0));

        // When: move_right 6 times (to end of "World!")
        let cursor = move_right(
            move_right(
                move_right(
                    move_right(move_right(move_right(cursor, &buf), &buf), &buf),
                    &buf,
                ),
                &buf,
            ),
            &buf,
        );
        // Then: cursor is at (1, 6) -- end of "World!"
        assert_eq!(cursor, new(1, 6));

        // When: move_right from end of line 1 (next line exists)
        let cursor = move_right(cursor, &buf);
        // Then: wraps to (2, 0)
        assert_eq!(cursor, new(2, 0));

        // When: move_left from start of line 2
        let cursor = move_left(cursor, &buf);
        // Then: wraps to (1, end) = (1, 6)
        assert_eq!(cursor, new(1, 6));

        // When: move_up from line 1 to line 0
        let cursor_at_1_6 = new(1, 6);
        let cursor = move_up(cursor_at_1_6, &buf);
        // Then: cursor is at (0, 5) -- clamped to line 0 length
        assert_eq!(cursor, new(0, 5));

        // When: move_up from line 0 (already at top)
        let cursor = move_up(cursor, &buf);
        // Then: stays at line 0, column unchanged
        assert_eq!(cursor, new(0, 5));

        // When: move_down to line 2, column is clamped
        let cursor_at_0_5 = new(0, 5);
        let cursor = move_down(move_down(cursor_at_0_5, &buf), &buf);
        // Then: at (2, 2) -- clamped to "Hi" length
        assert_eq!(cursor, new(2, 2));

        // When: move_right from end of last line (no next line)
        let cursor = move_right(cursor, &buf);
        // Then: stays at (2, 2)
        assert_eq!(cursor, new(2, 2));

        // When: ensure_within_bounds on a cursor beyond buffer
        let out_of_bounds = new(10, 50);
        let clamped = ensure_within_bounds(out_of_bounds, &buf);
        // Then: clamped to last line, last column
        assert_eq!(clamped, new(2, 2));
    }

    // -----------------------------------------------------------------------
    // Unit tests: move_down
    // -----------------------------------------------------------------------

    #[test]
    fn move_down_from_first_line_moves_to_second_line() {
        let buf = Buffer::from_string("aaa\nbbb\nccc");
        let cursor = move_down(new(0, 0), &buf);
        assert_eq!(cursor.line, 1);
    }

    #[test]
    fn move_down_from_last_line_stays_on_last_line() {
        let buf = Buffer::from_string("aaa\nbbb");
        let cursor = move_down(new(1, 0), &buf);
        assert_eq!(cursor.line, 1);
    }

    #[test]
    fn move_down_clamps_column_to_shorter_line() {
        // line 0: "abcdef" (6 chars), line 1: "xy" (2 chars)
        let buf = Buffer::from_string("abcdef\nxy");
        let cursor = move_down(new(0, 5), &buf);
        assert_eq!(cursor, new(1, 2));
    }

    // -----------------------------------------------------------------------
    // Unit tests: move_up
    // -----------------------------------------------------------------------

    #[test]
    fn move_up_from_second_line_moves_to_first_line() {
        let buf = Buffer::from_string("aaa\nbbb");
        let cursor = move_up(new(1, 0), &buf);
        assert_eq!(cursor.line, 0);
    }

    #[test]
    fn move_up_from_first_line_stays_on_first_line() {
        let buf = Buffer::from_string("aaa\nbbb");
        let cursor = move_up(new(0, 2), &buf);
        assert_eq!(cursor, new(0, 2));
    }

    #[test]
    fn move_up_clamps_column_to_shorter_line() {
        // line 0: "ab" (2 chars), line 1: "cdefgh" (6 chars)
        let buf = Buffer::from_string("ab\ncdefgh");
        let cursor = move_up(new(1, 5), &buf);
        assert_eq!(cursor, new(0, 2));
    }

    // -----------------------------------------------------------------------
    // Unit tests: move_right
    // -----------------------------------------------------------------------

    #[test]
    fn move_right_within_line_increments_column() {
        let buf = Buffer::from_string("abc");
        let cursor = move_right(new(0, 0), &buf);
        assert_eq!(cursor, new(0, 1));
    }

    #[test]
    fn move_right_at_end_of_line_wraps_to_next_line() {
        let buf = Buffer::from_string("ab\ncd");
        let cursor = move_right(new(0, 2), &buf);
        assert_eq!(cursor, new(1, 0));
    }

    #[test]
    fn move_right_at_end_of_last_line_stays_clamped() {
        let buf = Buffer::from_string("ab\ncd");
        let cursor = move_right(new(1, 2), &buf);
        assert_eq!(cursor, new(1, 2));
    }

    // -----------------------------------------------------------------------
    // Unit tests: move_left
    // -----------------------------------------------------------------------

    #[test]
    fn move_left_within_line_decrements_column() {
        let buf = Buffer::from_string("abc");
        let cursor = move_left(new(0, 2), &buf);
        assert_eq!(cursor, new(0, 1));
    }

    #[test]
    fn move_left_at_start_of_line_wraps_to_previous_line_end() {
        let buf = Buffer::from_string("ab\ncd");
        let cursor = move_left(new(1, 0), &buf);
        assert_eq!(cursor, new(0, 2));
    }

    #[test]
    fn move_left_at_start_of_first_line_stays_clamped() {
        let buf = Buffer::from_string("ab\ncd");
        let cursor = move_left(new(0, 0), &buf);
        assert_eq!(cursor, new(0, 0));
    }

    // -----------------------------------------------------------------------
    // Unit tests: ensure_within_bounds
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_within_bounds_clamps_line_beyond_buffer() {
        let buf = Buffer::from_string("abc\ndef");
        let cursor = ensure_within_bounds(new(10, 0), &buf);
        assert_eq!(cursor.line, 1);
    }

    #[test]
    fn ensure_within_bounds_clamps_column_beyond_line_length() {
        let buf = Buffer::from_string("abc\ndef");
        let cursor = ensure_within_bounds(new(0, 50), &buf);
        assert_eq!(cursor, new(0, 3));
    }

    #[test]
    fn ensure_within_bounds_leaves_valid_cursor_unchanged() {
        let buf = Buffer::from_string("abc\ndef");
        let cursor = ensure_within_bounds(new(1, 2), &buf);
        assert_eq!(cursor, new(1, 2));
    }
}
