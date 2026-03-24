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
use alfred_core::editor_state::{self, EditorState};
use alfred_core::panel::{self, PanelPosition};
use alfred_core::theme;
use crossterm::cursor::SetCursorStyle;
use ratatui::backend::Backend;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

/// Renders a single frame of the editor state to the given terminal.
///
/// This is the main rendering entry point. It reads panel layout from
/// `state.panels` and draws:
/// 1. Left panels (e.g., gutter/line numbers)
/// 2. Bottom panels (e.g., status bar)
/// 3. Top panels
/// 4. Buffer content in the remaining text area
/// 5. Cursor at the correct position relative to the viewport
/// 6. Message line on the bottom row (if `state.message` is `Some`)
pub fn render_frame<B: Backend>(terminal: &mut Terminal<B>, state: &EditorState) -> io::Result<()> {
    // Collect panel layout information
    let left_panels = panel::panels_at(&state.panels, &PanelPosition::Left);
    let bottom_panels = panel::panels_at(&state.panels, &PanelPosition::Bottom);
    let top_panels = panel::panels_at(&state.panels, &PanelPosition::Top);

    let total_left_width: u16 = left_panels
        .iter()
        .filter(|p| p.visible)
        .map(|p| p.size)
        .sum();
    let total_bottom_height: u16 = bottom_panels
        .iter()
        .filter(|p| p.visible)
        .map(|p| p.size)
        .sum();
    let total_top_height: u16 = top_panels
        .iter()
        .filter(|p| p.visible)
        .map(|p| p.size)
        .sum();

    let has_bottom_panels = total_bottom_height > 0;

    terminal.draw(|frame| {
        let area = frame.area();

        // Compute text area: subtract top panels, bottom panels, and message row
        let content_area = compute_text_area(area, state.message.is_some(), has_bottom_panels);
        let content_area = Rect {
            x: content_area.x,
            y: content_area.y + total_top_height,
            width: content_area.width,
            height: content_area.height.saturating_sub(total_top_height),
        };

        // Render top panels
        {
            let mut top_y = area.y;
            for panel in &top_panels {
                if !panel.visible {
                    continue;
                }
                let panel_area = Rect {
                    x: area.x,
                    y: top_y,
                    width: area.width,
                    height: panel.size,
                };
                let style = resolve_panel_style(state, panel);
                let widget = Paragraph::new(panel.content.as_str()).style(style);
                frame.render_widget(widget, panel_area);
                top_y += panel.size;
            }
        }

        // Render left panels and buffer content
        if total_left_width > 0 {
            let (gutter_area, buffer_area) = split_gutter_and_text(content_area, total_left_width);

            // Render each left panel
            let mut left_x = gutter_area.x;
            for panel in &left_panels {
                if !panel.visible {
                    continue;
                }
                let panel_area = Rect {
                    x: left_x,
                    y: gutter_area.y,
                    width: panel.size,
                    height: gutter_area.height,
                };
                let gutter_content = collect_panel_lines(panel, panel_area.height as usize);
                let style = resolve_panel_style(state, panel);
                let gutter_widget = Paragraph::new(gutter_content).style(style);
                frame.render_widget(gutter_widget, panel_area);
                left_x += panel.size;
            }

            let visible_lines = collect_visible_lines(state, buffer_area.height as usize);
            let text_widget = Paragraph::new(visible_lines);
            frame.render_widget(text_widget, buffer_area);
        } else {
            let visible_lines = collect_visible_lines(state, content_area.height as usize);
            let text_widget = Paragraph::new(visible_lines);
            frame.render_widget(text_widget, content_area);
        }

        // Render bottom panels (e.g., status bar)
        {
            let message_rows = if state.message.is_some() { 1u16 } else { 0 };
            let mut bottom_y = area
                .height
                .saturating_sub(message_rows + total_bottom_height);
            for panel in &bottom_panels {
                if !panel.visible {
                    continue;
                }
                let panel_area = Rect {
                    x: area.x,
                    y: area.y + bottom_y,
                    width: area.width,
                    height: panel.size,
                };
                let style = resolve_panel_style(state, panel);
                let widget = Paragraph::new(panel.content.as_str()).style(style);
                frame.render_widget(widget, panel_area);
                bottom_y += panel.size;
            }
        }

        if let Some(ref message) = state.message {
            let message_area = compute_message_area(area);
            let message_fg = resolve_theme_color(state, "message-fg", Color::Reset);
            let message_bg = resolve_theme_color(state, "message-bg", Color::Reset);
            let message_style = Style::default().fg(message_fg).bg(message_bg);
            let message_widget = Paragraph::new(message.as_str()).style(message_style);
            frame.render_widget(message_widget, message_area);
        }

        let cursor_position = compute_cursor_position(state);
        frame.set_cursor_position(cursor_position);
    })?;
    Ok(())
}

/// Resolves the ratatui Style for a panel, using panel-specific colors if set,
/// falling back to theme defaults based on panel name.
fn resolve_panel_style(state: &EditorState, panel: &alfred_core::panel::Panel) -> Style {
    // Priority: theme color > panel's own color > Color::Reset
    // Theme overrides panel defaults so that user themes always win.
    let panel_fg_fallback = panel
        .fg_color
        .as_deref()
        .and_then(theme::parse_color)
        .map(theme_color_to_ratatui)
        .unwrap_or(Color::Reset);
    let fg = {
        let theme_key = format!("{}-fg", panel.name);
        let resolved = resolve_theme_color(state, &theme_key, Color::Reset);
        if resolved == Color::Reset {
            panel_fg_fallback
        } else {
            resolved
        }
    };
    let panel_bg_fallback = panel
        .bg_color
        .as_deref()
        .and_then(theme::parse_color)
        .map(theme_color_to_ratatui)
        .unwrap_or(Color::Reset);
    let bg = {
        let theme_key = format!("{}-bg", panel.name);
        let resolved = resolve_theme_color(state, &theme_key, Color::Reset);
        if resolved == Color::Reset {
            panel_bg_fallback
        } else {
            resolved
        }
    };
    Style::default().fg(fg).bg(bg)
}

/// Collects per-line content from a left/right panel for rendering.
///
/// Reads from the panel's `lines` HashMap. If a line index has no content,
/// an empty string is used.
fn collect_panel_lines(
    panel: &alfred_core::panel::Panel,
    visible_height: usize,
) -> Vec<Line<'static>> {
    (0..visible_height)
        .map(|row| {
            let content = panel.lines.get(&row).map(|s| s.as_str()).unwrap_or("");
            Line::raw(content.to_string())
        })
        .collect()
}

/// Computes the area available for buffer text content.
///
/// When a message is present, the last row is reserved for the message line.
/// Bottom panel rows are reserved above the message line.
/// The text area height is reduced accordingly.
fn compute_text_area(total_area: Rect, has_message: bool, has_bottom_panels: bool) -> Rect {
    let message_rows = if has_message { 1 } else { 0 };
    let bottom_rows = if has_bottom_panels { 1 } else { 0 };
    let reserved = message_rows + bottom_rows;
    let text_height = total_area.height.saturating_sub(reserved);
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
/// When `line_styles` contains segments for a line, the text is split into
/// colored Spans; otherwise the line is rendered as plain text.
fn collect_visible_lines(state: &EditorState, visible_height: usize) -> Vec<Line<'static>> {
    let top_line = state.viewport.top_line;
    let total_lines = buffer::line_count(&state.buffer);

    (0..visible_height)
        .map(|row| {
            let buffer_line_index = top_line + row;
            if buffer_line_index < total_lines {
                let line_content = buffer::get_line_string(&state.buffer, buffer_line_index)
                    .trim_end_matches('\n')
                    .to_string();
                build_styled_line(&line_content, state.line_styles.get(&buffer_line_index))
            } else {
                Line::raw("")
            }
        })
        .collect()
}

/// Builds a ratatui Line from text and optional style segments.
///
/// If segments are provided, the text is split into styled Spans where each
/// segment applies a foreground color. Any text not covered by a segment
/// retains the default style. If no segments, returns a plain Line.
fn build_styled_line(
    text: &str,
    segments: Option<&Vec<(usize, usize, alfred_core::theme::ThemeColor)>>,
) -> Line<'static> {
    match segments {
        Some(segs) if !segs.is_empty() => {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut pos = 0usize;
            let text_len = text.len();

            for &(start, end, color) in segs {
                let seg_start = start.min(text_len);
                let seg_end = end.min(text_len);

                // Skip segments that overlap with already-rendered text.
                // This prevents duplicate character rendering when segments
                // are not perfectly non-overlapping.
                if seg_start < pos {
                    // Partial overlap: only render the non-overlapping tail
                    let adjusted_start = pos;
                    if adjusted_start < seg_end {
                        let fg = theme_color_to_ratatui(color);
                        let style = Style::default().fg(fg);
                        spans.push(Span::styled(
                            text[adjusted_start..seg_end].to_string(),
                            style,
                        ));
                        pos = seg_end;
                    }
                    continue;
                }

                // Add unstyled gap before this segment if needed
                if pos < seg_start {
                    spans.push(Span::raw(text[pos..seg_start].to_string()));
                }

                // Add the styled segment
                if seg_start < seg_end {
                    let fg = theme_color_to_ratatui(color);
                    let style = Style::default().fg(fg);
                    spans.push(Span::styled(text[seg_start..seg_end].to_string(), style));
                }

                pos = seg_end;
            }

            // Add any remaining text after the last segment
            if pos < text_len {
                spans.push(Span::raw(text[pos..].to_string()));
            }

            Line::from(spans)
        }
        _ => Line::raw(text.to_string()),
    }
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

/// Converts a cursor shape name string to a crossterm `SetCursorStyle`.
///
/// Returns `None` for "default" (which requires `DefaultUserShape`), or
/// `Some(style)` for all other recognized shape names. Unrecognized names
/// return `None` as a safe fallback.
fn shape_name_to_cursor_style(shape_name: &str) -> Option<SetCursorStyle> {
    match shape_name {
        "block" | "steady-block" => Some(SetCursorStyle::SteadyBlock),
        "blinking-block" => Some(SetCursorStyle::BlinkingBlock),
        "bar" | "steady-bar" => Some(SetCursorStyle::SteadyBar),
        "blinking-bar" => Some(SetCursorStyle::BlinkingBar),
        "underline" | "steady-underline" => Some(SetCursorStyle::SteadyUnderScore),
        "blinking-underline" => Some(SetCursorStyle::BlinkingUnderScore),
        "default" => Some(SetCursorStyle::DefaultUserShape),
        _ => None,
    }
}

/// Sets the terminal cursor shape based on the current editor mode.
///
/// Looks up the cursor shape name configured for the current mode in
/// `state.cursor_shapes`, converts it to a crossterm cursor style, and
/// emits the escape sequence to the terminal. If no shape is configured
/// or the shape name is unrecognized, defaults to `DefaultUserShape`.
pub fn apply_cursor_shape(state: &EditorState) -> io::Result<()> {
    let shape_name = editor_state::cursor_shape_for_mode(state);
    let style = shape_name_to_cursor_style(shape_name).unwrap_or(SetCursorStyle::DefaultUserShape);
    crossterm::execute!(io::stdout(), style)
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
        // Reset cursor to default shape before leaving
        let _ = crossterm::execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        // Reverse order: leave alternate screen first, then disable raw mode
        let _ = leave_alternate_screen();
        let _ = exit_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use alfred_core::buffer::Buffer;
    use alfred_core::editor_state;
    use alfred_core::panel::{self, PanelPosition};
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
    // Acceptance test: gutter rendering via left panel
    // -----------------------------------------------------------------------

    #[test]
    fn given_left_panel_with_content_when_rendered_then_gutter_appears_left_and_text_shifts_right()
    {
        use alfred_core::panel::{self, PanelPosition};

        // Given: an EditorState with a left panel (gutter) of size 4
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        panel::define_panel(&mut state.panels, "gutter", PanelPosition::Left, 4).unwrap();
        panel::set_line(&mut state.panels, "gutter", 0, " 1 ").unwrap();
        panel::set_line(&mut state.panels, "gutter", 1, " 2 ").unwrap();
        panel::set_line(&mut state.panels, "gutter", 2, " 3 ").unwrap();
        state.viewport.gutter_width = 4;

        // And: a TestBackend terminal (30 cols wide)
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the gutter content appears on the left side of each row
        let rendered = terminal.backend();
        let row0 = extract_row_text(rendered.buffer(), 0);
        assert!(
            row0.starts_with(" 1 "),
            "Row 0 should start with gutter ' 1 ' but was: '{}'",
            row0
        );

        // And: buffer text appears shifted right (after gutter columns)
        let row0_after_gutter = &row0[4..]; // panel size=4
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
        use alfred_core::panel::{self, PanelPosition};

        // Given: an EditorState with a left panel (gutter) of size 4 and cursor at line 1, column 3
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.cursor = alfred_core::cursor::new(1, 3);
        state.viewport.gutter_width = 4;
        panel::define_panel(&mut state.panels, "gutter", PanelPosition::Left, 4).unwrap();
        panel::set_line(&mut state.panels, "gutter", 0, " 1 ").unwrap();
        panel::set_line(&mut state.panels, "gutter", 1, " 2 ").unwrap();
        panel::set_line(&mut state.panels, "gutter", 2, " 3 ").unwrap();

        // And: a TestBackend terminal
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

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
        super::render_frame(&mut terminal, &state).unwrap();

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
    fn given_fewer_panel_lines_than_visible_when_rendered_then_remaining_gutter_rows_empty() {
        use alfred_core::panel::{self, PanelPosition};

        // Given: an EditorState with 3 buffer lines but only 1 gutter panel line set
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld\nLine3");
        state.viewport.gutter_width = 4;
        panel::define_panel(&mut state.panels, "gutter", PanelPosition::Left, 4).unwrap();
        panel::set_line(&mut state.panels, "gutter", 0, " 1 ").unwrap();

        // And: a TestBackend terminal
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

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
        super::render_frame(&mut terminal, &state).unwrap();

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
    fn given_bottom_panel_without_message_when_rendered_then_status_on_last_row_and_text_reduced() {
        use alfred_core::panel::{self, PanelPosition};

        // Given: 5-row terminal, no message, bottom panel (status) present
        // Layout: rows 0-3 text, row 4 status
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("A\nB\nC\nD\nE");
        state.message = None;
        panel::define_panel(&mut state.panels, "status", PanelPosition::Bottom, 1).unwrap();
        panel::set_content(&mut state.panels, "status", "status text").unwrap();

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: render with bottom panel, no message
        super::render_frame(&mut terminal, &state).unwrap();

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
    fn given_bottom_panel_with_message_when_rendered_then_text_height_reduced_by_two() {
        use alfred_core::panel::{self, PanelPosition};

        // Given: 5-row terminal, message + bottom panel present
        // Layout: rows 0-2 text (3 rows), row 3 status panel, row 4 message
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("A\nB\nC\nD\nE");
        state.message = Some("msg".to_string());
        panel::define_panel(&mut state.panels, "status", PanelPosition::Bottom, 1).unwrap();
        panel::set_content(&mut state.panels, "status", "bar").unwrap();

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: render with bottom panel + message
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: text rows 0-2 contain "A", "B", "C"
        let rendered = terminal.backend();
        let row2 = extract_row_text(rendered.buffer(), 2);
        assert!(
            row2.starts_with("C"),
            "Row 2 should show 'C' but was: '{}'",
            row2
        );

        // And: row 3 is status bar panel
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
        panel::define_panel(&mut state.panels, "status", PanelPosition::Bottom, 1).unwrap();
        panel::set_content(&mut state.panels, "status", "-- INSERT --").unwrap();

        let backend = TestBackend::new(20, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state).unwrap();

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
        panel::define_panel(&mut state.panels, "status-bar", PanelPosition::Bottom, 1).unwrap();
        panel::set_content(&mut state.panels, "status-bar", "status").unwrap();

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state).unwrap();

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
        // The status-bar panel has a default DarkGray background (panel-level default).
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello");
        panel::define_panel(&mut state.panels, "status-bar", PanelPosition::Bottom, 1).unwrap();
        panel::set_content(&mut state.panels, "status-bar", "status").unwrap();
        panel::set_style(
            &mut state.panels,
            "status-bar",
            Some("white"),
            Some("dark-gray"),
        )
        .unwrap();

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with a status line
        super::render_frame(&mut terminal, &state).unwrap();

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

    // -----------------------------------------------------------------------
    // Acceptance test (10-04): gutter uses theme color when set
    // -----------------------------------------------------------------------

    #[test]
    fn given_gutter_theme_color_when_rendered_then_gutter_uses_themed_foreground() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        // Given: an EditorState with gutter_width=4, gutter content, and gutter-fg themed
        let mut state = editor_state::new(30, 5);
        state.buffer = Buffer::from_string("Hello\nWorld");
        state.viewport.gutter_width = 4;
        state
            .theme
            .insert("gutter-fg".to_string(), ThemeColor::Rgb(108, 112, 134));

        panel::define_panel(&mut state.panels, "gutter", PanelPosition::Left, 4).unwrap();
        panel::set_line(&mut state.panels, "gutter", 0, " 1 ").unwrap();
        panel::set_line(&mut state.panels, "gutter", 1, " 2 ").unwrap();

        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with gutter
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the gutter cells use the themed foreground color
        let rendered = terminal.backend();
        let gutter_cell = &rendered.buffer()[(1, 0)]; // column 1 of row 0 (in gutter)
        assert_eq!(
            gutter_cell.fg,
            Color::Rgb(108, 112, 134),
            "Gutter fg should be themed RGB(108,112,134) but was: {:?}",
            gutter_cell.fg
        );
    }

    // -----------------------------------------------------------------------
    // Unit test (10-04): message uses theme color when set
    // -----------------------------------------------------------------------

    #[test]
    fn given_message_theme_color_when_rendered_then_message_uses_themed_foreground() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        // Given: an EditorState with message and themed message-fg
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("Hello");
        state.message = Some("Test message".to_string());
        state
            .theme
            .insert("message-fg".to_string(), ThemeColor::Rgb(200, 100, 50));

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render with message
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the message row uses the themed foreground color
        let rendered = terminal.backend();
        let msg_row = 4u16; // last row
        let cell = &rendered.buffer()[(0, msg_row)];
        assert_eq!(
            cell.fg,
            Color::Rgb(200, 100, 50),
            "Message fg should be themed RGB(200,100,50) but was: {:?}",
            cell.fg
        );
    }

    // -----------------------------------------------------------------------
    // Acceptance test: line_styles colorize CSV columns in rendered output
    // Test Budget: 3 behaviors x 2 = 6 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_line_styles_when_rendered_then_columns_have_different_foreground_colors() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        // Given: an EditorState with CSV content and line_styles set
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("aa,bb,cc");
        // Manually set line styles: col0 red, col1 green, col2 blue
        state.line_styles.insert(
            0,
            vec![
                (0, 2, ThemeColor::Rgb(255, 0, 0)), // "aa" in red
                (3, 5, ThemeColor::Rgb(0, 255, 0)), // "bb" in green
                (6, 8, ThemeColor::Rgb(0, 0, 255)), // "cc" in blue
            ],
        );

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: column 0 ("aa") has red foreground
        let rendered = terminal.backend();
        let cell_a = &rendered.buffer()[(0, 0)];
        assert_eq!(
            cell_a.fg,
            Color::Rgb(255, 0, 0),
            "Column 0 should be red but was: {:?}",
            cell_a.fg
        );

        // And: column 1 ("bb") has green foreground
        let cell_b = &rendered.buffer()[(3, 0)];
        assert_eq!(
            cell_b.fg,
            Color::Rgb(0, 255, 0),
            "Column 1 should be green but was: {:?}",
            cell_b.fg
        );

        // And: column 2 ("cc") has blue foreground
        let cell_c = &rendered.buffer()[(6, 0)];
        assert_eq!(
            cell_c.fg,
            Color::Rgb(0, 0, 255),
            "Column 2 should be blue but was: {:?}",
            cell_c.fg
        );
    }

    #[test]
    fn given_no_line_styles_when_rendered_then_text_uses_default_color() {
        use ratatui::style::Color;

        // Given: an EditorState with content but no line_styles
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("hello,world");

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: text uses the default/reset color (no custom fg)
        let rendered = terminal.backend();
        let cell = &rendered.buffer()[(0, 0)];
        assert_eq!(
            cell.fg,
            Color::Reset,
            "Without line_styles, text should use default color but was: {:?}",
            cell.fg
        );
    }

    #[test]
    fn given_line_styles_with_delimiter_gap_when_rendered_then_delimiter_uses_default_color() {
        use alfred_core::theme::ThemeColor;
        use ratatui::style::Color;

        // Given: line_styles that skip the comma character
        let mut state = editor_state::new(20, 5);
        state.buffer = Buffer::from_string("a,b");
        state.line_styles.insert(
            0,
            vec![
                (0, 1, ThemeColor::Rgb(255, 0, 0)), // "a" in red
                (2, 3, ThemeColor::Rgb(0, 255, 0)), // "b" in green
            ],
        );

        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        // When: we render
        super::render_frame(&mut terminal, &state).unwrap();

        // Then: the comma at position 1 has default color (gap between segments)
        let rendered = terminal.backend();
        let comma_cell = &rendered.buffer()[(1, 0)];
        assert_eq!(
            comma_cell.fg,
            Color::Reset,
            "Delimiter should use default color but was: {:?}",
            comma_cell.fg
        );
    }
}
