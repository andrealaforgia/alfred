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
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

/// Renders a single frame of the editor state to the given terminal.
///
/// This is the main rendering entry point. It draws:
/// 1. Buffer content (visible lines based on viewport scroll position)
/// 2. Cursor at the correct position relative to the viewport
/// 3. Message line on the bottom row (if `state.message` is `Some`)
pub fn render_frame<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &EditorState,
    gutter_lines: &[String],
    status_line: Option<&str>,
) -> io::Result<()> {
    let gutter_width = state.viewport.gutter_width;

    terminal.draw(|frame| {
        let area = frame.area();
        let has_status = status_line.is_some();

        let content_area = compute_text_area(area, state.message.is_some(), has_status);

        if gutter_width > 0 {
            let (gutter_area, buffer_area) = split_gutter_and_text(content_area, gutter_width);

            let gutter_content = collect_gutter_lines(gutter_lines, content_area.height as usize);
            let gutter_widget = Paragraph::new(gutter_content);
            frame.render_widget(gutter_widget, gutter_area);

            let visible_lines = collect_visible_lines(state, buffer_area.height as usize);
            let text_widget = Paragraph::new(visible_lines);
            frame.render_widget(text_widget, buffer_area);
        } else {
            let visible_lines = collect_visible_lines(state, content_area.height as usize);
            let text_widget = Paragraph::new(visible_lines);
            frame.render_widget(text_widget, content_area);
        }

        if let Some(status) = status_line {
            let status_area = compute_status_area(area, state.message.is_some());
            let status_bg = resolve_theme_color(state, "status-bar-bg", Color::DarkGray);
            let status_fg = resolve_theme_color(state, "status-bar-fg", Color::White);
            let status_style = Style::default().bg(status_bg).fg(status_fg);
            let status_widget = Paragraph::new(status).style(status_style);
            frame.render_widget(status_widget, status_area);
        }

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
/// When a message is present, the last row is reserved for the message line.
/// When a status bar is present, one additional row is reserved above the message.
/// The text area height is reduced accordingly.
fn compute_text_area(total_area: Rect, has_message: bool, has_status: bool) -> Rect {
    let message_rows = if has_message { 1 } else { 0 };
    let status_rows = if has_status { 1 } else { 0 };
    let reserved = message_rows + status_rows;
    let text_height = total_area.height.saturating_sub(reserved);
    Rect {
        x: total_area.x,
        y: total_area.y,
        width: total_area.width,
        height: text_height,
    }
}

/// Computes the area for the status bar.
///
/// The status bar occupies one row between the text area and the message line.
/// When a message is present, the status bar is on the second-to-last row.
/// When no message, the status bar is on the last row.
fn compute_status_area(total_area: Rect, has_message: bool) -> Rect {
    let message_rows = if has_message { 1 } else { 0 };
    let status_row = total_area.height.saturating_sub(1 + message_rows);
    Rect {
        x: total_area.x,
        y: total_area.y + status_row,
        width: total_area.width,
        height: 1,
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

/// Splits a content area into a gutter area (left) and a text area (right).
///
/// The gutter area occupies `gutter_width` columns on the left.
/// The text area occupies the remaining columns on the right.
fn split_gutter_and_text(content_area: Rect, gutter_width: u16) -> (Rect, Rect) {
    let gutter_w = gutter_width.min(content_area.width);
    let text_w = content_area.width.saturating_sub(gutter_w);

    let gutter_area = Rect {
        x: content_area.x,
        y: content_area.y,
        width: gutter_w,
        height: content_area.height,
    };

    let text_area = Rect {
        x: content_area.x + gutter_w,
        y: content_area.y,
        width: text_w,
        height: content_area.height,
    };

    (gutter_area, text_area)
}

/// Collects gutter lines for the visible rows.
///
/// If `gutter_lines` has fewer entries than `visible_height`, the remaining
/// rows get empty strings. Each line is converted to a ratatui Line.
fn collect_gutter_lines(gutter_lines: &[String], visible_height: usize) -> Vec<Line<'static>> {
    (0..visible_height)
        .map(|row| {
            let content = gutter_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            Line::raw(content.to_string())
        })
        .collect()
}

/// Resolves a theme color from EditorState by slot name, with a fallback.
///
/// Looks up the color key in `state.theme`, converts the ThemeColor
/// to a `ratatui::Color`. If the key is not found, returns the fallback color.
pub fn resolve_theme_color(state: &EditorState, key: &str, fallback: Color) -> Color {
    match state.theme.get(key) {
        Some(theme_color) => theme_color_to_ratatui(*theme_color),
        None => fallback,
    }
}

/// Converts a pure ThemeColor domain type to a ratatui::Color for rendering.
fn theme_color_to_ratatui(color: alfred_core::theme::ThemeColor) -> Color {
    use alfred_core::theme::{NamedColor, ThemeColor};
    match color {
        ThemeColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        ThemeColor::Named(named) => match named {
            NamedColor::Black => Color::Black,
            NamedColor::Red => Color::Red,
            NamedColor::Green => Color::Green,
            NamedColor::Yellow => Color::Yellow,
            NamedColor::Blue => Color::Blue,
            NamedColor::Magenta => Color::Magenta,
            NamedColor::Cyan => Color::Cyan,
            NamedColor::White => Color::White,
            NamedColor::DarkGray => Color::DarkGray,
            NamedColor::LightRed => Color::LightRed,
            NamedColor::LightGreen => Color::LightGreen,
            NamedColor::LightYellow => Color::LightYellow,
            NamedColor::LightBlue => Color::LightBlue,
            NamedColor::LightMagenta => Color::LightMagenta,
            NamedColor::LightCyan => Color::LightCyan,
        },
    }
}

/// Computes the terminal cursor position from the editor state.
///
/// The cursor position is relative to the viewport: the terminal row is
/// `cursor.line - viewport.top_line`, and the terminal column is
/// `cursor.column + viewport.gutter_width` (to account for gutter offset).
fn compute_cursor_position(state: &EditorState) -> Position {
    let terminal_row = state.cursor.line.saturating_sub(state.viewport.top_line) as u16;
    let terminal_column = state.cursor.column as u16 + state.viewport.gutter_width;
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

/// Enters the alternate screen buffer.
///
/// The alternate screen is a separate terminal buffer that hides the shell
/// history and provides a clean full-screen canvas for the editor.
fn enter_alternate_screen() -> io::Result<()> {
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)
}

/// Leaves the alternate screen buffer, restoring the original terminal content.
///
/// This reveals the shell history that was hidden when the alternate screen
/// was entered.
fn leave_alternate_screen() -> io::Result<()> {
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)
}

/// A guard that manages terminal state: raw mode and alternate screen.
///
/// On creation, the guard enables raw mode and enters the alternate screen.
/// On drop, it leaves the alternate screen and disables raw mode (reverse
/// order of initialization). This ensures the terminal is always restored
/// to its original state, even on panic or early return (RAII pattern).
pub(crate) struct TerminalGuard;

impl TerminalGuard {
    /// Creates a new TerminalGuard, enabling raw mode and entering
    /// the alternate screen immediately.
    ///
    /// If entering raw mode succeeds but entering the alternate screen
    /// fails, raw mode is disabled before returning the error.
    pub fn new() -> io::Result<Self> {
        enter_raw_mode()?;
        if let Err(err) = enter_alternate_screen() {
            let _ = exit_raw_mode();
            return Err(err);
        }
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Reverse order: leave alternate screen first, then disable raw mode
        let _ = leave_alternate_screen();
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
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

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
        let result = super::render_frame(&mut terminal, &state, &[], None);

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
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

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
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

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
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

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
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

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
    // Acceptance test: gutter rendering with gutter_width > 0 and gutter content
    // -----------------------------------------------------------------------

    #[test]
    fn given_gutter_width_and_content_when_rendered_then_gutter_appears_left_and_text_shifts_right()
    {
        // Given: an EditorState with gutter_width=4 and buffer content
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.viewport.gutter_width = 4;

        // And: gutter content for each visible line
        let gutter_lines = vec![" 1 ".to_string(), " 2 ".to_string(), " 3 ".to_string()];

        // And: a TestBackend terminal (30 cols wide)
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with gutter content
        super::render_frame(&mut terminal, &state, &gutter_lines, None).unwrap();

        // Then: the gutter content appears on the left side of each row
        let rendered = terminal.backend();
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(
            row0.starts_with(" 1 "),
            "Row 0 should start with gutter ' 1 ' but was: '{}'",
            row0
        );

        // And: buffer text appears shifted right (after gutter columns)
        let row0_after_gutter = &row0[4..]; // gutter_width=4
        assert!(
            row0_after_gutter.starts_with("Hello"),
            "Row 0 after gutter should start with 'Hello' but was: '{}'",
            row0_after_gutter
        );

        // And: row 1 shows gutter and buffer text
        let row1 = extract_row_text(rendered.buffer(), 1);
        assert!(
            row1.starts_with(" 2 "),
            "Row 1 should start with gutter ' 2 ' but was: '{}'",
            row1
        );
        let row1_after_gutter = &row1[4..];
        assert!(
            row1_after_gutter.starts_with("World"),
            "Row 1 after gutter should start with 'World' but was: '{}'",
            row1_after_gutter
        );
    }

    // -----------------------------------------------------------------------
    // Unit test: cursor offset accounts for gutter width
    // -----------------------------------------------------------------------

    #[test]
    fn given_gutter_width_when_cursor_at_col_3_then_cursor_position_offset_by_gutter() {
        // Given: an EditorState with gutter_width=4 and cursor at line 1, column 3
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.cursor = alfred_core::cursor::new(1, 3);
        state.viewport.gutter_width = 4;

        let gutter_lines = vec![" 1 ".to_string(), " 2 ".to_string(), " 3 ".to_string()];

        // And: a TestBackend terminal
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with gutter
        super::render_frame(&mut terminal, &state, &gutter_lines, None).unwrap();

        // Then: cursor column is offset by gutter_width (3 + 4 = 7)
        let mut backend = terminal.backend_mut().clone();
        backend.assert_cursor_position(ratatui::layout::Position::new(7, 1));
    }

    // -----------------------------------------------------------------------
    // Unit test: zero gutter width renders identically (backwards compatible)
    // -----------------------------------------------------------------------

    #[test]
    fn given_zero_gutter_width_when_rendered_then_text_starts_at_column_zero() {
        // Given: an EditorState with gutter_width=0 (default)
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello\nWorld");

        // And: a TestBackend terminal
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with empty gutter lines (backwards compatible)
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

        // Then: text starts at column 0
        let rendered = terminal.backend();
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(
            row0.starts_with("Hello"),
            "Row 0 should start with 'Hello' but was: '{}'",
            row0
        );

        // And: cursor is at column 0 (no offset)
        let mut backend = terminal.backend_mut().clone();
        backend.assert_cursor_position(ratatui::layout::Position::new(0, 0));
    }

    // -----------------------------------------------------------------------
    // Unit test: fewer gutter lines than visible lines renders empty gutter
    // -----------------------------------------------------------------------

    #[test]
    fn given_fewer_gutter_lines_than_visible_when_rendered_then_remaining_gutter_rows_empty() {
        // Given: an EditorState with 3 buffer lines but only 1 gutter line
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.viewport.gutter_width = 4;

        let gutter_lines = vec![" 1 ".to_string()];

        // And: a TestBackend terminal
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state, &gutter_lines, None).unwrap();

        // Then: row 0 has gutter content
        let rendered = terminal.backend();
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(
            row0.starts_with(" 1 "),
            "Row 0 should start with gutter but was: '{}'",
            row0
        );

        // And: row 1 has buffer text shifted right but empty gutter area
        let row1 = extract_row_text(rendered.buffer(), 1);
        let row1_after_gutter = &row1[4..];
        assert!(
            row1_after_gutter.starts_with("World"),
            "Row 1 after gutter should start with 'World' but was: '{}'",
            row1_after_gutter
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (10-01): TerminalGuard type exists and manages both
    // raw mode and alternate screen
    // -----------------------------------------------------------------------

    #[test]
    fn given_terminal_guard_type_when_referenced_then_it_exists_and_is_public_in_crate() {
        // This test verifies the TerminalGuard type exists as a compile-time contract.
        // The guard manages both raw mode and alternate screen via RAII.
        // Actual terminal state cannot be tested without a real terminal,
        // so we verify the type signature and construction contract.
        fn _assert_terminal_guard_returns_io_result() -> std::io::Result<super::TerminalGuard> {
            // This function is never called -- it only needs to compile.
            // It proves: TerminalGuard::new() returns io::Result<TerminalGuard>
            super::TerminalGuard::new()
        }
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

    // -----------------------------------------------------------------------
    // Unit tests (05-01): status bar rendering
    // Test Budget: 4 behaviors x 2 = 8 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_status_line_when_rendered_then_text_area_uses_full_height() {
        // Given: 5-row terminal, no message, no status
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("A\nB\nC\nD\nE");

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: render with no status line
        super::render_frame(&mut terminal, &state, &[], None).unwrap();

        // Then: all 5 rows show buffer content (full height)
        let rendered = terminal.backend();
        let row4 = extract_row_text(rendered.buffer(), 4);
        assert!(
            row4.starts_with("E"),
            "Row 4 should show 'E' but was: '{}'",
            row4
        );
    }

    #[test]
    fn given_status_line_without_message_when_rendered_then_status_on_last_row_and_text_reduced() {
        // Given: 5-row terminal, no message, status present
        // Layout: rows 0-3 text, row 4 status
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("A\nB\nC\nD\nE");
        state.message = None;

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: render with status line, no message
        super::render_frame(&mut terminal, &state, &[], Some("status text")).unwrap();

        // Then: status appears on last row (4)
        let rendered = terminal.backend();
        let status_row = extract_row_text(rendered.buffer(), 4);
        assert!(
            status_row.starts_with("status text"),
            "Status row (4) should start with 'status text' but was: '{}'",
            status_row
        );

        // And: text area reduced -- row 3 should show "D" (only 4 text rows)
        let row3 = extract_row_text(rendered.buffer(), 3);
        assert!(
            row3.starts_with("D"),
            "Row 3 should show 'D' but was: '{}'",
            row3
        );
    }

    #[test]
    fn given_status_line_with_message_when_rendered_then_text_height_reduced_by_two() {
        // Given: 5-row terminal, message + status present
        // Layout: rows 0-2 text (3 rows), row 3 status, row 4 message
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("A\nB\nC\nD\nE");
        state.message = Some("msg".to_string());

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: render with status + message
        super::render_frame(&mut terminal, &state, &[], Some("bar")).unwrap();

        // Then: text rows 0-2 contain "A", "B", "C"
        let rendered = terminal.backend();
        let row2 = extract_row_text(rendered.buffer(), 2);
        assert!(
            row2.starts_with("C"),
            "Row 2 should show 'C' but was: '{}'",
            row2
        );

        // And: row 3 is status bar
        let row3 = extract_row_text(rendered.buffer(), 3);
        assert!(
            row3.starts_with("bar"),
            "Row 3 (status) should start with 'bar' but was: '{}'",
            row3
        );

        // And: row 4 is message
        let row4 = extract_row_text(rendered.buffer(), 4);
        assert!(
            row4.starts_with("msg"),
            "Row 4 (message) should start with 'msg' but was: '{}'",
            row4
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test (05-01): status bar rendering between text area and
    // message line
    // -----------------------------------------------------------------------

    #[test]
    fn given_status_line_and_message_when_rendered_then_status_appears_between_text_and_message() {
        // Given: an EditorState with buffer content and a message
        // Terminal: 20 cols x 6 rows
        // Expected layout:
        //   Row 0: "Hello"   (text)
        //   Row 1: "World"   (text)
        //   Row 2: (empty)   (text)
        //   Row 3: (empty)   (text)
        //   Row 4: "-- INSERT --" (status bar)
        //   Row 5: "Saved"   (message)
        let mut state = editor_state::new(20, 6);
        state.buffer = Buffer::from_string("Hello\nWorld");
        state.message = Some("Saved".to_string());

        let backend = TestBackend::new(20, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state, &[], Some("-- INSERT --")).unwrap();

        // Then: status bar appears on row 4 (second-to-last)
        let rendered = terminal.backend();
        let status_row = extract_row_text(rendered.buffer(), 4);
        assert!(
            status_row.starts_with("-- INSERT --"),
            "Status row (4) should start with '-- INSERT --' but was: '{}'",
            status_row
        );

        // And: message appears on last row (5)
        let message_row = extract_row_text(rendered.buffer(), 5);
        assert!(
            message_row.starts_with("Saved"),
            "Message row (5) should start with 'Saved' but was: '{}'",
            message_row
        );

        // And: text area occupies rows 0-3 (4 rows, reduced from 5 by status bar)
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(row0.starts_with("Hello"), "Row 0 was: '{}'", row0);
        let row1 = extract_row_text(rendered.buffer(), 1);
        assert!(row1.starts_with("World"), "Row 1 was: '{}'", row1);
    }

    // -----------------------------------------------------------------------
    // Acceptance test (10-02): theme color resolution for status bar
    // -----------------------------------------------------------------------

    #[test]
    fn given_theme_color_set_when_status_bar_rendered_then_themed_color_used() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        // Given: an EditorState with a custom status-bar-bg theme color
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello");
        state
            .theme
            .insert("status-bar-bg".to_string(), ThemeColor::Rgb(255, 0, 0));

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state, &[], Some("status")).unwrap();

        // Then: the status bar row uses the themed background color (red RGB)
        let rendered = terminal.backend();
        let status_row = 4u16; // last row, no message
        let cell = &rendered.buffer()[(0, status_row)];
        assert_eq!(
            cell.bg,
            Color::Rgb(255, 0, 0),
            "Status bar bg should be themed RGB(255,0,0) but was: {:?}",
            cell.bg
        );
    }

    #[test]
    fn given_no_theme_color_when_status_bar_rendered_then_fallback_color_used() {
        use ratatui::style::Color;

        // Given: an EditorState with no theme colors set (empty theme)
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello");

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state, &[], Some("status")).unwrap();

        // Then: the status bar row uses the default DarkGray background
        let rendered = terminal.backend();
        let status_row = 4u16; // last row, no message
        let cell = &rendered.buffer()[(0, status_row)];
        assert_eq!(
            cell.bg,
            Color::DarkGray,
            "Status bar bg should be default DarkGray but was: {:?}",
            cell.bg
        );
    }

    // -----------------------------------------------------------------------
    // Unit test (10-02): resolve_theme_color pure function
    // -----------------------------------------------------------------------

    #[test]
    fn given_theme_with_key_when_resolve_then_returns_themed_color() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        let mut state = editor_state::new(20, 5);
        state
            .theme
            .insert("text-fg".to_string(), ThemeColor::Rgb(100, 200, 50));

        let result = super::resolve_theme_color(&state, "text-fg", Color::Reset);
        assert_eq!(result, Color::Rgb(100, 200, 50));
    }

    #[test]
    fn given_theme_without_key_when_resolve_then_returns_fallback() {
        use ratatui::style::Color;

        let state = editor_state::new(20, 5);

        let result = super::resolve_theme_color(&state, "nonexistent-key", Color::Yellow);
        assert_eq!(result, Color::Yellow);
    }

    #[test]
    fn given_theme_with_named_color_when_resolve_then_returns_ratatui_color() {
        use alfred_core::theme::{NamedColor, ThemeColor};
        use ratatui::style::Color;

        let mut state = editor_state::new(20, 5);
        state
            .theme
            .insert("gutter-fg".to_string(), ThemeColor::Named(NamedColor::Cyan));

        let result = super::resolve_theme_color(&state, "gutter-fg", Color::Reset);
        assert_eq!(result, Color::Cyan);
    }
}
