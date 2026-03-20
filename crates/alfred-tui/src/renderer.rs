//! Renderer: terminal rendering of EditorState via ratatui.
//!
//! This module is the imperative shell -- it performs terminal I/O
//! using crossterm for raw mode and ratatui for immediate-mode rendering.
//! The renderer takes an &EditorState and produces a frame showing:
//! - Buffer content (visible lines based on viewport)
//! - Cursor at the correct terminal position
//! - Message line at the bottom row

use std::io;

use alfred_core::buffer;
use alfred_core::editor_state::EditorState;
use ratatui::backend::Backend;
use ratatui::layout::{Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

/// Renders a single frame of the editor state to the given terminal.
///
/// This is the main rendering entry point. It draws:
/// 1. Buffer content (visible lines based on viewport scroll position)
/// 2. Cursor at the correct position relative to the viewport
/// 3. Message line on the bottom row (if `state.message` is `Some`)
pub fn render_frame<B: Backend>(terminal: &mut Terminal<B>, state: &EditorState) -> io::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();

        let text_area = compute_text_area(area, state.message.is_some());
        let visible_lines = collect_visible_lines(state, text_area.height as usize);
        let text_widget = Paragraph::new(visible_lines);
        frame.render_widget(text_widget, text_area);

        if let Some(ref message) = state.message {
            let message_area = compute_message_area(area);
            let message_widget = Paragraph::new(message.as_str());
            frame.render_widget(message_widget, message_area);
        }

        let cursor_position = compute_cursor_position(state);
        frame.set_cursor_position(cursor_position);
    })?;
    Ok(())
}

/// Computes the area available for buffer text content.
///
/// When a message is present, the last row is reserved for the message line,
/// so the text area height is reduced by one.
fn compute_text_area(total_area: Rect, has_message: bool) -> Rect {
    let message_rows = if has_message { 1 } else { 0 };
    let text_height = total_area.height.saturating_sub(message_rows);
    Rect {
        x: total_area.x,
        y: total_area.y,
        width: total_area.width,
        height: text_height,
    }
}

/// Computes the area for the message line (always the bottom row).
fn compute_message_area(total_area: Rect) -> Rect {
    let last_row = total_area.height.saturating_sub(1);
    Rect {
        x: total_area.x,
        y: total_area.y + last_row,
        width: total_area.width,
        height: 1,
    }
}

/// Collects the visible lines from the buffer based on viewport scroll position.
///
/// Returns a Vec of ratatui Line values for the visible portion of the buffer.
fn collect_visible_lines(state: &EditorState, visible_height: usize) -> Vec<Line<'static>> {
    let top_line = state.viewport.top_line;
    let total_lines = buffer::line_count(&state.buffer);

    (0..visible_height)
        .map(|row| {
            let buffer_line_index = top_line + row;
            if buffer_line_index < total_lines {
                let line_content = buffer::get_line(&state.buffer, buffer_line_index)
                    .unwrap_or("")
                    .trim_end_matches('\n');
                Line::raw(line_content.to_string())
            } else {
                Line::raw("")
            }
        })
        .collect()
}

/// Computes the terminal cursor position from the editor state.
///
/// The cursor position is relative to the viewport: the terminal row is
/// `cursor.line - viewport.top_line`, and the terminal column is `cursor.column`.
fn compute_cursor_position(state: &EditorState) -> Position {
    let terminal_row = state.cursor.line.saturating_sub(state.viewport.top_line) as u16;
    let terminal_column = state.cursor.column as u16;
    Position::new(terminal_column, terminal_row)
}

/// Enters raw mode for terminal input handling.
///
/// Raw mode disables line buffering, echo, and special key processing,
/// allowing the editor to handle every keystroke directly.
fn enter_raw_mode() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()
}

/// Exits raw mode, restoring the terminal to its normal state.
///
/// This should be called on shutdown or on error to ensure the terminal
/// is usable after the editor exits.
fn exit_raw_mode() -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()
}

/// A guard that enables raw mode on creation and disables it on drop.
///
/// This ensures raw mode is always cleaned up, even on panic or early return.
pub(crate) struct RawModeGuard;

impl RawModeGuard {
    /// Creates a new RawModeGuard, enabling raw mode immediately.
    pub fn new() -> io::Result<Self> {
        enter_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = exit_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use alfred_core::buffer::Buffer;
    use alfred_core::editor_state;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    // -----------------------------------------------------------------------
    // Acceptance test: render EditorState with buffer content, cursor, and
    // message to a TestBackend and verify the output
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_state_with_content_when_rendered_then_buffer_lines_cursor_and_message_appear() {
        // Given: an EditorState with buffer content and a message
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.message = Some("Welcome".to_string());

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render the editor state
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the buffer content appears on the correct rows
        let rendered = terminal.backend();
        // Row 0 should contain "Hello" (padded to width)
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(row0.starts_with("Hello"), "Row 0 was: '{}'", row0);

        // Row 1 should contain "World"
        let row1 = extract_row_text(rendered.buffer(), 1);
        assert!(row1.starts_with("World"), "Row 1 was: '{}'", row1);

        // Row 2 should contain "Line3"
        let row2 = extract_row_text(rendered.buffer(), 2);
        assert!(row2.starts_with("Line3"), "Row 2 was: '{}'", row2);

        // The last row (4) should show the message "Welcome"
        let last_row = extract_row_text(rendered.buffer(), 4);
        assert!(
            last_row.starts_with("Welcome"),
            "Last row was: '{}'",
            last_row
        );
    }

    // -----------------------------------------------------------------------
    // Unit test: empty buffer renders without panic
    // -----------------------------------------------------------------------

    #[test]
    fn given_empty_buffer_when_rendered_then_no_panic_and_rows_are_blank() {
        // Given: an EditorState with an empty buffer
        let state = editor_state::new(20, 5);

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render the editor state
        let result = super::render_frame(&mut terminal, &state);

        // Then: rendering succeeds without panic
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Unit test: cursor positioned correctly
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_at_line_1_col_3_when_rendered_then_cursor_position_matches() {
        // Given: an EditorState with cursor at line 1, column 3
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.cursor = alfred_core::cursor::new(1, 3);

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render the editor state
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the cursor position in the backend reflects (column=3, row=1)
        let mut backend = terminal.backend_mut().clone();
        backend.assert_cursor_position(ratatui::layout::Position::new(3, 1));
    }

    // -----------------------------------------------------------------------
    // Unit test: viewport offset affects visible lines
    // -----------------------------------------------------------------------

    #[test]
    fn given_viewport_scrolled_down_when_rendered_then_only_visible_lines_shown() {
        // Given: an EditorState with 5 lines but viewport scrolled to top_line=2
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Line0\nLine1\nLine2\nLine3\nLine4");
        state.viewport.top_line = 2;

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render the editor state
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: row 0 shows Line2 (the first visible line after scroll)
        let rendered = terminal.backend();
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(row0.starts_with("Line2"), "Row 0 was: '{}'", row0);
    }

    // -----------------------------------------------------------------------
    // Unit test: message line at bottom when message is Some
    // -----------------------------------------------------------------------

    #[test]
    fn given_message_when_rendered_then_message_appears_on_last_row() {
        // Given: an EditorState with a message
        let mut state = editor_state::new(20, 3);
        state.buffer = Buffer::from_string("Hello");
        state.message = Some("Status: OK".to_string());

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: last row (2) shows the message
        let rendered = terminal.backend();
        let last_row = extract_row_text(rendered.buffer(), 2);
        assert!(
            last_row.starts_with("Status: OK"),
            "Last row was: '{}'",
            last_row
        );
    }

    // -----------------------------------------------------------------------
    // Unit test: no message leaves bottom row empty
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_message_when_rendered_then_last_row_is_empty() {
        // Given: an EditorState with no message
        let mut state = editor_state::new(20, 3);
        state.buffer = Buffer::from_string("Hello");
        state.message = None;

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: last row (2) is blank
        let rendered = terminal.backend();
        let last_row = extract_row_text(rendered.buffer(), 2);
        assert!(
            last_row.trim().is_empty(),
            "Last row should be empty but was: '{}'",
            last_row
        );
    }

    // -----------------------------------------------------------------------
    // Helper: extract text content of a specific row from the ratatui buffer
    // -----------------------------------------------------------------------

    fn extract_row_text(buffer: &ratatui::buffer::Buffer, row: u16) -> String {
        let width = buffer.area.width;
        (0..width)
            .map(|col| buffer[(col, row)].symbol().to_string())
            .collect::<String>()
    }
}
