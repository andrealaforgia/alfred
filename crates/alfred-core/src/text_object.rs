//! Text objects: pure functions that compute ranges for vim-style text objects.
//!
//! Text objects define (start, end) cursor ranges within a buffer.
//! They are used in operator-pending mode: after an operator like `d`, `c`, or `y`,
//! a text object key sequence (e.g. `iw`, `a"`, `i(`) selects the range to act on.
//!
//! All functions are pure: they take a cursor and buffer reference and return
//! an optional (start, end) cursor pair. The range is exclusive of the end position
//! (consistent with `delete_char_range`).

use crate::buffer::{self, Buffer};
use crate::cursor::Cursor;

/// Modifier for text objects: Inner (inside) or Around (including delimiters/whitespace).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObjectModifier {
    Inner,
    Around,
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

/// Computes the range for the "inner word" text object (`iw`).
///
/// Returns the start (inclusive) and end (exclusive) cursors covering the current word
/// under the cursor, excluding surrounding whitespace.
/// If the cursor is on whitespace, selects the whitespace run instead.
pub fn inner_word(cursor: Cursor, buf: &Buffer) -> Option<(Cursor, Cursor)> {
    let chars = line_chars(buf, cursor.line);
    if chars.is_empty() || cursor.column >= chars.len() {
        return None;
    }

    let col = cursor.column;
    let current_is_word = is_word_char(chars[col]);
    let current_is_whitespace = chars[col].is_whitespace();

    // Find start of the current word/non-word/whitespace group
    let mut start = col;
    if current_is_whitespace {
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
    } else {
        while start > 0
            && is_word_char(chars[start - 1]) == current_is_word
            && !chars[start - 1].is_whitespace()
        {
            start -= 1;
        }
    }

    // Find end of the current word/non-word/whitespace group
    let mut end = col;
    if current_is_whitespace {
        while end + 1 < chars.len() && chars[end + 1].is_whitespace() {
            end += 1;
        }
    } else {
        while end + 1 < chars.len()
            && is_word_char(chars[end + 1]) == current_is_word
            && !chars[end + 1].is_whitespace()
        {
            end += 1;
        }
    }

    // end is inclusive, convert to exclusive
    Some((
        Cursor {
            line: cursor.line,
            column: start,
        },
        Cursor {
            line: cursor.line,
            column: end + 1,
        },
    ))
}

/// Computes the range for the "around word" text object (`aw`).
///
/// Returns the start (inclusive) and end (exclusive) cursors covering the current word
/// plus trailing whitespace (or leading whitespace if no trailing space).
pub fn around_word(cursor: Cursor, buf: &Buffer) -> Option<(Cursor, Cursor)> {
    let (word_start, word_end) = inner_word(cursor, buf)?;
    let chars = line_chars(buf, cursor.line);

    // Try to include trailing whitespace first
    let mut end = word_end.column;
    if end < chars.len() && chars[end].is_whitespace() {
        while end < chars.len() && chars[end].is_whitespace() {
            end += 1;
        }
        return Some((
            word_start,
            Cursor {
                line: cursor.line,
                column: end,
            },
        ));
    }

    // No trailing whitespace: include leading whitespace
    let mut start = word_start.column;
    if start > 0 && chars[start - 1].is_whitespace() {
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        return Some((
            Cursor {
                line: cursor.line,
                column: start,
            },
            word_end,
        ));
    }

    // No surrounding whitespace at all
    Some((word_start, word_end))
}

/// Computes the range for the "inner quotes" text object (`i"`, `i'`).
///
/// Searches the current line for a pair of matching quote characters surrounding
/// the cursor position and returns the range of text between them (exclusive of quotes).
pub fn inner_quotes(cursor: Cursor, buf: &Buffer, quote_char: char) -> Option<(Cursor, Cursor)> {
    let chars = line_chars(buf, cursor.line);
    if chars.is_empty() {
        return None;
    }

    // Find the opening quote: search backward from cursor, or if cursor is on a quote,
    // that could be the opener. Also handle cursor between quotes.
    let mut open = None;
    let mut close = None;

    // Strategy: find all quote positions on the line, then find the pair surrounding cursor
    let quote_positions: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, &c)| c == quote_char)
        .map(|(i, _)| i)
        .collect();

    // Need at least two quotes to form a pair
    if quote_positions.len() < 2 {
        return None;
    }

    // Find the pair that surrounds the cursor (cursor is between open and close, or on them)
    for pair in quote_positions.chunks(2) {
        if pair.len() == 2 {
            let (o, c) = (pair[0], pair[1]);
            if cursor.column >= o && cursor.column <= c {
                open = Some(o);
                close = Some(c);
                break;
            }
        }
    }

    let open = open?;
    let close = close?;

    // Inner: between the quotes (exclusive of both)
    Some((
        Cursor {
            line: cursor.line,
            column: open + 1,
        },
        Cursor {
            line: cursor.line,
            column: close,
        },
    ))
}

/// Computes the range for the "around quotes" text object (`a"`, `a'`).
///
/// Like `inner_quotes` but includes the quote characters themselves.
pub fn around_quotes(cursor: Cursor, buf: &Buffer, quote_char: char) -> Option<(Cursor, Cursor)> {
    let (inner_start, inner_end) = inner_quotes(cursor, buf, quote_char)?;

    // Around includes the quotes themselves
    Some((
        Cursor {
            line: cursor.line,
            column: inner_start.column.saturating_sub(1),
        },
        Cursor {
            line: cursor.line,
            column: inner_end.column + 1,
        },
    ))
}

/// Computes the range for the "inner parentheses/brackets/braces" text object (`i(`, `i[`, `i{`).
///
/// Finds matching open/close bracket pair surrounding the cursor (respecting nesting)
/// and returns the range of text between them (exclusive of brackets). Supports multi-line.
pub fn inner_parens(
    cursor: Cursor,
    buf: &Buffer,
    open_char: char,
    close_char: char,
) -> Option<(Cursor, Cursor)> {
    // Collect all characters with their (line, col) positions
    let total_lines = buffer::line_count(buf);
    let mut all_chars: Vec<(char, usize, usize)> = Vec::new(); // (char, line, col)
    let mut cursor_index = None;

    for line_idx in 0..total_lines {
        let line_str = buffer::get_line(buf, line_idx).unwrap_or("");
        for (col, ch) in line_str.chars().enumerate() {
            if line_idx == cursor.line && col == cursor.column {
                cursor_index = Some(all_chars.len());
            }
            all_chars.push((ch, line_idx, col));
        }
    }

    let cursor_idx = cursor_index?;

    // Find the opening bracket: scan backward from cursor
    let mut depth = 0i32;
    let mut open_idx = None;

    // If cursor is on the open bracket itself, use it
    if all_chars[cursor_idx].0 == open_char {
        open_idx = Some(cursor_idx);
    } else {
        for i in (0..=cursor_idx).rev() {
            let ch = all_chars[i].0;
            if ch == close_char {
                depth += 1;
            } else if ch == open_char {
                if depth == 0 {
                    open_idx = Some(i);
                    break;
                }
                depth -= 1;
            }
        }
    }

    let open_idx = open_idx?;

    // Find the closing bracket: scan forward from the opening bracket
    let mut depth = 1i32;
    let mut close_idx = None;
    for (i, &(ch, _, _)) in all_chars.iter().enumerate().skip(open_idx + 1) {
        if ch == open_char {
            depth += 1;
        } else if ch == close_char {
            depth -= 1;
            if depth == 0 {
                close_idx = Some(i);
                break;
            }
        }
    }

    let close_idx = close_idx?;

    // Inner: between the brackets (exclusive of both)
    // The start of inner is one position after the opening bracket
    // The end is at the closing bracket position (exclusive)
    let inner_start_idx = open_idx + 1;
    let inner_end_idx = close_idx;

    if inner_start_idx >= all_chars.len() || inner_start_idx > inner_end_idx {
        // Empty parens: return the same position for start and end
        let (_, cl_line, cl_col) = all_chars[close_idx];
        return Some((
            Cursor {
                line: cl_line,
                column: cl_col,
            },
            Cursor {
                line: cl_line,
                column: cl_col,
            },
        ));
    }

    let (_, start_line, start_col) = all_chars[inner_start_idx];
    let (_, end_line, end_col) = all_chars[inner_end_idx];

    Some((
        Cursor {
            line: start_line,
            column: start_col,
        },
        Cursor {
            line: end_line,
            column: end_col,
        },
    ))
}

/// Computes the range for the "around parentheses/brackets/braces" text object (`a(`, `a[`, `a{`).
///
/// Like `inner_parens` but includes the bracket characters themselves.
pub fn around_parens(
    cursor: Cursor,
    buf: &Buffer,
    open_char: char,
    close_char: char,
) -> Option<(Cursor, Cursor)> {
    // We need to find the open and close bracket positions
    let total_lines = buffer::line_count(buf);
    let mut all_chars: Vec<(char, usize, usize)> = Vec::new();
    let mut cursor_index = None;

    for line_idx in 0..total_lines {
        let line_str = buffer::get_line(buf, line_idx).unwrap_or("");
        for (col, ch) in line_str.chars().enumerate() {
            if line_idx == cursor.line && col == cursor.column {
                cursor_index = Some(all_chars.len());
            }
            all_chars.push((ch, line_idx, col));
        }
    }

    let cursor_idx = cursor_index?;

    // Find the opening bracket
    let mut depth = 0i32;
    let mut open_idx = None;

    if all_chars[cursor_idx].0 == open_char {
        open_idx = Some(cursor_idx);
    } else {
        for i in (0..=cursor_idx).rev() {
            let ch = all_chars[i].0;
            if ch == close_char {
                depth += 1;
            } else if ch == open_char {
                if depth == 0 {
                    open_idx = Some(i);
                    break;
                }
                depth -= 1;
            }
        }
    }

    let open_idx = open_idx?;

    // Find the closing bracket
    let mut depth = 1i32;
    let mut close_idx = None;
    for (i, &(ch, _, _)) in all_chars.iter().enumerate().skip(open_idx + 1) {
        if ch == open_char {
            depth += 1;
        } else if ch == close_char {
            depth -= 1;
            if depth == 0 {
                close_idx = Some(i);
                break;
            }
        }
    }

    let close_idx = close_idx?;

    let (_, ol, oc) = all_chars[open_idx];
    let (_, cl, cc) = all_chars[close_idx];

    // Around includes the brackets themselves, end is exclusive (one past closing bracket)
    Some((
        Cursor {
            line: ol,
            column: oc,
        },
        Cursor {
            line: cl,
            column: cc + 1,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor;

    // -----------------------------------------------------------------------
    // Table-driven: inner_word (iw)
    // -----------------------------------------------------------------------

    #[test]
    fn inner_word_cases() {
        let cases: Vec<(&str, Cursor, Option<(Cursor, Cursor)>, &str)> = vec![
            (
                "hello world",
                cursor::new(0, 7),
                Some((cursor::new(0, 6), cursor::new(0, 11))),
                "cursor on 'world' selects 'world'",
            ),
            (
                "hello world",
                cursor::new(0, 0),
                Some((cursor::new(0, 0), cursor::new(0, 5))),
                "cursor on 'hello' selects 'hello'",
            ),
            (
                "hello world",
                cursor::new(0, 5),
                Some((cursor::new(0, 5), cursor::new(0, 6))),
                "cursor on space selects the space",
            ),
            (
                "foo.bar",
                cursor::new(0, 0),
                Some((cursor::new(0, 0), cursor::new(0, 3))),
                "cursor on 'foo' in foo.bar selects 'foo'",
            ),
            ("", cursor::new(0, 0), None, "empty buffer returns None"),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = inner_word(*start, &buf);
            assert_eq!(result, *expected, "inner_word: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: around_word (aw)
    // -----------------------------------------------------------------------

    #[test]
    fn around_word_cases() {
        let cases: Vec<(&str, Cursor, Option<(Cursor, Cursor)>, &str)> = vec![
            (
                "hello world",
                cursor::new(0, 0),
                Some((cursor::new(0, 0), cursor::new(0, 6))),
                "cursor on 'hello' selects 'hello ' (word + trailing space)",
            ),
            (
                "hello world",
                cursor::new(0, 6),
                Some((cursor::new(0, 5), cursor::new(0, 11))),
                "cursor on 'world' selects ' world' (leading space + word, no trailing)",
            ),
            (
                "one",
                cursor::new(0, 0),
                Some((cursor::new(0, 0), cursor::new(0, 3))),
                "single word, no surrounding spaces",
            ),
        ];

        for (buffer_text, start, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = around_word(*start, &buf);
            assert_eq!(result, *expected, "around_word: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: inner_quotes (i")
    // -----------------------------------------------------------------------

    #[test]
    fn inner_quotes_cases() {
        let cases: Vec<(&str, Cursor, char, Option<(Cursor, Cursor)>, &str)> = vec![
            (
                r#"say "hello" done"#,
                cursor::new(0, 6),
                '"',
                Some((cursor::new(0, 5), cursor::new(0, 10))),
                "cursor inside double quotes selects content between quotes",
            ),
            (
                r#"say "hello" done"#,
                cursor::new(0, 4),
                '"',
                Some((cursor::new(0, 5), cursor::new(0, 10))),
                "cursor on opening quote selects content between quotes",
            ),
            (
                "no quotes here",
                cursor::new(0, 5),
                '"',
                None,
                "no quotes returns None",
            ),
            (
                r#""hello""#,
                cursor::new(0, 3),
                '"',
                Some((cursor::new(0, 1), cursor::new(0, 6))),
                "cursor inside quotes at start of line",
            ),
        ];

        for (buffer_text, start, quote, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = inner_quotes(*start, &buf, *quote);
            assert_eq!(result, *expected, "inner_quotes: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: around_quotes (a")
    // -----------------------------------------------------------------------

    #[test]
    fn around_quotes_cases() {
        let cases: Vec<(&str, Cursor, char, Option<(Cursor, Cursor)>, &str)> = vec![(
            r#"say "hello" done"#,
            cursor::new(0, 6),
            '"',
            Some((cursor::new(0, 4), cursor::new(0, 11))),
            "around quotes includes the quote characters",
        )];

        for (buffer_text, start, quote, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = around_quotes(*start, &buf, *quote);
            assert_eq!(result, *expected, "around_quotes: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: inner_parens (i()
    // -----------------------------------------------------------------------

    #[test]
    fn inner_parens_cases() {
        let cases: Vec<(&str, Cursor, char, char, Option<(Cursor, Cursor)>, &str)> = vec![
            (
                "fn(arg1, arg2)",
                cursor::new(0, 5),
                '(',
                ')',
                Some((cursor::new(0, 3), cursor::new(0, 13))),
                "cursor inside parens selects content between parens",
            ),
            (
                "fn()",
                cursor::new(0, 2),
                '(',
                ')',
                Some((cursor::new(0, 3), cursor::new(0, 3))),
                "cursor on opening paren of empty parens returns empty range",
            ),
            (
                "no parens",
                cursor::new(0, 3),
                '(',
                ')',
                None,
                "no parens returns None",
            ),
            (
                "a(b(c)d)e",
                cursor::new(0, 4),
                '(',
                ')',
                Some((cursor::new(0, 4), cursor::new(0, 5))),
                "nested parens: cursor inside inner pair selects inner content",
            ),
        ];

        for (buffer_text, start, open, close, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = inner_parens(*start, &buf, *open, *close);
            assert_eq!(result, *expected, "inner_parens: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Table-driven: around_parens (a()
    // -----------------------------------------------------------------------

    #[test]
    fn around_parens_cases() {
        let cases: Vec<(&str, Cursor, char, char, Option<(Cursor, Cursor)>, &str)> = vec![
            (
                "fn(arg1, arg2)",
                cursor::new(0, 5),
                '(',
                ')',
                Some((cursor::new(0, 2), cursor::new(0, 14))),
                "around parens includes the parentheses",
            ),
            (
                "a[b, c]d",
                cursor::new(0, 3),
                '[',
                ']',
                Some((cursor::new(0, 1), cursor::new(0, 7))),
                "around brackets includes the brackets",
            ),
        ];

        for (buffer_text, start, open, close, expected, label) in &cases {
            let buf = Buffer::from_string(buffer_text);
            let result = around_parens(*start, &buf, *open, *close);
            assert_eq!(result, *expected, "around_parens: {}", label);
        }
    }

    // -----------------------------------------------------------------------
    // Multi-line inner_parens
    // -----------------------------------------------------------------------

    #[test]
    fn inner_parens_multiline() {
        let buf = Buffer::from_string("if (\n  true\n)");
        let result = inner_parens(cursor::new(1, 2), &buf, '(', ')');
        assert!(result.is_some(), "should find parens across lines");
        let (start, end) = result.unwrap();
        // The opening paren is at (0, 3), so inner starts at (0, 4) which is '\n'
        // The closing paren is at (2, 0), so inner ends at (2, 0)
        assert_eq!(
            start,
            cursor::new(0, 4),
            "inner start is after opening paren"
        );
        assert_eq!(end, cursor::new(2, 0), "inner end is at closing paren");
    }
}
