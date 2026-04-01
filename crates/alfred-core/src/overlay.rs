//! Overlay: a floating search/command overlay panel.
//!
//! Pure data model and functions for a modal overlay that displays a title,
//! text input, and a scrollable list of selectable items. All functions are
//! pure -- they take an Overlay value and return a new Overlay value.

/// A floating overlay panel for search, command palette, or similar UI.
///
/// All fields are public for read access. Mutation is done through the
/// module-level pure functions which return new `Overlay` values.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    /// Whether the overlay is currently displayed.
    pub visible: bool,
    /// The overlay title shown in the header.
    pub title: String,
    /// The current text input (search query, command filter, etc.).
    pub input: String,
    /// The list of selectable items displayed in the overlay.
    pub items: Vec<String>,
    /// The index of the currently highlighted item.
    pub cursor_index: usize,
    /// The width of the overlay in columns.
    pub width: usize,
    /// The maximum number of items visible at once (scroll window size).
    pub max_visible_items: usize,
    /// The scroll offset into the items list (first visible item index).
    pub scroll_offset: usize,
    /// Foreground color for normal items (hex string like "#cdd6f4"), empty = default.
    pub fg_color: String,
    /// Background color for the overlay box (hex string), empty = default.
    pub bg_color: String,
    /// Foreground color for the highlighted/selected item, empty = default.
    pub highlight_fg: String,
    /// Background color for the highlighted/selected item, empty = default.
    pub highlight_bg: String,
    /// Foreground color for the input/prompt line, empty = default.
    pub prompt_fg: String,
    /// Border color (hex string), empty = default.
    pub border_color: String,
}

/// Returns a new overlay with style colors updated.
pub fn set_style(
    overlay: &Overlay,
    fg: &str,
    bg: &str,
    highlight_fg: &str,
    highlight_bg: &str,
    prompt_fg: &str,
    border_color: &str,
) -> Overlay {
    Overlay {
        fg_color: fg.to_string(),
        bg_color: bg.to_string(),
        highlight_fg: highlight_fg.to_string(),
        highlight_bg: highlight_bg.to_string(),
        prompt_fg: prompt_fg.to_string(),
        border_color: border_color.to_string(),
        ..overlay.clone()
    }
}

/// Creates a new overlay with the given title, width, and max visible items.
///
/// The overlay starts invisible with empty input, no items, and cursor at zero.
pub fn create(title: &str, width: usize, max_visible_items: usize) -> Overlay {
    Overlay {
        title: title.to_string(),
        width,
        max_visible_items,
        ..Overlay::default()
    }
}

/// Resets an overlay to its initial state, preserving title, width, and max_visible_items.
///
/// Clears input, items, cursor, scroll offset, and sets visible to false.
pub fn reset(overlay: &Overlay) -> Overlay {
    Overlay {
        visible: false,
        input: String::new(),
        items: Vec::new(),
        cursor_index: 0,
        scroll_offset: 0,
        ..overlay.clone()
    }
}

/// Returns a new overlay with the input text updated.
pub fn set_input(overlay: &Overlay, input: &str) -> Overlay {
    Overlay {
        input: input.to_string(),
        ..overlay.clone()
    }
}

/// Returns a new overlay with the items list replaced.
///
/// Resets cursor_index and scroll_offset to zero since the item list changed.
pub fn set_items(overlay: &Overlay, items: Vec<String>) -> Overlay {
    Overlay {
        items,
        cursor_index: 0,
        scroll_offset: 0,
        ..overlay.clone()
    }
}

/// Returns the currently selected item, or `None` if the items list is empty.
pub fn get_selected(overlay: &Overlay) -> Option<String> {
    overlay.items.get(overlay.cursor_index).cloned()
}

/// Returns a new overlay with the cursor moved down by one, clamped at the last item.
///
/// If items is empty, cursor stays at zero.
pub fn cursor_down(overlay: &Overlay) -> Overlay {
    let max_index = overlay.items.len().saturating_sub(1);
    let new_index = if overlay.items.is_empty() {
        0
    } else {
        (overlay.cursor_index + 1).min(max_index)
    };
    Overlay {
        cursor_index: new_index,
        ..overlay.clone()
    }
}

/// Returns a new overlay with the cursor moved up by one, clamped at zero.
pub fn cursor_up(overlay: &Overlay) -> Overlay {
    let new_index = overlay.cursor_index.saturating_sub(1);
    Overlay {
        cursor_index: new_index,
        ..overlay.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // Acceptance test: full overlay data model contract
    // -------------------------------------------------------------------

    #[test]
    fn overlay_full_contract() {
        // AC1: New overlay created with visible=false and empty collections
        let overlay = create("Search", 60, 10);
        assert!(!overlay.visible);
        assert_eq!(overlay.title, "Search");
        assert!(overlay.input.is_empty());
        assert!(overlay.items.is_empty());
        assert_eq!(overlay.cursor_index, 0);
        assert_eq!(overlay.width, 60);
        assert_eq!(overlay.max_visible_items, 10);
        assert_eq!(overlay.scroll_offset, 0);

        // AC2: Setting input text produces new overlay with updated input
        let with_input = set_input(&overlay, "hello");
        assert_eq!(with_input.input, "hello");
        // Original unchanged (immutability)
        assert!(overlay.input.is_empty());

        // AC3: Setting items produces new overlay with updated item list
        let items = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let with_items = set_items(&with_input, items.clone());
        assert_eq!(with_items.items, items);
        // Original unchanged
        assert!(with_input.items.is_empty());

        // AC4: Get-selected returns item at cursor index or nothing when empty
        assert_eq!(get_selected(&with_items), Some("alpha".to_string()));
        assert_eq!(get_selected(&overlay), None); // empty items

        // AC5: Cursor-down clamps at last item; cursor-up clamps at zero
        let moved_down = cursor_down(&with_items);
        assert_eq!(moved_down.cursor_index, 1);
        assert_eq!(get_selected(&moved_down), Some("beta".to_string()));

        let moved_down2 = cursor_down(&moved_down);
        assert_eq!(moved_down2.cursor_index, 2);

        // Clamped at last item
        let clamped = cursor_down(&moved_down2);
        assert_eq!(clamped.cursor_index, 2);

        // Cursor up from zero clamps at zero
        let at_zero = cursor_up(&with_items);
        assert_eq!(at_zero.cursor_index, 0);
    }

    // -------------------------------------------------------------------
    // Unit tests: one per acceptance criterion
    // -------------------------------------------------------------------

    // AC1: create/reset produces default overlay
    #[test]
    fn create_returns_invisible_overlay_with_empty_collections() {
        let overlay = create("Commands", 40, 5);
        assert!(!overlay.visible);
        assert_eq!(overlay.title, "Commands");
        assert!(overlay.input.is_empty());
        assert!(overlay.items.is_empty());
        assert_eq!(overlay.cursor_index, 0);
        assert_eq!(overlay.width, 40);
        assert_eq!(overlay.max_visible_items, 5);
        assert_eq!(overlay.scroll_offset, 0);
    }

    #[test]
    fn reset_returns_overlay_to_initial_state() {
        let overlay = create("Search", 60, 10);
        let modified = set_input(&set_items(&overlay, vec!["a".to_string()]), "query");
        let after_reset = reset(&modified);
        assert!(!after_reset.visible);
        assert!(after_reset.input.is_empty());
        assert!(after_reset.items.is_empty());
        assert_eq!(after_reset.cursor_index, 0);
        assert_eq!(after_reset.scroll_offset, 0);
        // Title, width, max_visible_items preserved
        assert_eq!(after_reset.title, "Search");
        assert_eq!(after_reset.width, 60);
        assert_eq!(after_reset.max_visible_items, 10);
    }

    // AC2: set_input produces new overlay with updated input
    #[test]
    fn set_input_updates_input_without_mutating_original() {
        let original = create("Search", 60, 10);
        let updated = set_input(&original, "test query");
        assert_eq!(updated.input, "test query");
        assert!(original.input.is_empty());
    }

    // AC3: set_items produces new overlay with updated item list
    #[test]
    fn set_items_updates_items_and_resets_cursor() {
        let overlay = create("Search", 60, 10);
        let with_cursor = cursor_down(&set_items(&overlay, vec!["a".to_string(), "b".to_string()]));
        assert_eq!(with_cursor.cursor_index, 1);

        // Setting new items resets cursor to 0
        let new_items = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let refreshed = set_items(&with_cursor, new_items.clone());
        assert_eq!(refreshed.items, new_items);
        assert_eq!(refreshed.cursor_index, 0);
        assert_eq!(refreshed.scroll_offset, 0);
    }

    // AC4: get_selected returns item at cursor or None when empty
    #[test]
    fn get_selected_returns_item_at_cursor_index() {
        let overlay = set_items(
            &create("Search", 60, 10),
            vec!["first".to_string(), "second".to_string()],
        );
        assert_eq!(get_selected(&overlay), Some("first".to_string()));

        let at_second = cursor_down(&overlay);
        assert_eq!(get_selected(&at_second), Some("second".to_string()));
    }

    #[test]
    fn get_selected_returns_none_when_items_empty() {
        let overlay = create("Search", 60, 10);
        assert_eq!(get_selected(&overlay), None);
    }

    // AC5: cursor clamping
    #[test]
    fn cursor_down_clamps_at_last_item() {
        let overlay = set_items(
            &create("Search", 60, 10),
            vec!["only".to_string(), "two".to_string()],
        );
        let down1 = cursor_down(&overlay);
        assert_eq!(down1.cursor_index, 1);
        let down2 = cursor_down(&down1);
        assert_eq!(down2.cursor_index, 1); // clamped
    }

    #[test]
    fn cursor_up_clamps_at_zero() {
        let overlay = set_items(
            &create("Search", 60, 10),
            vec!["a".to_string(), "b".to_string()],
        );
        let up = cursor_up(&overlay);
        assert_eq!(up.cursor_index, 0); // already at zero, clamped
    }

    #[test]
    fn cursor_down_on_empty_items_stays_at_zero() {
        let overlay = create("Search", 60, 10);
        let down = cursor_down(&overlay);
        assert_eq!(down.cursor_index, 0);
    }

    #[test]
    fn cursor_up_from_nonzero_decrements() {
        let overlay = set_items(
            &create("Search", 60, 10),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        let at_two = cursor_down(&cursor_down(&overlay));
        assert_eq!(at_two.cursor_index, 2);
        let at_one = cursor_up(&at_two);
        assert_eq!(at_one.cursor_index, 1);
    }
}
