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

/// Returns the zero-indexed last line of the buffer (0 for an empty buffer).
fn last_line_index(buf: &Buffer) -> usize {
    buffer::line_count(buf).saturating_sub(1)
}

/// Creates a cursor at the given line with column clamped to line length.
fn clamp_column_to_line(line: usize, desired_column: usize, buf: &Buffer) -> Cursor {
    let max_column = line_length(buf, line);
    Cursor {
        line,
        column: desired_column.min(max_column),
    }
}

/// Moves the cursor down by one line, clamping column to the target line length.
///
/// If already on the last line, the cursor stays unchanged.
pub fn move_down(cursor: Cursor, buf: &Buffer) -> Cursor {
    let last_line = last_line_index(buf);
    let new_line = if cursor.line < last_line {
        cursor.line + 1
    } else {
        cursor.line
    };
    clamp_column_to_line(new_line, cursor.column, buf)
}

/// Moves the cursor up by one line, clamping column to the target line length.
///
/// If already on the first line, the cursor stays unchanged.
pub fn move_up(cursor: Cursor, buf: &Buffer) -> Cursor {
    let new_line = cursor.line.saturating_sub(1);
    clamp_column_to_line(new_line, cursor.column, buf)
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
    } else if cursor.line < last_line_index(buf) {
        Cursor {
            line: cursor.line + 1,
            column: 0,
        }
    } else {
        cursor
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
    let clamped_line = cursor.line.min(last_line_index(buf));
    clamp_column_to_line(clamped_line, cursor.column, buf)
}

/// Moves the cursor to column 0 of the current line (vim `0`).
pub fn move_to_line_start(cursor: Cursor, _buf: &Buffer) -> Cursor {
    Cursor {
        line: cursor.line,
        column: 0,
    }
}

/// Moves the cursor to the end of the current line (vim `$`).
pub fn move_to_line_end(cursor: Cursor, buf: &Buffer) -> Cursor {
    let len = line_length(buf, cursor.line);
    Cursor {
        line: cursor.line,
        column: if len > 0 { len.saturating_sub(1) } else { 0 },
    }
}

/// Moves the cursor to the insert position after the last character (vim `A`).
///
/// Unlike `move_to_line_end` which lands on the last character (vim `$`),
/// this positions the cursor one past the last character, where new text
/// would be appended in insert mode.
pub fn move_to_line_end_for_insert(cursor: Cursor, buf: &Buffer) -> Cursor {
    let len = line_length(buf, cursor.line);
    Cursor {
        line: cursor.line,
        column: len,
    }
}

/// Moves the cursor right by one column on the same line, clamped to line length (vim `a`).
///
/// Unlike `move_right` which wraps to the next line at end of line,
/// this stays on the current line and clamps to the line length (insert position).
pub fn move_right_on_line(cursor: Cursor, buf: &Buffer) -> Cursor {
    let len = line_length(buf, cursor.line);
    Cursor {
        line: cursor.line,
        column: (cursor.column + 1).min(len),
    }
}

/// Moves the cursor to the first non-blank character on the current line (vim `^`).
pub fn move_to_first_non_blank(cursor: Cursor, buf: &Buffer) -> Cursor {
    let line_content = buffer::get_line(buf, cursor.line).unwrap_or("");
    let first_non_blank = line_content
        .chars()
        .position(|c| !c.is_whitespace())
        .unwrap_or(0);
    Cursor {
        line: cursor.line,
        column: first_non_blank,
    }
}

/// Moves the cursor to the start of the document (vim `gg`).
pub fn move_to_document_start(_cursor: Cursor, _buf: &Buffer) -> Cursor {
    Cursor { line: 0, column: 0 }
}

/// Moves the cursor to the last line of the document (vim `G`).
pub fn move_to_document_end(_cursor: Cursor, buf: &Buffer) -> Cursor {
    Cursor {
        line: last_line_index(buf),
        column: 0,
    }
}

/// Returns true if the character is a "word" character (alphanumeric or underscore).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns the line content without trailing newline as a Vec<char>.
fn line_chars(buf: &Buffer, line_index: usize) -> Vec<char> {
    buffer::get_line(buf, line_index)
        .unwrap_or("")
        .trim_end_matches('\n')
        .chars()
        .collect()
}

/// Moves the cursor forward to the start of the next word (vim `w`).
///
/// Word boundaries are transitions between word characters (alphanumeric/underscore)
/// and non-word characters, or whitespace boundaries. Skips across lines.
pub fn move_word_forward(cursor: Cursor, buf: &Buffer) -> Cursor {
    let total_lines = buffer::line_count(buf);
    let mut line = cursor.line;
    let mut col = cursor.column;
    let mut chars = line_chars(buf, line);

    // If we're within the current line, skip past current word/non-word group
    if col < chars.len() {
        let starting_is_word = is_word_char(chars[col]);
        // Skip characters of the same class
        while col < chars.len() && is_word_char(chars[col]) == starting_is_word {
            col += 1;
        }
        // Skip whitespace
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        if col < chars.len() {
            return Cursor { line, column: col };
        }
    }

    // Move to next line(s) to find start of next word
    line += 1;
    while line < total_lines {
        chars = line_chars(buf, line);
        // Skip leading whitespace
        let first_non_ws = chars.iter().position(|c| !c.is_whitespace());
        if let Some(pos) = first_non_ws {
            return Cursor { line, column: pos };
        }
        line += 1;
    }

    // No more words; stay at end of buffer
    let last_line = last_line_index(buf);
    let last_len = line_length(buf, last_line);
    Cursor {
        line: last_line,
        column: if last_len > 0 {
            last_len.saturating_sub(1)
        } else {
            0
        },
    }
}

/// Moves the cursor backward to the start of the previous word (vim `b`).
///
/// Word boundaries are transitions between word characters (alphanumeric/underscore)
/// and non-word characters, or whitespace boundaries. Skips across lines.
pub fn move_word_backward(cursor: Cursor, buf: &Buffer) -> Cursor {
    let mut line = cursor.line;
    let mut col = cursor.column;
    let mut chars = line_chars(buf, line);

    // If at start of line, go to previous line end
    if col == 0 {
        if line == 0 {
            return Cursor { line: 0, column: 0 };
        }
        line -= 1;
        chars = line_chars(buf, line);
        col = chars.len();
    }

    // Move back one step to look behind
    col = col.saturating_sub(1);

    // Skip whitespace backward
    while col > 0 && chars[col].is_whitespace() {
        col -= 1;
    }

    // Handle case where we landed on whitespace at position 0
    if chars.is_empty() || (col == 0 && chars[0].is_whitespace()) {
        // Try previous lines
        if line == 0 {
            return Cursor { line: 0, column: 0 };
        }
        line -= 1;
        chars = line_chars(buf, line);
        if chars.is_empty() {
            return Cursor { line, column: 0 };
        }
        col = chars.len().saturating_sub(1);
        // Skip whitespace backward again on the new line
        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }
    }

    // Now skip backward over the current word/non-word class to find the start
    let current_is_word = is_word_char(chars[col]);
    while col > 0 && is_word_char(chars[col - 1]) == current_is_word {
        col -= 1;
    }

    Cursor { line, column: col }
}

/// Moves the cursor to the end of the current or next word (vim `e`).
///
/// Word boundaries are transitions between word characters (alphanumeric/underscore)
/// and non-word characters, or whitespace boundaries. Skips across lines.
pub fn move_word_end(cursor: Cursor, buf: &Buffer) -> Cursor {
    let total_lines = buffer::line_count(buf);
    let mut line = cursor.line;
    let mut col = cursor.column;
    let mut chars = line_chars(buf, line);

    // Move forward at least one position to avoid staying on current word end
    if !chars.is_empty() && col < chars.len() {
        col += 1;
    }

    // Skip whitespace (possibly across lines)
    loop {
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        if col < chars.len() {
            break;
        }
        // Move to next line
        line += 1;
        if line >= total_lines {
            // At end of buffer
            let last_line = last_line_index(buf);
            let last_len = line_length(buf, last_line);
            return Cursor {
                line: last_line,
                column: if last_len > 0 {
                    last_len.saturating_sub(1)
                } else {
                    0
                },
            };
        }
        chars = line_chars(buf, line);
        col = 0;
    }

    // Now col is at a non-whitespace character; skip to end of word/non-word group
    let current_is_word = is_word_char(chars[col]);
    while col + 1 < chars.len() && is_word_char(chars[col + 1]) == current_is_word {
        col += 1;
    }

    Cursor { line, column: col }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Table-driven: move_down boundary clamping
    // -----------------------------------------------------------------------

    #[test]
    fn move_down_boundary_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "aaa\nbbb\nccc",
                new(0, 0),
                new(1, 0),
                "first line moves to second",
            ),
            (
                "aaa\nbbb",
                new(1, 0),
                new(1, 0),
                "last line stays on last line",
            ),
            (
                "abcdef\nxy",
                new(0, 5),
                new(1, 2),
                "clamps column to shorter line",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_down(*start, &buf);
            assert_eq!(result, *expected, "move_down: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_up boundary clamping
    // -----------------------------------------------------------------------

    #[test]
    fn move_up_boundary_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "aaa\nbbb",
                new(1, 0),
                new(0, 0),
                "second line moves to first",
            ),
            (
                "aaa\nbbb",
                new(0, 2),
                new(0, 2),
                "first line stays on first line",
            ),
            (
                "ab\ncdefgh",
                new(1, 5),
                new(0, 2),
                "clamps column to shorter line",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_up(*start, &buf);
            assert_eq!(result, *expected, "move_up: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_right boundary clamping
    // -----------------------------------------------------------------------

    #[test]
    fn move_right_boundary_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("abc", new(0, 0), new(0, 1), "within line increments column"),
            (
                "ab\ncd",
                new(0, 2),
                new(1, 0),
                "end of line wraps to next line",
            ),
            (
                "ab\ncd",
                new(1, 2),
                new(1, 2),
                "end of last line stays clamped",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_right(*start, &buf);
            assert_eq!(result, *expected, "move_right: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_left boundary clamping
    // -----------------------------------------------------------------------

    #[test]
    fn move_left_boundary_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("abc", new(0, 2), new(0, 1), "within line decrements column"),
            (
                "ab\ncd",
                new(1, 0),
                new(0, 2),
                "start of line wraps to previous line end",
            ),
            (
                "ab\ncd",
                new(0, 0),
                new(0, 0),
                "start of first line stays clamped",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_left(*start, &buf);
            assert_eq!(result, *expected, "move_left: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: ensure_within_bounds
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_within_bounds_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "abc\ndef",
                new(10, 0),
                new(1, 0),
                "clamps line beyond buffer",
            ),
            (
                "abc\ndef",
                new(0, 50),
                new(0, 3),
                "clamps column beyond line length",
            ),
            (
                "abc\ndef",
                new(1, 2),
                new(1, 2),
                "leaves valid cursor unchanged",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = ensure_within_bounds(*start, &buf);
            assert_eq!(result, *expected, "ensure_within_bounds: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_to_line_start (vim 0)
    // -----------------------------------------------------------------------

    #[test]
    fn move_to_line_start_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("hello world", new(0, 5), new(0, 0), "sets column to zero"),
            ("hello", new(0, 0), new(0, 0), "already at zero stays"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_to_line_start(*start, &buf);
            assert_eq!(result, *expected, "move_to_line_start: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_to_line_end (vim $)
    // -----------------------------------------------------------------------

    #[test]
    fn move_to_line_end_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("hello", new(0, 0), new(0, 4), "moves to last character"),
            ("", new(0, 0), new(0, 0), "empty line stays at zero"),
            (
                "abc\ndefgh",
                new(1, 0),
                new(1, 4),
                "second line moves to last char",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_to_line_end(*start, &buf);
            assert_eq!(result, *expected, "move_to_line_end: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_to_first_non_blank (vim ^)
    // -----------------------------------------------------------------------

    #[test]
    fn move_to_first_non_blank_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("   hello", new(0, 0), new(0, 3), "skips leading spaces"),
            ("hello", new(0, 5), new(0, 0), "no leading spaces"),
            ("\t  world", new(0, 0), new(0, 3), "tabs and spaces"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_to_first_non_blank(*start, &buf);
            assert_eq!(result, *expected, "move_to_first_non_blank: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_to_document_start (vim gg)
    // -----------------------------------------------------------------------

    #[test]
    fn move_to_document_start_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "aaa\nbbb\nccc",
                new(2, 2),
                new(0, 0),
                "from middle of buffer",
            ),
            ("aaa\nbbb", new(0, 0), new(0, 0), "already at start"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_to_document_start(*start, &buf);
            assert_eq!(result, *expected, "move_to_document_start: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_to_document_end (vim G)
    // -----------------------------------------------------------------------

    #[test]
    fn move_to_document_end_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("aaa\nbbb\nccc", new(0, 0), new(2, 0), "from first line"),
            ("aaa\nbbb", new(1, 2), new(1, 0), "already at last line"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_to_document_end(*start, &buf);
            assert_eq!(result, *expected, "move_to_document_end: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Acceptance test: move_word_forward across multiple words (vim w)
    // -----------------------------------------------------------------------

    #[test]
    fn move_word_forward_across_multiple_words() {
        let buf = Buffer::from_string("hello world foo");
        // Start at 'h'
        let c1 = move_word_forward(new(0, 0), &buf);
        assert_eq!(c1, new(0, 6)); // 'w' of "world"
        let c2 = move_word_forward(c1, &buf);
        assert_eq!(c2, new(0, 12)); // 'f' of "foo"
    }

    // -----------------------------------------------------------------------
    // Unit test: move_word_forward punctuation (multi-step, kept separate)
    // -----------------------------------------------------------------------

    #[test]
    fn move_word_forward_skips_punctuation_as_separate_word() {
        let buf = Buffer::from_string("foo.bar");
        let c1 = move_word_forward(new(0, 0), &buf);
        // After "foo", '.' is different class, so word start is at '.'
        assert_eq!(c1, new(0, 3)); // '.'
        let c2 = move_word_forward(c1, &buf);
        assert_eq!(c2, new(0, 4)); // 'b' of "bar"
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_word_forward single-step cases (vim w)
    // -----------------------------------------------------------------------

    #[test]
    fn move_word_forward_single_step_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            ("end\nstart", new(0, 0), new(1, 0), "across lines"),
            ("last", new(0, 3), new(0, 3), "at end of buffer stays"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_word_forward(*start, &buf);
            assert_eq!(result, *expected, "move_word_forward: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_word_backward (vim b)
    // -----------------------------------------------------------------------

    #[test]
    fn move_word_backward_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "hello world",
                new(0, 6),
                new(0, 0),
                "to previous word start",
            ),
            ("first\nsecond", new(1, 0), new(0, 0), "across lines"),
            ("hello", new(0, 0), new(0, 0), "at start of buffer stays"),
            ("hello world", new(0, 8), new(0, 6), "from middle of word"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_word_backward(*start, &buf);
            assert_eq!(result, *expected, "move_word_backward: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: move_word_end (vim e)
    // -----------------------------------------------------------------------

    #[test]
    fn move_word_end_cases() {
        // (buffer, start_cursor, expected_cursor, label)
        let cases: Vec<(&str, Cursor, Cursor, &str)> = vec![
            (
                "hello world",
                new(0, 0),
                new(0, 4),
                "to end of current word",
            ),
            ("hello world", new(0, 4), new(0, 10), "to end of next word"),
            ("hi\nthere", new(0, 1), new(1, 4), "across lines"),
            ("end", new(0, 2), new(0, 2), "at end of buffer stays"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = move_word_end(*start, &buf);
            assert_eq!(result, *expected, "move_word_end: {}", label);
        }
    }
}
