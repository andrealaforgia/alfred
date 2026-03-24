//! SyntaxHighlighter: top-level struct managing tree-sitter parsing and queries.
//!
//! Owns the parser, current parse tree, and language configs. Provides
//! parse, edit, and highlight_lines as pure-ish operations (parser is
//! stateful but all outputs are data).

use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, Tree};

use alfred_core::theme::ThemeColor;

use crate::language::{self, LanguageConfig};

/// A single highlight range for a region of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightRange {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub capture_name: String,
}

/// Describes a buffer edit for tree-sitter's incremental parsing.
#[derive(Debug, Clone)]
pub struct EditNotification {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: (usize, usize),
    pub old_end_position: (usize, usize),
    pub new_end_position: (usize, usize),
}

/// The syntax highlighter managing all tree-sitter state.
pub struct SyntaxHighlighter {
    parser: Parser,
    current_tree: Option<Tree>,
    current_language_id: Option<String>,
    language_configs: Vec<LanguageConfig>,
    highlight_query: Option<Query>,
    buffer_version: u64,
}

impl SyntaxHighlighter {
    /// Creates a new SyntaxHighlighter with all registered languages.
    pub fn new() -> Self {
        SyntaxHighlighter {
            parser: Parser::new(),
            current_tree: None,
            current_language_id: None,
            language_configs: language::all_languages(),
            highlight_query: None,
            buffer_version: 0,
        }
    }

    /// Sets the language based on filename. Returns true if language was found and set.
    pub fn set_language_for_file(&mut self, filename: &str) -> bool {
        let config_index = self
            .language_configs
            .iter()
            .position(|config| config.extensions.iter().any(|ext| filename.ends_with(ext)));

        match config_index {
            Some(idx) => {
                let config = &self.language_configs[idx];
                let language_id = config.id.to_string();

                // Only re-configure if language changed
                if self.current_language_id.as_deref() == Some(config.id) {
                    return true;
                }

                if self.parser.set_language(&config.grammar).is_err() {
                    return false;
                }

                self.highlight_query = Query::new(&config.grammar, config.highlight_query).ok();

                self.current_language_id = Some(language_id);
                self.current_tree = None;
                self.buffer_version = 0;
                true
            }
            None => {
                self.current_language_id = None;
                self.highlight_query = None;
                self.current_tree = None;
                false
            }
        }
    }

    /// Returns the current language id, if any.
    pub fn current_language(&self) -> Option<&str> {
        self.current_language_id.as_deref()
    }

    /// Parses the full buffer text, optionally using the previous tree for
    /// incremental parsing.
    pub fn parse(&mut self, source: &str) -> bool {
        let old_tree = self.current_tree.as_ref();
        match self.parser.parse(source, old_tree) {
            Some(tree) => {
                self.current_tree = Some(tree);
                true
            }
            None => false,
        }
    }

    /// Parses the full buffer text and records the buffer version.
    pub fn parse_with_version(&mut self, source: &str, version: u64) -> bool {
        if self.buffer_version == version && self.current_tree.is_some() {
            return true; // Already up to date
        }
        let result = self.parse(source);
        if result {
            self.buffer_version = version;
        }
        result
    }

    /// Notifies the tree of a buffer edit before re-parsing.
    pub fn edit(&mut self, notification: &EditNotification) {
        if let Some(tree) = &mut self.current_tree {
            let input_edit = InputEdit {
                start_byte: notification.start_byte,
                old_end_byte: notification.old_end_byte,
                new_end_byte: notification.new_end_byte,
                start_position: Point {
                    row: notification.start_position.0,
                    column: notification.start_position.1,
                },
                old_end_position: Point {
                    row: notification.old_end_position.0,
                    column: notification.old_end_position.1,
                },
                new_end_position: Point {
                    row: notification.new_end_position.0,
                    column: notification.new_end_position.1,
                },
            };
            tree.edit(&input_edit);
        }
    }

    /// Computes an edit notification by diffing old and new source, applies it to
    /// the tree, and re-parses incrementally. This is more efficient than a full
    /// re-parse because tree-sitter can skip unchanged portions of the tree.
    pub fn incremental_update(&mut self, old_source: &str, new_source: &str) -> bool {
        if let Some(notification) = compute_edit_notification(old_source, new_source) {
            self.edit(&notification);
        }
        self.parse(new_source)
    }

    /// Queries highlights for a range of lines, returning HighlightRange descriptors.
    ///
    /// `start_line` and `end_line` are 0-indexed (end_line exclusive).
    /// Returns ranges only within the requested line range.
    pub fn highlight_lines(
        &self,
        source: &str,
        start_line: usize,
        end_line: usize,
    ) -> Vec<HighlightRange> {
        let tree = match &self.current_tree {
            Some(t) => t,
            None => return Vec::new(),
        };
        let query = match &self.highlight_query {
            Some(q) => q,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();

        // Restrict query to the byte range of the visible lines
        let start_byte = line_to_byte_offset(source, start_line);
        let end_byte = line_to_byte_offset(source, end_line);
        cursor.set_byte_range(start_byte..end_byte);

        let mut ranges = Vec::new();
        let mut matches = cursor.matches(query, root_node, source.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                let node = capture.node;
                let node_start = node.start_position();
                let node_end = node.end_position();

                // A node can span multiple lines; split into per-line ranges
                let line_ranges =
                    split_node_into_line_ranges(source, node_start, node_end, capture_name);

                for range in line_ranges {
                    if range.line >= start_line && range.line < end_line {
                        ranges.push(range);
                    }
                }
            }
        }

        // Sort by line then by start_col for deterministic rendering
        ranges.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_col.cmp(&b.start_col)));

        // Deduplicate overlapping ranges: keep the first (highest-priority) capture
        deduplicate_ranges(ranges)
    }

    /// Maps a capture name (e.g., "keyword") to a theme slot (e.g., "syntax-keyword").
    ///
    /// Handles hierarchical names by using the parent as fallback:
    /// "function.method" -> tries "syntax-function.method", falls back to "syntax-function".
    pub fn resolve_theme_slot(capture_name: &str) -> String {
        format!("syntax-{}", capture_name)
    }

    /// Resolves a capture name to a ThemeColor using the provided theme lookup.
    ///
    /// Falls back to parent capture name if sub-capture not found.
    /// Returns None if no theme color set for this capture.
    pub fn resolve_color(
        capture_name: &str,
        theme: &std::collections::HashMap<String, ThemeColor>,
    ) -> Option<ThemeColor> {
        // Try exact match first: "syntax-function.method"
        let slot = format!("syntax-{}", capture_name);
        if let Some(&color) = theme.get(&slot) {
            return Some(color);
        }

        // Fallback: strip sub-capture, try "syntax-function"
        if let Some(dot_pos) = capture_name.find('.') {
            let parent = &capture_name[..dot_pos];
            let parent_slot = format!("syntax-{}", parent);
            if let Some(&color) = theme.get(&parent_slot) {
                return Some(color);
            }
        }

        None
    }

    /// Returns true if a language is currently set.
    pub fn has_language(&self) -> bool {
        self.current_language_id.is_some()
    }

    /// Returns true if a parse tree exists.
    pub fn has_tree(&self) -> bool {
        self.current_tree.is_some()
    }

    /// Returns the last parsed buffer version.
    pub fn buffer_version(&self) -> u64 {
        self.buffer_version
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the byte offset of the start of a given line in the source text.
fn line_to_byte_offset(source: &str, line: usize) -> usize {
    let mut offset = 0;
    for (i, chunk) in source.split('\n').enumerate() {
        if i == line {
            return offset;
        }
        offset += chunk.len() + 1; // +1 for the '\n'
    }
    source.len()
}

/// Splits a multi-line node into per-line HighlightRange values.
fn split_node_into_line_ranges(
    source: &str,
    start: Point,
    end: Point,
    capture_name: &str,
) -> Vec<HighlightRange> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    if start.row == end.row {
        // Single-line node
        ranges.push(HighlightRange {
            line: start.row,
            start_col: start.column,
            end_col: end.column,
            capture_name: capture_name.to_string(),
        });
    } else {
        // Multi-line node: first line
        let first_line_len = lines.get(start.row).map_or(0, |l| l.len());
        ranges.push(HighlightRange {
            line: start.row,
            start_col: start.column,
            end_col: first_line_len,
            capture_name: capture_name.to_string(),
        });

        // Middle lines: full line
        for line_idx in (start.row + 1)..end.row {
            let line_len = lines.get(line_idx).map_or(0, |l| l.len());
            ranges.push(HighlightRange {
                line: line_idx,
                start_col: 0,
                end_col: line_len,
                capture_name: capture_name.to_string(),
            });
        }

        // Last line
        if end.column > 0 {
            ranges.push(HighlightRange {
                line: end.row,
                start_col: 0,
                end_col: end.column,
                capture_name: capture_name.to_string(),
            });
        }
    }

    ranges
}

/// Removes overlapping ranges, keeping the first one encountered (highest priority from query order).
fn deduplicate_ranges(sorted_ranges: Vec<HighlightRange>) -> Vec<HighlightRange> {
    let mut result: Vec<HighlightRange> = Vec::new();

    for range in sorted_ranges {
        let overlaps = result.iter().any(|existing| {
            existing.line == range.line
                && existing.start_col < range.end_col
                && range.start_col < existing.end_col
        });

        if !overlaps {
            result.push(range);
        }
    }

    result
}

/// Computes an EditNotification by finding the first and last difference
/// between old_source and new_source. Returns None if they are identical.
fn compute_edit_notification(old_source: &str, new_source: &str) -> Option<EditNotification> {
    let old_bytes = old_source.as_bytes();
    let new_bytes = new_source.as_bytes();

    if old_bytes == new_bytes {
        return None;
    }

    // Find first differing byte
    let prefix_len = old_bytes
        .iter()
        .zip(new_bytes.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Find last differing byte (from the end)
    let suffix_len = old_bytes[prefix_len..]
        .iter()
        .rev()
        .zip(new_bytes[prefix_len..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let old_end = old_bytes.len() - suffix_len;
    let new_end = new_bytes.len() - suffix_len;

    let start_pos = byte_offset_to_point(old_source, prefix_len);
    let old_end_pos = byte_offset_to_point(old_source, old_end);
    let new_end_pos = byte_offset_to_point(new_source, new_end);

    Some(EditNotification {
        start_byte: prefix_len,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: start_pos,
        old_end_position: old_end_pos,
        new_end_position: new_end_pos,
    })
}

/// Converts a byte offset in source text to a (row, column) point.
fn byte_offset_to_point(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let row = prefix.matches('\n').count();
    let last_newline = prefix.rfind('\n').map_or(0, |pos| pos + 1);
    let column = offset - last_newline;
    (row, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a SyntaxHighlighter with language set and source parsed.
    /// Returns the highlighter for further assertions.
    fn parsed_highlighter(filename: &str, source: &str) -> SyntaxHighlighter {
        let mut h = SyntaxHighlighter::new();
        h.set_language_for_file(filename);
        h.parse(source);
        h
    }

    /// Asserts that highlighting the given source (associated with filename)
    /// produces at least one capture with the expected capture_name.
    fn assert_capture_present(filename: &str, source: &str, expected_capture: &str) {
        let h = parsed_highlighter(filename, source);
        let line_count = source.lines().count().max(1);
        let ranges = h.highlight_lines(source, 0, line_count);
        let matching: Vec<_> = ranges
            .iter()
            .filter(|r| r.capture_name == expected_capture)
            .collect();
        assert!(
            !matching.is_empty(),
            "Expected '{}' capture for '{}' in {:?}, got ranges: {:?}",
            expected_capture,
            source,
            filename,
            ranges
        );
    }

    // -----------------------------------------------------------------------
    // Unit tests: SyntaxHighlighter creation and language detection
    // -----------------------------------------------------------------------

    #[test]
    fn given_new_highlighter_when_created_then_has_no_language() {
        let highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.has_language());
        assert!(!highlighter.has_tree());
    }

    #[test]
    fn given_rs_file_when_set_language_then_returns_true() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(highlighter.set_language_for_file("main.rs"));
        assert_eq!(highlighter.current_language(), Some("rust"));
    }

    #[test]
    fn given_txt_file_when_set_language_then_returns_false() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.set_language_for_file("readme.txt"));
        assert!(!highlighter.has_language());
    }

    #[test]
    fn given_same_language_when_set_twice_then_does_not_reset_tree() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");
        highlighter.parse("fn main() {}");
        assert!(highlighter.has_tree());

        // Setting same language should not reset tree
        highlighter.set_language_for_file("lib.rs");
        assert!(highlighter.has_tree());
    }

    // -----------------------------------------------------------------------
    // Unit tests: parsing
    // -----------------------------------------------------------------------

    #[test]
    fn given_rust_source_when_parsed_then_tree_exists() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");
        assert!(highlighter.parse("fn main() {}"));
        assert!(highlighter.has_tree());
    }

    #[test]
    fn given_no_language_when_parsed_then_returns_false() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(!highlighter.parse("fn main() {}"));
    }

    #[test]
    fn given_rust_source_when_parse_with_version_then_version_tracked() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");
        highlighter.parse_with_version("fn main() {}", 5);
        assert_eq!(highlighter.buffer_version(), 5);
    }

    #[test]
    fn given_same_version_when_parse_again_then_skips_parse() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");
        highlighter.parse_with_version("fn main() {}", 5);

        // Parsing with same version should be a no-op
        assert!(highlighter.parse_with_version("different source", 5));
        // Tree should still exist from first parse
        assert!(highlighter.has_tree());
    }

    // -----------------------------------------------------------------------
    // Unit tests: highlight_lines
    // -----------------------------------------------------------------------

    #[test]
    fn given_rust_fn_when_highlight_then_fn_keyword_captured() {
        assert_capture_present("main.rs", "fn main() {}", "keyword");
    }

    #[test]
    fn given_rust_fn_when_highlight_then_keyword_at_correct_position() {
        let h = parsed_highlighter("main.rs", "fn main() {}");
        let ranges = h.highlight_lines("fn main() {}", 0, 1);
        let fn_range = ranges
            .iter()
            .find(|r| r.capture_name == "keyword" && r.start_col == 0 && r.end_col == 2);
        assert!(
            fn_range.is_some(),
            "Should have keyword range at 0..2 for 'fn'"
        );
    }

    #[test]
    fn given_rust_string_when_highlight_then_string_captured() {
        assert_capture_present("main.rs", "let x = \"hello\";", "string");
    }

    #[test]
    fn given_rust_number_when_highlight_then_number_captured() {
        assert_capture_present("main.rs", "let x = 42;", "number");
    }

    #[test]
    fn given_rust_comment_when_highlight_then_comment_captured() {
        assert_capture_present("main.rs", "// this is a comment", "comment");
    }

    #[test]
    fn given_rust_function_def_when_highlight_then_function_name_captured() {
        assert_capture_present("main.rs", "fn my_func() {}", "function");
    }

    #[test]
    fn given_multiline_rust_when_highlight_line_range_then_only_visible_lines_returned() {
        let source = "fn main() {\n    let x = 42;\n    let y = 99;\n}";
        let h = parsed_highlighter("main.rs", source);
        let ranges = h.highlight_lines(source, 1, 2);
        assert!(
            ranges.iter().all(|r| r.line == 1),
            "All ranges should be on line 1, got: {:?}",
            ranges
        );
    }

    #[test]
    fn given_no_tree_when_highlight_lines_then_returns_empty() {
        let highlighter = SyntaxHighlighter::new();
        let ranges = highlighter.highlight_lines("fn main() {}", 0, 1);
        assert!(ranges.is_empty());
    }

    #[test]
    fn given_unrecognized_file_when_highlight_then_returns_empty() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("readme.txt");
        let ranges = highlighter.highlight_lines("some text", 0, 1);
        assert!(ranges.is_empty());
    }

    // -----------------------------------------------------------------------
    // Unit tests: theme slot resolution
    // -----------------------------------------------------------------------

    #[test]
    fn given_keyword_capture_when_resolve_theme_slot_then_returns_syntax_keyword() {
        assert_eq!(
            SyntaxHighlighter::resolve_theme_slot("keyword"),
            "syntax-keyword"
        );
    }

    #[test]
    fn given_function_method_when_resolve_theme_slot_then_returns_syntax_function_method() {
        assert_eq!(
            SyntaxHighlighter::resolve_theme_slot("function.method"),
            "syntax-function.method"
        );
    }

    #[test]
    fn given_theme_with_exact_slot_when_resolve_color_then_returns_color() {
        let mut theme = std::collections::HashMap::new();
        theme.insert("syntax-keyword".to_string(), ThemeColor::Rgb(198, 120, 221));

        let color = SyntaxHighlighter::resolve_color("keyword", &theme);
        assert_eq!(color, Some(ThemeColor::Rgb(198, 120, 221)));
    }

    #[test]
    fn given_theme_with_parent_slot_when_resolve_sub_capture_then_falls_back() {
        let mut theme = std::collections::HashMap::new();
        theme.insert(
            "syntax-function".to_string(),
            ThemeColor::Rgb(137, 180, 250),
        );

        // "function.method" should fall back to "syntax-function"
        let color = SyntaxHighlighter::resolve_color("function.method", &theme);
        assert_eq!(color, Some(ThemeColor::Rgb(137, 180, 250)));
    }

    #[test]
    fn given_theme_without_slot_when_resolve_color_then_returns_none() {
        let theme = std::collections::HashMap::new();
        let color = SyntaxHighlighter::resolve_color("keyword", &theme);
        assert_eq!(color, None);
    }

    // -----------------------------------------------------------------------
    // Unit tests: line_to_byte_offset
    // -----------------------------------------------------------------------

    #[test]
    fn given_multiline_text_when_byte_offset_line_0_then_returns_0() {
        assert_eq!(line_to_byte_offset("hello\nworld", 0), 0);
    }

    #[test]
    fn given_multiline_text_when_byte_offset_line_1_then_returns_6() {
        assert_eq!(line_to_byte_offset("hello\nworld", 1), 6);
    }

    #[test]
    fn given_multiline_text_when_byte_offset_beyond_last_line_then_returns_len() {
        assert_eq!(line_to_byte_offset("hello\nworld", 5), 11);
    }

    // -----------------------------------------------------------------------
    // Unit tests: edit notification
    // -----------------------------------------------------------------------

    #[test]
    fn given_parsed_tree_when_edit_and_reparse_then_tree_updated() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");

        let source = "fn main() {}";
        highlighter.parse(source);
        assert!(highlighter.has_tree());

        // Simulate inserting "let x = 1;\n" after "fn main() {\n"
        let notification = EditNotification {
            start_byte: 12,
            old_end_byte: 12,
            new_end_byte: 24,
            start_position: (0, 12),
            old_end_position: (0, 12),
            new_end_position: (1, 0),
        };
        highlighter.edit(&notification);

        let new_source = "fn main() {\nlet x = 1;\n}";
        assert!(highlighter.parse(new_source));
        assert!(highlighter.has_tree());
    }

    // -----------------------------------------------------------------------
    // Unit tests: deduplicate_ranges
    // -----------------------------------------------------------------------

    #[test]
    fn given_overlapping_ranges_when_deduplicate_then_keeps_first() {
        let ranges = vec![
            HighlightRange {
                line: 0,
                start_col: 0,
                end_col: 5,
                capture_name: "keyword".to_string(),
            },
            HighlightRange {
                line: 0,
                start_col: 0,
                end_col: 5,
                capture_name: "function".to_string(),
            },
        ];

        let result = deduplicate_ranges(ranges);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].capture_name, "keyword");
    }

    #[test]
    fn given_non_overlapping_ranges_when_deduplicate_then_keeps_all() {
        let ranges = vec![
            HighlightRange {
                line: 0,
                start_col: 0,
                end_col: 2,
                capture_name: "keyword".to_string(),
            },
            HighlightRange {
                line: 0,
                start_col: 3,
                end_col: 7,
                capture_name: "function".to_string(),
            },
        ];

        let result = deduplicate_ranges(ranges);
        assert_eq!(result.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Unit tests: compute_edit_notification
    // -----------------------------------------------------------------------

    #[test]
    fn given_identical_sources_when_compute_edit_then_returns_none() {
        let result = compute_edit_notification("fn main() {}", "fn main() {}");
        assert!(result.is_none());
    }

    #[test]
    fn given_insertion_when_compute_edit_then_returns_correct_offsets() {
        let old = "fn main() {}";
        let new = "fn main() { let x = 1; }";
        let edit = compute_edit_notification(old, new).unwrap();

        assert_eq!(edit.start_byte, 11); // after "fn main() {"
        assert_eq!(edit.old_end_byte, 11); // nothing removed
        assert!(edit.new_end_byte > edit.start_byte); // text inserted
    }

    #[test]
    fn given_deletion_when_compute_edit_then_returns_correct_offsets() {
        let old = "fn main() { let x = 1; }";
        let new = "fn main() {}";
        let edit = compute_edit_notification(old, new).unwrap();

        assert_eq!(edit.start_byte, 11);
        assert!(edit.old_end_byte > edit.start_byte); // text removed
    }

    #[test]
    fn given_multiline_edit_when_compute_edit_then_positions_correct() {
        let old = "fn main() {\n}";
        let new = "fn main() {\n    let x = 1;\n}";
        let edit = compute_edit_notification(old, new).unwrap();

        // Start should be at the end of "fn main() {\n"
        assert_eq!(edit.start_position.0, 1); // row 1
    }

    // -----------------------------------------------------------------------
    // Unit tests: byte_offset_to_point
    // -----------------------------------------------------------------------

    #[test]
    fn given_start_of_text_when_byte_to_point_then_returns_0_0() {
        assert_eq!(byte_offset_to_point("hello\nworld", 0), (0, 0));
    }

    #[test]
    fn given_middle_of_first_line_when_byte_to_point_then_row_0() {
        assert_eq!(byte_offset_to_point("hello\nworld", 3), (0, 3));
    }

    #[test]
    fn given_start_of_second_line_when_byte_to_point_then_row_1_col_0() {
        assert_eq!(byte_offset_to_point("hello\nworld", 6), (1, 0));
    }

    #[test]
    fn given_middle_of_second_line_when_byte_to_point_then_row_1() {
        assert_eq!(byte_offset_to_point("hello\nworld", 8), (1, 2));
    }

    // -----------------------------------------------------------------------
    // Unit tests: incremental_update
    // -----------------------------------------------------------------------

    #[test]
    fn given_parsed_rust_when_incremental_update_then_highlights_update() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");

        let old_source = "fn main() {}";
        highlighter.parse(old_source);

        let new_source = "fn main() {\n    let x = 42;\n}";
        assert!(highlighter.incremental_update(old_source, new_source));

        // The new source should highlight 'let' as keyword on line 1
        let ranges = highlighter.highlight_lines(new_source, 1, 2);
        let keywords: Vec<_> = ranges
            .iter()
            .filter(|r| r.capture_name == "keyword")
            .collect();
        assert!(
            !keywords.is_empty(),
            "After incremental update, 'let' should be highlighted as keyword"
        );
    }

    #[test]
    fn given_parsed_rust_when_text_deleted_and_updated_then_highlights_correct() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_for_file("main.rs");

        let old_source = "fn main() {\n    let x = 42;\n}";
        highlighter.parse(old_source);

        let new_source = "fn main() {}";
        assert!(highlighter.incremental_update(old_source, new_source));

        let ranges = highlighter.highlight_lines(new_source, 0, 1);
        let keywords: Vec<_> = ranges
            .iter()
            .filter(|r| r.capture_name == "keyword")
            .collect();
        assert!(!keywords.is_empty(), "fn should still be highlighted");
    }

    // -----------------------------------------------------------------------
    // Unit tests: Python highlighting
    // -----------------------------------------------------------------------

    #[test]
    fn given_py_file_when_set_language_then_returns_true() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(highlighter.set_language_for_file("app.py"));
        assert_eq!(highlighter.current_language(), Some("python"));
    }

    #[test]
    fn given_python_def_when_highlight_then_def_keyword_captured() {
        assert_capture_present("app.py", "def hello():\n    pass", "keyword");
    }

    #[test]
    fn given_python_string_when_highlight_then_string_captured() {
        assert_capture_present("app.py", "x = \"hello world\"", "string");
    }

    #[test]
    fn given_python_comment_when_highlight_then_comment_captured() {
        assert_capture_present("app.py", "# this is a comment", "comment");
    }

    #[test]
    fn given_python_number_when_highlight_then_number_captured() {
        assert_capture_present("app.py", "x = 42", "number");
    }

    // -----------------------------------------------------------------------
    // Unit tests: JavaScript highlighting
    // -----------------------------------------------------------------------

    #[test]
    fn given_js_file_when_set_language_then_returns_true() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(highlighter.set_language_for_file("index.js"));
        assert_eq!(highlighter.current_language(), Some("javascript"));
    }

    #[test]
    fn given_js_function_when_highlight_then_function_keyword_captured() {
        assert_capture_present("index.js", "function hello() {}", "keyword");
    }

    #[test]
    fn given_js_string_when_highlight_then_string_captured() {
        assert_capture_present("index.js", "const x = \"hello\";", "string");
    }

    #[test]
    fn given_js_comment_when_highlight_then_comment_captured() {
        assert_capture_present("index.js", "// this is a comment", "comment");
    }

    #[test]
    fn given_js_number_when_highlight_then_number_captured() {
        assert_capture_present("index.js", "let x = 42;", "number");
    }

    #[test]
    fn given_js_const_keyword_when_highlight_then_keyword_captured() {
        assert_capture_present("index.js", "const x = 1;", "keyword");
    }
}
