//! Rainbow CSV: pure functions for CSV column colorization.
//!
//! This module computes per-line style segments for CSV files,
//! assigning a rotating color palette to each comma-separated column.
//! It has no I/O dependencies -- all functions are pure transformations.

use crate::buffer;
use crate::editor_state::{self, EditorState};
use crate::theme::ThemeColor;

/// The rainbow color palette: 8 distinct pastel colors cycling per column.
const PALETTE: [ThemeColor; 8] = [
    ThemeColor::Rgb(255, 121, 121), // pastel red
    ThemeColor::Rgb(253, 203, 110), // pastel yellow
    ThemeColor::Rgb(85, 239, 196),  // pastel green
    ThemeColor::Rgb(116, 185, 255), // pastel blue
    ThemeColor::Rgb(162, 155, 254), // pastel purple
    ThemeColor::Rgb(255, 159, 243), // pastel pink
    ThemeColor::Rgb(0, 206, 209),   // dark turquoise
    ThemeColor::Rgb(255, 185, 118), // pastel orange
];

/// Computes column-color segments for a single CSV line.
///
/// Splits the line by `delimiter` and returns a Vec of (start_col, end_col, ThemeColor)
/// segments where each column gets a color from the rotating palette.
///
/// Returns an empty Vec for empty lines.
pub fn compute_line_segments(line: &str, delimiter: char) -> Vec<(usize, usize, ThemeColor)> {
    if line.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut pos = 0usize;

    for (col_index, field) in line.split(delimiter).enumerate() {
        let start = pos;
        let end = pos + field.len();
        let color = PALETTE[col_index % PALETTE.len()];
        segments.push((start, end, color));
        // Move past the field and the delimiter
        pos = end + 1; // +1 for the delimiter character
    }

    segments
}

/// Colorizes the entire buffer as CSV, populating `line_styles` in EditorState.
///
/// Clears existing line styles, then for each non-empty line in the buffer,
/// computes column segments using comma as delimiter and stores them.
pub fn colorize_buffer(state: &mut EditorState) {
    editor_state::clear_line_styles(state);

    let total_lines = buffer::line_count(&state.buffer);
    for line_idx in 0..total_lines {
        let line_content = buffer::get_line(&state.buffer, line_idx)
            .unwrap_or("")
            .trim_end_matches('\n');
        let segments = compute_line_segments(line_content, ',');
        for (start, end, color) in segments {
            editor_state::add_line_style(state, line_idx, start, end, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::editor_state;

    // -----------------------------------------------------------------------
    // Acceptance test: colorize CSV buffer via EditorState driving port
    // Test Budget: 5 behaviors x 2 = 10 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_csv_buffer_when_colorize_then_each_line_has_colored_segments_per_column() {
        // Given: an editor state with a 2-line CSV buffer
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("name,age,city\nalice,30,london");

        // When: the buffer is colorized
        colorize_buffer(&mut state);

        // Then: line 0 has 3 segments (name, age, city)
        let line0_styles = state
            .line_styles
            .get(&0)
            .expect("line 0 should have styles");
        assert_eq!(line0_styles.len(), 3, "3 columns = 3 segments");

        // And: the segments cover the correct column ranges
        assert_eq!(line0_styles[0].0, 0); // "name" starts at 0
        assert_eq!(line0_styles[0].1, 4); // "name" ends at 4
        assert_eq!(line0_styles[1].0, 5); // "age" starts at 5
        assert_eq!(line0_styles[1].1, 8); // "age" ends at 8
        assert_eq!(line0_styles[2].0, 9); // "city" starts at 9
        assert_eq!(line0_styles[2].1, 13); // "city" ends at 13

        // And: each segment has a different color (from palette)
        assert_ne!(line0_styles[0].2, line0_styles[1].2);
        assert_ne!(line0_styles[1].2, line0_styles[2].2);

        // And: line 1 also has 3 segments
        let line1_styles = state
            .line_styles
            .get(&1)
            .expect("line 1 should have styles");
        assert_eq!(line1_styles.len(), 3);

        // And: column 0 on line 1 uses the same color as column 0 on line 0
        assert_eq!(line0_styles[0].2, line1_styles[0].2);
    }

    // -----------------------------------------------------------------------
    // Unit tests: compute_line_segments pure function
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_line_when_compute_segments_then_returns_empty() {
        let segments = compute_line_segments("", ',');
        assert!(segments.is_empty());
    }

    #[test]
    fn given_line_without_delimiter_when_compute_segments_then_single_segment_covering_entire_line()
    {
        let segments = compute_line_segments("hello", ',');
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, 0);
        assert_eq!(segments[0].1, 5);
        assert_eq!(segments[0].2, PALETTE[0]);
    }

    #[test]
    fn given_csv_line_when_compute_segments_then_each_column_gets_rotating_palette_color() {
        let segments = compute_line_segments("a,bb,ccc", ',');
        assert_eq!(segments.len(), 3);
        // Column boundaries
        assert_eq!((segments[0].0, segments[0].1), (0, 1)); // "a"
        assert_eq!((segments[1].0, segments[1].1), (2, 4)); // "bb"
        assert_eq!((segments[2].0, segments[2].1), (5, 8)); // "ccc"
                                                            // Colors rotate through palette
        assert_eq!(segments[0].2, PALETTE[0]);
        assert_eq!(segments[1].2, PALETTE[1]);
        assert_eq!(segments[2].2, PALETTE[2]);
    }

    #[test]
    fn given_colorized_buffer_when_clear_line_styles_then_styles_are_empty() {
        let mut state = editor_state::new(80, 24);
        state.buffer = Buffer::from_string("a,b,c");
        colorize_buffer(&mut state);
        assert!(!state.line_styles.is_empty());

        editor_state::clear_line_styles(&mut state);
        assert!(state.line_styles.is_empty());
    }

    #[test]
    fn given_more_columns_than_palette_size_when_compute_segments_then_colors_cycle() {
        // 9 columns should cycle back to PALETTE[0] for the 9th
        let line = "a,b,c,d,e,f,g,h,i";
        let segments = compute_line_segments(line, ',');
        assert_eq!(segments.len(), 9);
        assert_eq!(segments[8].2, PALETTE[0]); // 9th column wraps to index 0
    }
}
