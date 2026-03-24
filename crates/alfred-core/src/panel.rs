//! Panel system: generic named screen regions managed by plugins.
//!
//! Panels can be positioned at the edges of the screen (top, bottom, left, right).
//! The text area fills whatever space remains after all panels are laid out.
//!
//! The Rust core has no concept of "status bar" or "gutter" -- those are just
//! panels created by plugins with specific positions and content.

use std::collections::HashMap;

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

/// Returns panels at the given position, in creation order.
pub fn panels_at<'a>(registry: &'a PanelRegistry, position: &PanelPosition) -> Vec<&'a Panel> {
    registry
        .panels
        .iter()
        .filter(|p| &p.position == position)
        .collect()
}

/// Looks up a panel by name.
pub fn get<'a>(registry: &'a PanelRegistry, name: &str) -> Option<&'a Panel> {
    registry.panels.iter().find(|p| p.name == name)
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
}
