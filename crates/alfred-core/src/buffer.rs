//! Buffer: immutable text container wrapping ropey::Rope with metadata.
//!
//! Buffer is the core text-storage abstraction for the Alfred editor.
//! It wraps a `ropey::Rope` and carries metadata (id, filename, modified flag, version).
//! All operations are pure: modifications return new Buffer instances.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ropey::Rope;

use crate::error::{AlfredError, Result};

/// Global counter for generating unique buffer IDs.
static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

/// Generates the next unique buffer ID.
fn next_id() -> u64 {
    NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed)
}

/// Immutable text buffer wrapping a `ropey::Rope` with editor metadata.
///
/// Buffer is the fundamental text-storage type in Alfred. It carries:
/// - `id`: unique identifier for this buffer
/// - `rope`: the underlying text storage (ropey::Rope)
/// - `filename`: optional filename (the file's name component, not full path)
/// - `file_path`: optional full path to the file on disk
/// - `modified`: whether the buffer has been changed since loading
/// - `version`: monotonically increasing version counter
#[derive(Debug, Clone)]
pub struct Buffer {
    id: u64,
    rope: Rope,
    filename: Option<String>,
    file_path: Option<PathBuf>,
    modified: bool,
    version: u64,
}

impl Buffer {
    /// Creates a new Buffer from a string.
    ///
    /// Useful for testing and for creating buffers from non-file sources.
    /// The buffer has no filename, `modified` is `false`, and `version` starts at 1.
    pub fn from_string(text: &str) -> Self {
        Buffer {
            id: next_id(),
            rope: Rope::from_str(text),
            filename: None,
            file_path: None,
            modified: false,
            version: 1,
        }
    }

    /// Loads a text file into a new Buffer.
    ///
    /// Returns an error if the file cannot be read. The buffer's `modified`
    /// flag is `false` and `version` starts at 1.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).map_err(|source| AlfredError::FileReadError {
                path: path.to_path_buf(),
                source,
            })?;

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from);

        Ok(Buffer {
            id: next_id(),
            rope: Rope::from_str(&content),
            filename,
            file_path: Some(path.to_path_buf()),
            modified: false,
            version: 1,
        })
    }

    /// Returns the unique identifier for this buffer.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the filename (just the name component), or None if unnamed.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Returns the full file path, or None if the buffer was not loaded from a file.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Returns whether the buffer has been modified since loading.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Returns the current version of this buffer.
    pub fn version(&self) -> u64 {
        self.version
    }
}

/// Returns the number of lines in the buffer.
///
/// An empty buffer has 1 line (the empty line). A buffer ending with
/// a newline has an additional empty trailing line per ropey semantics.
pub fn line_count(buffer: &Buffer) -> usize {
    buffer.rope.len_lines()
}

/// Returns the content of line `index` (zero-indexed), or `None` if out of bounds.
///
/// Lines include their trailing newline character if present.
pub fn get_line(buffer: &Buffer, index: usize) -> Option<&str> {
    if index >= buffer.rope.len_lines() {
        return None;
    }
    let line = buffer.rope.line(index);
    // ropey::RopeSlice::as_str() returns Some when the slice is contiguous
    // in memory, which is the common case for single lines.
    line.as_str()
}

/// Returns the entire buffer content as a String.
pub fn content(buffer: &Buffer) -> String {
    buffer.rope.to_string()
}

/// Inserts text at the given line and column position, returning a new Buffer.
///
/// The line and column are clamped to valid positions within the buffer.
/// The new buffer has an incremented version and `modified` set to true.
pub fn insert_at(buffer: &Buffer, line: usize, column: usize, text: &str) -> Buffer {
    let mut rope = buffer.rope.clone();
    let char_index = line_column_to_char_index(&rope, line, column);
    rope.insert(char_index, text);

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Deletes one character at the given line and column position, returning a new Buffer.
///
/// If the position is at the end of the buffer (no character to delete),
/// the buffer is returned unchanged.
/// The new buffer has an incremented version and `modified` set to true.
pub fn delete_at(buffer: &Buffer, line: usize, column: usize) -> Buffer {
    let mut rope = buffer.rope.clone();
    let char_index = line_column_to_char_index(&rope, line, column);

    if char_index >= rope.len_chars() {
        return buffer.clone();
    }

    rope.remove(char_index..char_index + 1);

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Deletes an entire line from the buffer, returning a new Buffer.
///
/// If the line index is out of bounds, the buffer is returned unchanged.
/// When the last remaining line is deleted, the buffer becomes empty.
/// The trailing newline of the deleted line (or preceding newline if last line)
/// is also removed to avoid leaving blank lines.
pub fn delete_line(buffer: &Buffer, line: usize) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line >= total_lines {
        return buffer.clone();
    }

    let mut rope = buffer.rope.clone();
    let line_start = rope.line_to_char(line);
    let line_char_count = rope.line(line).len_chars();

    if line_char_count == 0 && rope.len_chars() == 0 {
        // Already empty
        return buffer.clone();
    }

    // Determine the range to delete:
    // Include the trailing newline if there is one, so no blank line is left.
    let end = if line_start + line_char_count <= rope.len_chars() {
        line_start + line_char_count
    } else {
        rope.len_chars()
    };

    // If deleting the last line and there's a preceding newline, also remove it
    let start = if line > 0 && end == rope.len_chars() && line_start > 0 {
        line_start - 1 // remove preceding newline
    } else {
        line_start
    };

    if start < end {
        rope.remove(start..end);
    }

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Saves the buffer's content to the given path, returning a new Buffer with `modified` set to false.
///
/// Writes the full buffer content as UTF-8 text. Returns an error if the file
/// cannot be written (e.g., directory does not exist, permission denied).
/// The returned buffer is identical to the input except `modified` is `false`.
pub fn save_to_file(buffer: &Buffer, path: &Path) -> Result<Buffer> {
    let text = content(buffer);
    std::fs::write(path, &text).map_err(|source| AlfredError::FileWriteError {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(Buffer {
        id: buffer.id,
        rope: buffer.rope.clone(),
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: false,
        version: buffer.version,
    })
}

/// Returns the content of a line as an owned String, without trailing newline.
///
/// If the line index is out of bounds, returns an empty string.
/// Useful for yanking: the caller gets clean text without newline artifacts.
pub fn get_line_content(buffer: &Buffer, line: usize) -> String {
    get_line(buffer, line)
        .unwrap_or("")
        .trim_end_matches('\n')
        .to_string()
}

/// Joins the given line with the next line, separated by a single space.
///
/// If the line is the last line or out of bounds, the buffer is returned unchanged.
/// Both lines' trailing newlines are consumed; the result is one line with a space separator.
pub fn join_lines(buffer: &Buffer, line: usize) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line + 1 >= total_lines {
        return buffer.clone();
    }

    let current_content = get_line_content(buffer, line);
    let next_content = get_line_content(buffer, line + 1);

    // Build joined content: "current next"
    let joined = if current_content.is_empty() {
        next_content
    } else if next_content.is_empty() {
        current_content
    } else {
        format!("{} {}", current_content, next_content)
    };

    // Delete both lines and insert the joined content
    let mut rope = buffer.rope.clone();
    let line_start = rope.line_to_char(line);
    let next_line_end_char = {
        let next_line_start = rope.line_to_char(line + 1);
        let next_line_len = rope.line(line + 1).len_chars();
        next_line_start + next_line_len
    };

    // Remove both lines
    rope.remove(line_start..next_line_end_char);

    // Insert joined content (with newline if there are more lines after)
    let has_more_lines = line + 2 < total_lines;
    let insert_text = if has_more_lines {
        format!("{}\n", joined)
    } else {
        joined
    };
    rope.insert(line_start, &insert_text);

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Replaces the content of a line with new text, preserving the trailing newline if present.
///
/// If the line index is out of bounds, the buffer is returned unchanged.
pub fn replace_line(buffer: &Buffer, line: usize, new_text: &str) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line >= total_lines {
        return buffer.clone();
    }

    let mut rope = buffer.rope.clone();
    let line_start = rope.line_to_char(line);
    let line_chars = rope.line(line).len_chars();
    let has_newline = line_chars > 0 && {
        let last_char_idx = line_start + line_chars - 1;
        last_char_idx < rope.len_chars() && rope.char(last_char_idx) == '\n'
    };

    // Remove old content
    rope.remove(line_start..line_start + line_chars);

    // Insert new content (preserving trailing newline)
    let insert_text = if has_newline {
        format!("{}\n", new_text)
    } else {
        new_text.to_string()
    };
    rope.insert(line_start, &insert_text);

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Deletes text from the given column to the end of the line.
///
/// If the line or column is out of bounds, the buffer is returned unchanged.
/// The trailing newline (if any) is preserved.
pub fn delete_to_line_end(buffer: &Buffer, line: usize, column: usize) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line >= total_lines {
        return buffer.clone();
    }

    let line_content = get_line_content(buffer, line);
    if column >= line_content.len() {
        return buffer.clone();
    }

    let new_content = &line_content[..column];
    replace_line(buffer, line, new_content)
}

/// Searches forward in the buffer for a literal substring, starting after the given position.
///
/// Searches from the character after `(start_line, start_col)` to the end of the buffer,
/// then wraps around from the beginning. Returns the `(line, column)` of the first match,
/// or `None` if the pattern is not found anywhere in the buffer.
///
/// This is a pure function with no side effects.
pub fn find_forward(
    buffer: &Buffer,
    start_line: usize,
    start_col: usize,
    pattern: &str,
) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }

    let total_lines = buffer.rope.len_lines();
    if total_lines == 0 {
        return None;
    }

    // Search from the current line (after start_col) through end of buffer
    for line_idx in start_line..total_lines {
        let line_str = buffer.rope.line(line_idx);
        let line_text = line_str.as_str()?;
        let search_from = if line_idx == start_line {
            // Skip past current position so we don't re-find the same match
            (start_col + 1).min(line_text.len())
        } else {
            0
        };
        if let Some(col) = line_text[search_from..].find(pattern) {
            return Some((line_idx, search_from + col));
        }
    }

    // Wrap around: search from beginning up to (and including) the start position
    for line_idx in 0..=start_line.min(total_lines - 1) {
        let line_str = buffer.rope.line(line_idx);
        let line_text = line_str.as_str()?;
        let search_to = if line_idx == start_line {
            (start_col + pattern.len()).min(line_text.len())
        } else {
            line_text.len()
        };
        if let Some(col) = line_text[..search_to].find(pattern) {
            return Some((line_idx, col));
        }
    }

    None
}

/// Searches backward in the buffer for a literal substring, starting before the given position.
///
/// Searches from just before `(start_line, start_col)` toward the beginning of the buffer,
/// then wraps around from the end. Returns the `(line, column)` of the first match found
/// in reverse order, or `None` if the pattern is not found anywhere.
///
/// This is a pure function with no side effects.
pub fn find_backward(
    buffer: &Buffer,
    start_line: usize,
    start_col: usize,
    pattern: &str,
) -> Option<(usize, usize)> {
    if pattern.is_empty() {
        return None;
    }

    let total_lines = buffer.rope.len_lines();
    if total_lines == 0 {
        return None;
    }

    // Search from current line (before start_col) backward to beginning
    for line_idx in (0..=start_line.min(total_lines - 1)).rev() {
        let line_str = buffer.rope.line(line_idx);
        let line_text = line_str.as_str()?;
        let search_to = if line_idx == start_line {
            start_col.min(line_text.len())
        } else {
            line_text.len()
        };
        if let Some(col) = line_text[..search_to].rfind(pattern) {
            return Some((line_idx, col));
        }
    }

    // Wrap around: search from end of buffer back to the start position
    for line_idx in (start_line.min(total_lines - 1)..total_lines).rev() {
        let line_str = buffer.rope.line(line_idx);
        let line_text = line_str.as_str()?;
        let search_from = if line_idx == start_line {
            (start_col + 1).min(line_text.len())
        } else {
            0
        };
        if search_from < line_text.len() {
            if let Some(col) = line_text[search_from..].rfind(pattern) {
                return Some((line_idx, search_from + col));
            }
        }
    }

    None
}

/// Prepends `indent_str` to the given line, returning a new Buffer.
///
/// If the line index is out of bounds, the buffer is returned unchanged.
/// The new buffer has an incremented version and `modified` set to true.
pub fn indent_line(buffer: &Buffer, line: usize, indent_str: &str) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line >= total_lines {
        return buffer.clone();
    }

    let line_content = get_line_content(buffer, line);
    let indented = format!("{}{}", indent_str, line_content);
    replace_line(buffer, line, &indented)
}

/// Removes up to `indent_width` leading whitespace characters (spaces or tabs) from the given line.
///
/// If the line index is out of bounds, the buffer is returned unchanged.
/// Each tab counts as one character removed toward the limit.
/// The new buffer has an incremented version and `modified` set to true.
pub fn unindent_line(buffer: &Buffer, line: usize, indent_width: usize) -> Buffer {
    let total_lines = buffer.rope.len_lines();
    if line >= total_lines {
        return buffer.clone();
    }

    let line_content = get_line_content(buffer, line);
    let chars_to_remove = line_content
        .chars()
        .take(indent_width)
        .take_while(|c| *c == ' ' || *c == '\t')
        .count();

    if chars_to_remove == 0 {
        return buffer.clone();
    }

    let unindented = &line_content[chars_to_remove..];
    replace_line(buffer, line, unindented)
}

/// Deletes a range of text from (from_line, from_col) to (to_line, to_col) exclusive.
///
/// The range is character-based: all characters from the start position up to
/// (but not including) the end position are removed. If the start position is
/// at or past the end position, the buffer is returned unchanged.
/// Positions are clamped to valid buffer boundaries.
pub fn delete_char_range(
    buffer: &Buffer,
    from_line: usize,
    from_col: usize,
    to_line: usize,
    to_col: usize,
) -> Buffer {
    let mut rope = buffer.rope.clone();
    let start = line_column_to_char_index(&rope, from_line, from_col);
    let end = line_column_to_char_index(&rope, to_line, to_col);

    if start >= end || start >= rope.len_chars() {
        return buffer.clone();
    }

    let clamped_end = end.min(rope.len_chars());
    rope.remove(start..clamped_end);

    Buffer {
        id: buffer.id,
        rope,
        filename: buffer.filename.clone(),
        file_path: buffer.file_path.clone(),
        modified: true,
        version: buffer.version + 1,
    }
}

/// Extracts text from (from_line, from_col) to (to_line, to_col) exclusive.
///
/// Returns the text in the given character range as a String.
/// If the start position is at or past the end position, returns an empty string.
/// Positions are clamped to valid buffer boundaries.
pub fn get_text_range(
    buffer: &Buffer,
    from_line: usize,
    from_col: usize,
    to_line: usize,
    to_col: usize,
) -> String {
    let start = line_column_to_char_index(&buffer.rope, from_line, from_col);
    let end = line_column_to_char_index(&buffer.rope, to_line, to_col);

    if start >= end || start >= buffer.rope.len_chars() {
        return String::new();
    }

    let clamped_end = end.min(buffer.rope.len_chars());
    buffer.rope.slice(start..clamped_end).to_string()
}

/// Converts a (line, column) position to a character index in the rope.
///
/// Clamps the line to the last line and the column to the line length.
fn line_column_to_char_index(rope: &Rope, line: usize, column: usize) -> usize {
    let max_line = rope.len_lines().saturating_sub(1);
    let clamped_line = line.min(max_line);
    let line_start = rope.line_to_char(clamped_line);
    let line_len = rope.line(clamped_line).len_chars();
    let clamped_column = column.min(line_len);
    line_start + clamped_column
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Acceptance test: Buffer loads a file and provides correct access
    // -----------------------------------------------------------------------

    #[test]
    fn given_file_with_content_when_loaded_then_buffer_provides_lines_and_metadata() {
        // Given: a file exists with known multi-line content
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("sample.txt");
        let content = "Line one\nLine two\nLine three\n";
        fs::write(&file_path, content).unwrap();

        // When: we load the file into a Buffer
        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // Then: line_count is accurate
        assert_eq!(super::line_count(&buffer), 4);

        // And: get_line returns correct content for each line
        assert_eq!(super::get_line(&buffer, 0), Some("Line one\n"));
        assert_eq!(super::get_line(&buffer, 1), Some("Line two\n"));
        assert_eq!(super::get_line(&buffer, 2), Some("Line three\n"));

        // And: content returns the full text
        assert_eq!(super::content(&buffer), content);

        // And: filename is stored
        assert_eq!(buffer.filename(), Some("sample.txt"));

        // And: modified is false after loading
        assert!(!buffer.is_modified());
    }

    #[test]
    fn given_nonexistent_file_when_loaded_then_returns_error() {
        // Given: a path to a file that does not exist
        let path = std::path::Path::new("/tmp/nonexistent_alfred_test_file.txt");

        // When: we attempt to load the file
        let result = super::Buffer::from_file(path);

        // Then: we get an error, not a panic
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Unit tests: individual behaviors
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_file_when_loaded_then_buffer_has_one_line_and_empty_content() {
        // Given: an empty file
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("empty.txt");
        fs::write(&file_path, "").unwrap();

        // When: loaded
        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // Then: ropey considers empty string as 1 line
        assert_eq!(super::line_count(&buffer), 1);

        // And: content is empty
        assert_eq!(super::content(&buffer), "");
    }

    #[test]
    fn given_buffer_when_get_line_out_of_bounds_then_returns_none() {
        // Given: a buffer with 2 lines
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("two_lines.txt");
        fs::write(&file_path, "first\nsecond").unwrap();

        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // When/Then: requesting a line beyond the buffer returns None
        assert_eq!(super::get_line(&buffer, 999), None);
    }

    #[test]
    fn given_file_without_trailing_newline_when_loaded_then_last_line_has_no_newline() {
        // Given: a file where the last line has no trailing newline
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("no_trailing.txt");
        fs::write(&file_path, "alpha\nbeta").unwrap();

        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // Then: line_count is 2
        assert_eq!(super::line_count(&buffer), 2);

        // And: last line has no trailing newline
        assert_eq!(super::get_line(&buffer, 0), Some("alpha\n"));
        assert_eq!(super::get_line(&buffer, 1), Some("beta"));

        // And: content roundtrips correctly
        assert_eq!(super::content(&buffer), "alpha\nbeta");
    }

    #[test]
    fn given_buffer_from_file_then_id_is_positive() {
        // Given/When: a buffer loaded from any file
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("id_test.txt");
        fs::write(&file_path, "hello").unwrap();

        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // Then: the buffer has a positive id
        assert!(buffer.id() > 0);
    }

    #[test]
    fn given_buffer_from_file_then_version_starts_at_one() {
        // Given/When: a freshly loaded buffer
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("version_test.txt");
        fs::write(&file_path, "hello").unwrap();

        let buffer = super::Buffer::from_file(&file_path).unwrap();

        // Then: version starts at 1 (initial loaded state)
        assert_eq!(buffer.version(), 1);
    }

    // -----------------------------------------------------------------------
    // Acceptance test: Buffer save_to_file writes content and resets modified
    // -----------------------------------------------------------------------

    #[test]
    fn given_modified_buffer_when_saved_to_file_then_file_contains_content_and_modified_resets() {
        // Given: a buffer loaded from a file, then modified
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("save_test.txt");
        fs::write(&file_path, "Original").unwrap();

        let buffer = super::Buffer::from_file(&file_path).unwrap();
        let buffer = super::insert_at(&buffer, 0, 8, " content");

        // Precondition: buffer is modified
        assert!(buffer.is_modified());

        // When: save_to_file is called
        let saved_buffer = super::save_to_file(&buffer, &file_path).unwrap();

        // Then: the file on disk contains the updated content
        let on_disk = fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, "Original content");

        // And: the returned buffer has modified=false
        assert!(!saved_buffer.is_modified());

        // And: content is preserved in the buffer
        assert_eq!(super::content(&saved_buffer), "Original content");
    }

    // -----------------------------------------------------------------------
    // Unit tests: save_to_file behaviors
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_when_saved_to_nonexistent_directory_then_returns_error() {
        // Given: a buffer and a path in a directory that does not exist
        let buffer = super::Buffer::from_string("some text");
        let bad_path = std::path::Path::new("/tmp/nonexistent_dir_alfred_test/save.txt");

        // When: save_to_file is called
        let result = super::save_to_file(&buffer, bad_path);

        // Then: it returns an error
        assert!(result.is_err());
    }

    #[test]
    fn given_buffer_with_utf8_content_when_saved_then_file_preserves_encoding() {
        // Given: a buffer containing multi-byte UTF-8 characters
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("utf8_test.txt");
        let utf8_content = "Hello \u{1F600} world \u{00E9}\u{00E8}\u{00EA}";
        let buffer = super::Buffer::from_string(utf8_content);

        // When: saved to file
        let _saved = super::save_to_file(&buffer, &file_path).unwrap();

        // Then: the file content preserves UTF-8 encoding exactly
        let on_disk = fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, utf8_content);
    }

    #[test]
    fn given_unmodified_buffer_when_saved_then_modified_remains_false() {
        // Given: a freshly created buffer (not modified)
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unmodified_save.txt");
        let buffer = super::Buffer::from_string("clean");

        // Precondition: buffer is not modified
        assert!(!buffer.is_modified());

        // When: saved to file
        let saved_buffer = super::save_to_file(&buffer, &file_path).unwrap();

        // Then: modified is still false
        assert!(!saved_buffer.is_modified());
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-03): get_line_content
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_get_line_content_then_returns_content_without_newline() {
        let buffer = super::Buffer::from_string("Hello\nWorld\n");
        assert_eq!(super::get_line_content(&buffer, 0), "Hello");
        assert_eq!(super::get_line_content(&buffer, 1), "World");
    }

    #[test]
    fn given_buffer_when_get_line_content_out_of_bounds_then_returns_empty() {
        let buffer = super::Buffer::from_string("Only line");
        assert_eq!(super::get_line_content(&buffer, 99), "");
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-03): join_lines
    // -----------------------------------------------------------------------

    #[test]
    fn given_two_lines_when_join_lines_then_lines_merged_with_space() {
        let buffer = super::Buffer::from_string("Hello\nWorld");
        let result = super::join_lines(&buffer, 0);
        assert_eq!(super::content(&result), "Hello World");
    }

    #[test]
    fn given_three_lines_when_join_first_then_first_two_merged_third_intact() {
        let buffer = super::Buffer::from_string("One\nTwo\nThree");
        let result = super::join_lines(&buffer, 0);
        assert_eq!(super::content(&result), "One Two\nThree");
    }

    #[test]
    fn given_last_line_when_join_lines_then_buffer_unchanged() {
        let buffer = super::Buffer::from_string("Hello\nWorld");
        let result = super::join_lines(&buffer, 1);
        assert_eq!(super::content(&result), "Hello\nWorld");
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-03): replace_line
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_buffer_when_replace_line_then_line_replaced() {
        let buffer = super::Buffer::from_string("First\nSecond\nThird");
        let result = super::replace_line(&buffer, 1, "Replaced");
        assert_eq!(super::content(&result), "First\nReplaced\nThird");
    }

    #[test]
    fn given_buffer_when_replace_last_line_no_trailing_newline_then_replaced() {
        let buffer = super::Buffer::from_string("First\nSecond");
        let result = super::replace_line(&buffer, 1, "New");
        assert_eq!(super::content(&result), "First\nNew");
    }

    // -----------------------------------------------------------------------
    // Unit tests (09-03): delete_to_line_end
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_when_delete_to_end_from_column_then_text_after_column_removed() {
        let buffer = super::Buffer::from_string("Hello World\nSecond");
        let result = super::delete_to_line_end(&buffer, 0, 5);
        assert_eq!(super::content(&result), "Hello\nSecond");
    }

    #[test]
    fn given_line_when_delete_to_end_from_start_then_line_becomes_empty() {
        let buffer = super::Buffer::from_string("Hello\nWorld");
        let result = super::delete_to_line_end(&buffer, 0, 0);
        assert_eq!(super::content(&result), "\nWorld");
    }

    // -----------------------------------------------------------------------
    // Unit tests: find_forward
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_when_find_forward_on_same_line_then_returns_match_position() {
        let buffer = super::Buffer::from_string("Hello World");
        let result = super::find_forward(&buffer, 0, 0, "World");
        assert_eq!(result, Some((0, 6)));
    }

    #[test]
    fn given_buffer_when_find_forward_on_next_line_then_returns_match_on_next_line() {
        let buffer = super::Buffer::from_string("Hello\nWorld here");
        let result = super::find_forward(&buffer, 0, 0, "World");
        assert_eq!(result, Some((1, 0)));
    }

    #[test]
    fn given_buffer_when_find_forward_wraps_around_then_returns_match_before_start() {
        let buffer = super::Buffer::from_string("Target line\nSecond line");
        // Start searching from line 1, col 0 — "Target" is before our position
        let result = super::find_forward(&buffer, 1, 0, "Target");
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn given_buffer_when_find_forward_no_match_then_returns_none() {
        let buffer = super::Buffer::from_string("Hello\nWorld");
        let result = super::find_forward(&buffer, 0, 0, "Missing");
        assert_eq!(result, None);
    }

    #[test]
    fn given_buffer_when_find_forward_starts_after_current_position_then_skips_current() {
        // Cursor is at the start of "Hello", searching for "Hello" should
        // start searching from col+1, so it wraps and finds the same "Hello"
        // only if it wraps around. With one occurrence, it should still find it
        // by wrapping.
        let buffer = super::Buffer::from_string("Hello World\nHello Again");
        // Start at (0, 0) — should find the next "Hello" at (1, 0)
        let result = super::find_forward(&buffer, 0, 0, "Hello");
        assert_eq!(result, Some((1, 0)));
    }

    // -----------------------------------------------------------------------
    // Unit tests: find_backward
    // -----------------------------------------------------------------------

    #[test]
    fn given_buffer_when_find_backward_on_same_line_then_returns_earlier_match() {
        let buffer = super::Buffer::from_string("Hello World Hello");
        // Start from col 12, searching backward should find "Hello" at col 0
        let result = super::find_backward(&buffer, 0, 12, "Hello");
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn given_buffer_when_find_backward_on_previous_line_then_returns_match() {
        let buffer = super::Buffer::from_string("First line\nSecond line");
        let result = super::find_backward(&buffer, 1, 5, "First");
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn given_buffer_when_find_backward_wraps_around_then_returns_match_after_start() {
        let buffer = super::Buffer::from_string("First line\nTarget here");
        // Start at (0, 5), searching backward for "Target" — wraps to end of buffer
        let result = super::find_backward(&buffer, 0, 5, "Target");
        assert_eq!(result, Some((1, 0)));
    }

    #[test]
    fn given_buffer_when_find_backward_no_match_then_returns_none() {
        let buffer = super::Buffer::from_string("Hello\nWorld");
        let result = super::find_backward(&buffer, 1, 5, "Missing");
        assert_eq!(result, None);
    }

    // -----------------------------------------------------------------------
    // Unit tests: indent_line
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_line_when_indent_then_line_becomes_indent_string() {
        let buffer = super::Buffer::from_string("");
        let result = super::indent_line(&buffer, 0, "    ");
        assert_eq!(super::content(&result), "    ");
    }

    #[test]
    fn given_line_with_content_when_indent_then_indent_prepended() {
        let buffer = super::Buffer::from_string("hello\nworld");
        let result = super::indent_line(&buffer, 0, "    ");
        assert_eq!(super::content(&result), "    hello\nworld");
    }

    #[test]
    fn given_out_of_bounds_line_when_indent_then_buffer_unchanged() {
        let buffer = super::Buffer::from_string("hello");
        let result = super::indent_line(&buffer, 99, "    ");
        assert_eq!(super::content(&result), "hello");
    }

    // -----------------------------------------------------------------------
    // Unit tests: unindent_line
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_with_4_spaces_when_unindent_by_4_then_spaces_removed() {
        let buffer = super::Buffer::from_string("    hello");
        let result = super::unindent_line(&buffer, 0, 4);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_line_with_2_spaces_when_unindent_by_4_then_only_2_removed() {
        let buffer = super::Buffer::from_string("  hello");
        let result = super::unindent_line(&buffer, 0, 4);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_line_with_no_spaces_when_unindent_then_no_change() {
        let buffer = super::Buffer::from_string("hello");
        let result = super::unindent_line(&buffer, 0, 4);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_out_of_bounds_line_when_unindent_then_buffer_unchanged() {
        let buffer = super::Buffer::from_string("hello");
        let result = super::unindent_line(&buffer, 99, 4);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_line_with_tab_when_unindent_then_tab_removed() {
        let buffer = super::Buffer::from_string("\thello");
        let result = super::unindent_line(&buffer, 0, 4);
        assert_eq!(super::content(&result), "hello");
    }

    // -----------------------------------------------------------------------
    // delete_char_range tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_single_line_when_delete_char_range_within_line_then_chars_removed() {
        let buffer = super::Buffer::from_string("hello world");
        // Delete "hello " (cols 0..6)
        let result = super::delete_char_range(&buffer, 0, 0, 0, 6);
        assert_eq!(super::content(&result), "world");
    }

    #[test]
    fn given_single_line_when_delete_char_range_from_middle_to_end_then_tail_removed() {
        let buffer = super::Buffer::from_string("hello world");
        // Delete " world" (cols 5..11)
        let result = super::delete_char_range(&buffer, 0, 5, 0, 11);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_multiline_when_delete_char_range_across_lines_then_range_removed() {
        let buffer = super::Buffer::from_string("hello\nworld\nbye");
        // Delete from (0,3) to (1,3): "lo\nwor"
        let result = super::delete_char_range(&buffer, 0, 3, 1, 3);
        assert_eq!(super::content(&result), "helld\nbye");
    }

    #[test]
    fn given_buffer_when_delete_char_range_same_position_then_unchanged() {
        let buffer = super::Buffer::from_string("hello");
        let result = super::delete_char_range(&buffer, 0, 3, 0, 3);
        assert_eq!(super::content(&result), "hello");
    }

    #[test]
    fn given_buffer_when_delete_char_range_start_past_end_then_unchanged() {
        let buffer = super::Buffer::from_string("hello");
        let result = super::delete_char_range(&buffer, 0, 5, 0, 3);
        assert_eq!(super::content(&result), "hello");
    }
}
