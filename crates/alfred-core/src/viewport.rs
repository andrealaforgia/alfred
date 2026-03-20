//! Viewport: visible window into a buffer and pure scrolling logic.
//!
//! A Viewport tracks which portion of a buffer is currently visible on screen.
//! The `adjust` function is pure: given a Viewport and a Cursor, it returns a
//! new Viewport that guarantees the cursor is visible.

use crate::cursor::Cursor;

/// The visible window into a buffer.
///
/// - `top_line`: the first visible line (zero-indexed)
/// - `height`: number of visible lines
/// - `width`: number of visible columns
/// - `gutter_width`: columns reserved for the gutter (0 in M1, set by line-numbers plugin in M4)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub top_line: usize,
    pub height: u16,
    pub width: u16,
    pub gutter_width: u16,
}

/// Creates a new Viewport with the given dimensions and gutter_width initialized to 0.
pub fn new(top_line: usize, height: u16, width: u16) -> Viewport {
    Viewport {
        top_line,
        height,
        width,
        gutter_width: 0,
    }
}

/// Returns a new Viewport adjusted so that the given cursor is visible.
///
/// - If the cursor is above the viewport, scroll up (set top_line to cursor line).
/// - If the cursor is below the viewport, scroll down (set top_line so cursor is on last visible line).
/// - If the cursor is already within the viewport, return it unchanged.
pub fn adjust(viewport: Viewport, cursor: &Cursor) -> Viewport {
    if cursor.line < viewport.top_line {
        Viewport {
            top_line: cursor.line,
            ..viewport
        }
    } else if cursor.line >= viewport.top_line + viewport.height as usize {
        Viewport {
            top_line: cursor.line - (viewport.height as usize - 1),
            ..viewport
        }
    } else {
        viewport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Acceptance test: viewport scrolls to keep cursor visible
    // -----------------------------------------------------------------------

    #[test]
    fn given_viewport_and_cursor_when_cursor_moves_beyond_boundaries_then_viewport_adjusts_to_keep_cursor_visible() {
        // Given: a viewport with top_line=0 and height=24
        let viewport = new(0, 24, 80);

        // When: cursor is at line 25 (beyond viewport bottom)
        let cursor = Cursor { line: 25, column: 0 };
        let adjusted = adjust(viewport, &cursor);

        // Then: top_line adjusts so cursor is on the last visible line
        assert_eq!(adjusted.top_line, 2); // 25 - (24 - 1) = 2
        assert!(cursor.line >= adjusted.top_line);
        assert!(cursor.line < adjusted.top_line + adjusted.height as usize);

        // When: cursor is at line 0 but top_line is 10
        let viewport_scrolled = new(10, 24, 80);
        let cursor_at_top = Cursor { line: 0, column: 0 };
        let adjusted = adjust(viewport_scrolled, &cursor_at_top);

        // Then: top_line adjusts to 0
        assert_eq!(adjusted.top_line, 0);

        // When: cursor is within the viewport
        let viewport = new(5, 24, 80);
        let cursor_within = Cursor { line: 15, column: 3 };
        let adjusted = adjust(viewport, &cursor_within);

        // Then: viewport does not change
        assert_eq!(adjusted, viewport);

        // Viewport adjustment is pure: original viewport is unchanged
        assert_eq!(viewport.top_line, 5);
        assert_eq!(viewport.height, 24);
    }

    // -----------------------------------------------------------------------
    // Unit tests: scroll down
    // -----------------------------------------------------------------------

    #[test]
    fn cursor_below_viewport_causes_scroll_down() {
        let viewport = new(0, 24, 80);
        let cursor = Cursor { line: 24, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted.top_line, 1); // 24 - 23 = 1
    }

    #[test]
    fn cursor_far_below_viewport_scrolls_to_correct_position() {
        let viewport = new(0, 10, 80);
        let cursor = Cursor { line: 50, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted.top_line, 41); // 50 - 9 = 41
        assert!(cursor.line < adjusted.top_line + adjusted.height as usize);
    }

    // -----------------------------------------------------------------------
    // Unit tests: scroll up
    // -----------------------------------------------------------------------

    #[test]
    fn cursor_above_viewport_causes_scroll_up() {
        let viewport = new(10, 24, 80);
        let cursor = Cursor { line: 5, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted.top_line, 5);
    }

    #[test]
    fn cursor_at_line_zero_with_scrolled_viewport_scrolls_to_top() {
        let viewport = new(15, 24, 80);
        let cursor = Cursor { line: 0, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted.top_line, 0);
    }

    // -----------------------------------------------------------------------
    // Unit tests: cursor within viewport (no change)
    // -----------------------------------------------------------------------

    #[test]
    fn cursor_within_viewport_causes_no_change() {
        let viewport = new(5, 24, 80);
        let cursor = Cursor { line: 20, column: 10 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted, viewport);
    }

    #[test]
    fn cursor_at_top_line_causes_no_change() {
        let viewport = new(5, 24, 80);
        let cursor = Cursor { line: 5, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted, viewport);
    }

    #[test]
    fn cursor_at_last_visible_line_causes_no_change() {
        let viewport = new(5, 24, 80);
        // Last visible line = 5 + 24 - 1 = 28
        let cursor = Cursor { line: 28, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted, viewport);
    }

    // -----------------------------------------------------------------------
    // Unit tests: gutter_width default
    // -----------------------------------------------------------------------

    #[test]
    fn gutter_width_initialized_to_zero() {
        let viewport = new(0, 24, 80);
        assert_eq!(viewport.gutter_width, 0);
    }

    // -----------------------------------------------------------------------
    // Unit tests: purity (returns new Viewport, does not mutate)
    // -----------------------------------------------------------------------

    #[test]
    fn adjust_returns_new_viewport_without_mutating_original() {
        let original = new(0, 24, 80);
        let cursor = Cursor { line: 30, column: 0 };
        let adjusted = adjust(original, &cursor);

        // original is unchanged (Copy type, but semantically verifying purity)
        assert_eq!(original.top_line, 0);
        assert_ne!(adjusted.top_line, original.top_line);
    }

    // -----------------------------------------------------------------------
    // Unit tests: dimensions preserved across adjustment
    // -----------------------------------------------------------------------

    #[test]
    fn adjust_preserves_viewport_dimensions() {
        let viewport = new(0, 24, 80);
        let cursor = Cursor { line: 30, column: 0 };
        let adjusted = adjust(viewport, &cursor);
        assert_eq!(adjusted.height, 24);
        assert_eq!(adjusted.width, 80);
        assert_eq!(adjusted.gutter_width, 0);
    }
}
