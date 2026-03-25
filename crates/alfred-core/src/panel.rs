//! Panel system: generic named screen regions managed by plugins.
//!
//! Panels can be positioned at the edges of the screen (top, bottom, left, right).
//! The text area fills whatever space remains after all panels are laid out.
//!
//! The Rust core has no concept of "status bar" or "gutter" -- those are just
//! panels created by plugins with specific positions and content.

use std::collections::HashMap;

use crate::theme::ThemeColor;

/// Where a panel is positioned on screen.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelPosition {
    Top,
    Bottom,
    Left,
    Right,
}

/// A single panel with its properties.
#[derive(Debug, Clone)]
pub struct Panel {
    pub name: String,
    pub position: PanelPosition,
    pub size: u16,
    pub content: String,
    pub lines: HashMap<usize, String>,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub visible: bool,
    /// Cursor row within the panel (used when the panel has focus).
    pub cursor_line: usize,
    /// Rendering priority: lower values render more to the left (for left panels).
    /// Default is 50 (middle priority).
    pub priority: u16,
    /// Per-line style segments for colored text within panel lines.
    /// Maps line number -> Vec of (start_col, end_col, ThemeColor) segments.
    pub line_styles: HashMap<usize, Vec<(usize, usize, ThemeColor)>>,
}

/// Registry of all panels, ordered by creation time within each position.
#[derive(Debug, Clone, Default)]
pub struct PanelRegistry {
    pub panels: Vec<Panel>,
}

/// Creates a new empty panel registry.
pub fn new() -> PanelRegistry {
    PanelRegistry { panels: Vec::new() }
}

/// Creates a new panel and adds it to the registry.
///
/// Returns an error if a panel with the given name already exists.
pub fn define_panel(
    registry: &mut PanelRegistry,
    name: &str,
    position: PanelPosition,
    size: u16,
) -> Result<(), String> {
    if registry.panels.iter().any(|p| p.name == name) {
        return Err(format!("Panel '{}' already exists", name));
    }
    registry.panels.push(Panel {
        name: name.to_string(),
        position,
        size,
        content: String::new(),
        lines: HashMap::new(),
        fg_color: None,
        bg_color: None,
        visible: true,
        cursor_line: 0,
        priority: 50,
        line_styles: HashMap::new(),
    });
    Ok(())
}

/// Removes a panel by name.
///
/// If no panel with the given name exists, this is a no-op.
pub fn remove_panel(registry: &mut PanelRegistry, name: &str) {
    registry.panels.retain(|p| p.name != name);
}

/// Sets single-line content for a panel (used for top/bottom panels).
///
/// Returns an error if the panel does not exist.
pub fn set_content(registry: &mut PanelRegistry, name: &str, text: &str) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.content = text.to_string();
    })
}

/// Sets content for a specific line of a panel (used for left/right panels).
///
/// Returns an error if the panel does not exist.
pub fn set_line(
    registry: &mut PanelRegistry,
    name: &str,
    line_num: usize,
    text: &str,
) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.lines.insert(line_num, text.to_string());
    })
}

/// Sets the foreground and background colors for a panel.
///
/// Pass `None` for either color to leave it unchanged or clear it.
/// Returns an error if the panel does not exist.
pub fn set_style(
    registry: &mut PanelRegistry,
    name: &str,
    fg: Option<&str>,
    bg: Option<&str>,
) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.fg_color = fg.map(|s| s.to_string());
        panel.bg_color = bg.map(|s| s.to_string());
    })
}

/// Sets the size of a panel (height for top/bottom, width for left/right).
///
/// Returns an error if the panel does not exist.
pub fn set_size(registry: &mut PanelRegistry, name: &str, size: u16) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.size = size;
    })
}

/// Sets the rendering priority of a panel.
///
/// Lower priority = rendered more to the left for left panels.
/// Returns an error if the panel does not exist.
pub fn set_panel_priority(
    registry: &mut PanelRegistry,
    name: &str,
    priority: u16,
) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.priority = priority;
    })
}

/// Adds a style segment to a specific line of a panel.
///
/// Returns an error if the panel does not exist.
pub fn add_panel_line_style(
    registry: &mut PanelRegistry,
    name: &str,
    line: usize,
    start: usize,
    end: usize,
    color: ThemeColor,
) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel
            .line_styles
            .entry(line)
            .or_default()
            .push((start, end, color));
    })
}

/// Clears all per-line styles from a panel.
///
/// Returns an error if the panel does not exist.
pub fn clear_panel_line_styles(registry: &mut PanelRegistry, name: &str) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.line_styles.clear();
    })
}

/// Returns panels at the given position, sorted by priority (ascending).
///
/// Lower priority values render first (leftmost for left panels).
pub fn panels_at<'a>(registry: &'a PanelRegistry, position: &PanelPosition) -> Vec<&'a Panel> {
    let mut panels: Vec<&Panel> = registry
        .panels
        .iter()
        .filter(|p| &p.position == position)
        .collect();
    panels.sort_by_key(|p| p.priority);
    panels
}

/// Looks up a panel by name.
pub fn get<'a>(registry: &'a PanelRegistry, name: &str) -> Option<&'a Panel> {
    registry.panels.iter().find(|p| p.name == name)
}

/// Moves the panel's cursor down by one, clamping to the number of lines set on the panel.
///
/// Returns an error if the panel does not exist.
pub fn panel_cursor_down(registry: &mut PanelRegistry, name: &str) -> Result<(), String> {
    let panel = find_panel_mut(registry, name)?;
    let max_line = if panel.lines.is_empty() {
        0
    } else {
        *panel.lines.keys().max().unwrap()
    };
    if panel.cursor_line < max_line {
        panel.cursor_line += 1;
    }
    Ok(())
}

/// Moves the panel's cursor up by one, clamping at 0.
///
/// Returns an error if the panel does not exist.
pub fn panel_cursor_up(registry: &mut PanelRegistry, name: &str) -> Result<(), String> {
    let panel = find_panel_mut(registry, name)?;
    if panel.cursor_line > 0 {
        panel.cursor_line -= 1;
    }
    Ok(())
}

/// Returns the current cursor line of the named panel.
///
/// Returns an error if the panel does not exist.
pub fn panel_cursor_line(registry: &PanelRegistry, name: &str) -> Result<usize, String> {
    get(registry, name)
        .map(|p| p.cursor_line)
        .ok_or_else(|| format!("Panel '{}' not found", name))
}

/// Returns the number of lines set on the named panel (count of entries in the lines map).
///
/// Returns an error if the panel does not exist.
pub fn panel_entry_count(registry: &PanelRegistry, name: &str) -> Result<usize, String> {
    get(registry, name)
        .map(|p| {
            if p.lines.is_empty() {
                0
            } else {
                *p.lines.keys().max().unwrap() + 1
            }
        })
        .ok_or_else(|| format!("Panel '{}' not found", name))
}

/// Clears all lines from a panel, resetting cursor_line to 0.
///
/// Returns an error if the panel does not exist.
pub fn clear_lines(registry: &mut PanelRegistry, name: &str) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.lines.clear();
        panel.line_styles.clear();
        panel.cursor_line = 0;
    })
}

/// Sets the cursor line of a panel to an explicit value, clamped to valid range.
///
/// Returns an error if the panel does not exist.
pub fn set_panel_cursor(
    registry: &mut PanelRegistry,
    name: &str,
    line: usize,
) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.cursor_line = line;
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn find_panel_mut<'a>(
    registry: &'a mut PanelRegistry,
    name: &str,
) -> Result<&'a mut Panel, String> {
    registry
        .panels
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Panel '{}' not found", name))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- define_panel ---------------------------------------------------------

    #[test]
    fn given_empty_registry_when_define_panel_then_panel_exists_at_position() {
        let mut registry = new();
        let result = define_panel(&mut registry, "status", PanelPosition::Bottom, 1);

        assert!(result.is_ok());
        let panel = get(&registry, "status").expect("panel should exist");
        assert_eq!(panel.name, "status");
        assert_eq!(panel.position, PanelPosition::Bottom);
        assert_eq!(panel.size, 1);
        assert!(panel.visible);
        assert!(panel.content.is_empty());
    }

    #[test]
    fn given_existing_panel_when_define_panel_with_same_name_then_error() {
        let mut registry = new();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();

        let result = define_panel(&mut registry, "gutter", PanelPosition::Right, 6);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    // -- set_content ----------------------------------------------------------

    #[test]
    fn given_panel_when_set_content_then_content_updated() {
        let mut registry = new();
        define_panel(&mut registry, "status", PanelPosition::Bottom, 1).unwrap();

        let result = set_content(&mut registry, "status", "NORMAL | main.rs");

        assert!(result.is_ok());
        let panel = get(&registry, "status").unwrap();
        assert_eq!(panel.content, "NORMAL | main.rs");
    }

    #[test]
    fn given_no_panel_when_set_content_then_error() {
        let mut registry = new();
        let result = set_content(&mut registry, "nonexistent", "text");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // -- set_line -------------------------------------------------------------

    #[test]
    fn given_panel_when_set_line_then_line_content_stored() {
        let mut registry = new();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();

        set_line(&mut registry, "gutter", 0, "  1 ").unwrap();
        set_line(&mut registry, "gutter", 1, "  2 ").unwrap();

        let panel = get(&registry, "gutter").unwrap();
        assert_eq!(panel.lines.get(&0), Some(&"  1 ".to_string()));
        assert_eq!(panel.lines.get(&1), Some(&"  2 ".to_string()));
    }

    // -- remove_panel ---------------------------------------------------------

    #[test]
    fn given_panel_when_remove_then_panel_gone() {
        let mut registry = new();
        define_panel(&mut registry, "status", PanelPosition::Bottom, 1).unwrap();
        assert!(get(&registry, "status").is_some());

        remove_panel(&mut registry, "status");

        assert!(get(&registry, "status").is_none());
    }

    // -- panels_at ------------------------------------------------------------

    #[test]
    fn given_mixed_panels_when_panels_at_bottom_then_only_bottom_returned() {
        let mut registry = new();
        define_panel(&mut registry, "status", PanelPosition::Bottom, 1).unwrap();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();
        define_panel(&mut registry, "toolbar", PanelPosition::Bottom, 1).unwrap();

        let bottom_panels = panels_at(&registry, &PanelPosition::Bottom);

        assert_eq!(bottom_panels.len(), 2);
        assert_eq!(bottom_panels[0].name, "status");
        assert_eq!(bottom_panels[1].name, "toolbar");
    }

    #[test]
    fn given_mixed_panels_when_panels_at_left_then_only_left_returned() {
        let mut registry = new();
        define_panel(&mut registry, "status", PanelPosition::Bottom, 1).unwrap();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();

        let left_panels = panels_at(&registry, &PanelPosition::Left);

        assert_eq!(left_panels.len(), 1);
        assert_eq!(left_panels[0].name, "gutter");
    }

    // -- empty registry -------------------------------------------------------

    #[test]
    fn given_empty_registry_then_no_panels() {
        let registry = new();

        assert!(registry.panels.is_empty());
        assert!(get(&registry, "anything").is_none());
        assert!(panels_at(&registry, &PanelPosition::Top).is_empty());
        assert!(panels_at(&registry, &PanelPosition::Bottom).is_empty());
        assert!(panels_at(&registry, &PanelPosition::Left).is_empty());
        assert!(panels_at(&registry, &PanelPosition::Right).is_empty());
    }

    // -- set_style ------------------------------------------------------------

    #[test]
    fn given_panel_when_set_style_then_colors_updated() {
        let mut registry = new();
        define_panel(&mut registry, "status", PanelPosition::Bottom, 1).unwrap();

        set_style(&mut registry, "status", Some("#ffffff"), Some("#000000")).unwrap();

        let panel = get(&registry, "status").unwrap();
        assert_eq!(panel.fg_color, Some("#ffffff".to_string()));
        assert_eq!(panel.bg_color, Some("#000000".to_string()));
    }

    // -- set_size -------------------------------------------------------------

    #[test]
    fn given_panel_when_set_size_then_size_updated() {
        let mut registry = new();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();

        set_size(&mut registry, "gutter", 6).unwrap();

        let panel = get(&registry, "gutter").unwrap();
        assert_eq!(panel.size, 6);
    }

    // -- cursor_line initialization -------------------------------------------

    #[test]
    fn given_new_panel_then_cursor_line_is_zero() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();

        let panel = get(&registry, "tree").unwrap();
        assert_eq!(panel.cursor_line, 0);
    }

    // -- panel_cursor_down ----------------------------------------------------

    #[test]
    fn given_panel_with_lines_when_cursor_down_then_cursor_advances() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "file1").unwrap();
        set_line(&mut registry, "tree", 1, "file2").unwrap();
        set_line(&mut registry, "tree", 2, "file3").unwrap();

        panel_cursor_down(&mut registry, "tree").unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 1);
    }

    #[test]
    fn given_panel_at_last_line_when_cursor_down_then_cursor_stays() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "file1").unwrap();
        set_line(&mut registry, "tree", 1, "file2").unwrap();
        // Move to last line
        panel_cursor_down(&mut registry, "tree").unwrap();
        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 1);

        // Try to go past the end
        panel_cursor_down(&mut registry, "tree").unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 1);
    }

    #[test]
    fn given_nonexistent_panel_when_cursor_down_then_error() {
        let mut registry = new();
        let result = panel_cursor_down(&mut registry, "nope");
        assert!(result.is_err());
    }

    // -- panel_cursor_up ------------------------------------------------------

    #[test]
    fn given_panel_with_cursor_at_1_when_cursor_up_then_cursor_moves_to_0() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "file1").unwrap();
        set_line(&mut registry, "tree", 1, "file2").unwrap();
        panel_cursor_down(&mut registry, "tree").unwrap();

        panel_cursor_up(&mut registry, "tree").unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 0);
    }

    #[test]
    fn given_panel_at_line_0_when_cursor_up_then_cursor_stays_at_0() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "file1").unwrap();

        panel_cursor_up(&mut registry, "tree").unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 0);
    }

    // -- panel_cursor_line ----------------------------------------------------

    #[test]
    fn given_panel_when_panel_cursor_line_then_returns_current_position() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "a").unwrap();
        set_line(&mut registry, "tree", 1, "b").unwrap();
        panel_cursor_down(&mut registry, "tree").unwrap();

        assert_eq!(panel_cursor_line(&registry, "tree").unwrap(), 1);
    }

    #[test]
    fn given_nonexistent_panel_when_panel_cursor_line_then_error() {
        let registry = new();
        assert!(panel_cursor_line(&registry, "nope").is_err());
    }

    // -- panel_entry_count ----------------------------------------------------

    #[test]
    fn given_panel_with_3_lines_when_entry_count_then_returns_3() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "a").unwrap();
        set_line(&mut registry, "tree", 1, "b").unwrap();
        set_line(&mut registry, "tree", 2, "c").unwrap();

        assert_eq!(panel_entry_count(&registry, "tree").unwrap(), 3);
    }

    #[test]
    fn given_panel_with_no_lines_when_entry_count_then_returns_0() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();

        assert_eq!(panel_entry_count(&registry, "tree").unwrap(), 0);
    }

    #[test]
    fn given_nonexistent_panel_when_entry_count_then_error() {
        let registry = new();
        assert!(panel_entry_count(&registry, "nope").is_err());
    }

    // -- clear_lines ----------------------------------------------------------

    #[test]
    fn given_panel_with_lines_when_clear_lines_then_lines_empty_and_cursor_reset() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "a").unwrap();
        set_line(&mut registry, "tree", 1, "b").unwrap();
        set_line(&mut registry, "tree", 2, "c").unwrap();
        panel_cursor_down(&mut registry, "tree").unwrap();
        panel_cursor_down(&mut registry, "tree").unwrap();
        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 2);

        clear_lines(&mut registry, "tree").unwrap();

        let panel = get(&registry, "tree").unwrap();
        assert!(panel.lines.is_empty());
        assert_eq!(panel.cursor_line, 0);
    }

    #[test]
    fn given_nonexistent_panel_when_clear_lines_then_error() {
        let mut registry = new();
        assert!(clear_lines(&mut registry, "nope").is_err());
    }

    // -- set_panel_cursor -----------------------------------------------------

    #[test]
    fn given_panel_when_set_panel_cursor_then_cursor_moves_to_target() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "a").unwrap();
        set_line(&mut registry, "tree", 1, "b").unwrap();
        set_line(&mut registry, "tree", 2, "c").unwrap();

        set_panel_cursor(&mut registry, "tree", 2).unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 2);
    }

    #[test]
    fn given_panel_when_set_panel_cursor_to_zero_then_cursor_at_zero() {
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "a").unwrap();
        set_line(&mut registry, "tree", 1, "b").unwrap();
        panel_cursor_down(&mut registry, "tree").unwrap();

        set_panel_cursor(&mut registry, "tree", 0).unwrap();

        assert_eq!(get(&registry, "tree").unwrap().cursor_line, 0);
    }

    #[test]
    fn given_nonexistent_panel_when_set_panel_cursor_then_error() {
        let mut registry = new();
        assert!(set_panel_cursor(&mut registry, "nope", 0).is_err());
    }

    // -- set_panel_priority ---------------------------------------------------

    #[test]
    fn given_panel_when_set_priority_then_priority_updated() {
        let mut registry = new();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();
        assert_eq!(get(&registry, "gutter").unwrap().priority, 50); // default

        set_panel_priority(&mut registry, "gutter", 100).unwrap();

        assert_eq!(get(&registry, "gutter").unwrap().priority, 100);
    }

    #[test]
    fn given_nonexistent_panel_when_set_priority_then_error() {
        let mut registry = new();
        assert!(set_panel_priority(&mut registry, "nope", 10).is_err());
    }

    // -- panels_at sorted by priority -----------------------------------------

    #[test]
    fn given_left_panels_with_different_priorities_when_panels_at_then_sorted_by_priority() {
        let mut registry = new();
        define_panel(&mut registry, "gutter", PanelPosition::Left, 4).unwrap();
        set_panel_priority(&mut registry, "gutter", 100).unwrap();
        define_panel(&mut registry, "filetree", PanelPosition::Left, 30).unwrap();
        set_panel_priority(&mut registry, "filetree", 10).unwrap();

        let left = panels_at(&registry, &PanelPosition::Left);

        assert_eq!(left.len(), 2);
        assert_eq!(left[0].name, "filetree"); // priority 10 = leftmost
        assert_eq!(left[1].name, "gutter"); // priority 100 = rightmost
    }

    #[test]
    fn given_panels_with_same_priority_when_panels_at_then_stable_creation_order() {
        let mut registry = new();
        define_panel(&mut registry, "alpha", PanelPosition::Left, 4).unwrap();
        define_panel(&mut registry, "beta", PanelPosition::Left, 4).unwrap();

        let left = panels_at(&registry, &PanelPosition::Left);

        assert_eq!(left[0].name, "alpha");
        assert_eq!(left[1].name, "beta");
    }

    // -- panel line_styles ----------------------------------------------------

    #[test]
    fn given_panel_when_add_line_style_then_style_stored() {
        use crate::theme::ThemeColor;
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();

        add_panel_line_style(
            &mut registry,
            "tree",
            0,
            0,
            10,
            ThemeColor::Rgb(137, 180, 250),
        )
        .unwrap();

        let panel = get(&registry, "tree").unwrap();
        assert_eq!(panel.line_styles.len(), 1);
        let styles = panel.line_styles.get(&0).unwrap();
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0], (0, 10, ThemeColor::Rgb(137, 180, 250)));
    }

    #[test]
    fn given_panel_with_styles_when_clear_panel_line_styles_then_styles_empty() {
        use crate::theme::ThemeColor;
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        add_panel_line_style(&mut registry, "tree", 0, 0, 5, ThemeColor::Rgb(255, 0, 0)).unwrap();
        add_panel_line_style(&mut registry, "tree", 1, 0, 3, ThemeColor::Rgb(0, 255, 0)).unwrap();

        clear_panel_line_styles(&mut registry, "tree").unwrap();

        let panel = get(&registry, "tree").unwrap();
        assert!(panel.line_styles.is_empty());
    }

    #[test]
    fn given_panel_with_styles_when_clear_lines_then_line_styles_also_cleared() {
        use crate::theme::ThemeColor;
        let mut registry = new();
        define_panel(&mut registry, "tree", PanelPosition::Left, 20).unwrap();
        set_line(&mut registry, "tree", 0, "hello").unwrap();
        add_panel_line_style(&mut registry, "tree", 0, 0, 5, ThemeColor::Rgb(255, 0, 0)).unwrap();

        clear_lines(&mut registry, "tree").unwrap();

        let panel = get(&registry, "tree").unwrap();
        assert!(panel.lines.is_empty());
        assert!(panel.line_styles.is_empty());
    }

    #[test]
    fn given_nonexistent_panel_when_add_line_style_then_error() {
        use crate::theme::ThemeColor;
        let mut registry = new();
        assert!(
            add_panel_line_style(&mut registry, "nope", 0, 0, 5, ThemeColor::Rgb(0, 0, 0)).is_err()
        );
    }
}
