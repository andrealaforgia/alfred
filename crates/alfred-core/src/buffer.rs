//! Buffer: immutable text container wrapping ropey::Rope with metadata.
//!
//! Buffer is the core text-storage abstraction for the Alfred editor.
//! It wraps a `ropey::Rope` and carries metadata (id, filename, modified flag, version).
//! All operations are pure: modifications return new Buffer instances.

use std::path::Path;
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
/// - `modified`: whether the buffer has been changed since loading
/// - `version`: monotonically increasing version counter
#[derive(Debug, Clone)]
pub struct Buffer {
    id: u64,
    rope: Rope,
    filename: Option<String>,
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
        modified: true,
        version: buffer.version + 1,
    }
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
}
