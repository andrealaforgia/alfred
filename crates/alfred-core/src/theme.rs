//! Theme: pure domain types and functions for editor color theming.
//!
//! This module defines ThemeColor, NamedColor, and parse_color as pure
//! functions with no I/O dependencies. Conversion to ratatui::Color
//! happens in the renderer (alfred-tui), not here.

use std::collections::HashMap;

/// Named ANSI colors supported by the theme system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
}

/// A theme color value: either an RGB triple or a named ANSI color.
///
/// This is a pure domain type with no rendering dependency.
/// Conversion to terminal-specific color types happens at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColor {
    Rgb(u8, u8, u8),
    Named(NamedColor),
}

/// A theme is a mapping from color slot names to ThemeColor values.
///
/// Color slots follow a naming convention: "component-property",
/// e.g., "status-bar-bg", "gutter-fg", "text-bg".
pub type Theme = HashMap<String, ThemeColor>;

/// Creates a new empty theme.
pub fn new_theme() -> Theme {
    HashMap::new()
}

/// Parses a color string into a ThemeColor.
///
/// Supported formats:
/// - RGB hex: "#rrggbb" (e.g., "#ff5733" -> Rgb(255, 87, 51))
/// - Named ANSI colors: "red", "blue", "dark-gray", etc.
/// - "default" -> None (use terminal default)
///
/// Returns None for "default" or unrecognized strings.
pub fn parse_color(input: &str) -> Option<ThemeColor> {
    let trimmed = input.trim().to_lowercase();

    if trimmed == "default" {
        return None;
    }

    if let Some(rgb) = parse_hex_color(&trimmed) {
        return Some(rgb);
    }

    parse_named_color(&trimmed).map(ThemeColor::Named)
}

/// Parses a "#rrggbb" hex string into an Rgb ThemeColor.
fn parse_hex_color(input: &str) -> Option<ThemeColor> {
    if input.len() != 7 || !input.starts_with('#') {
        return None;
    }

    let r = u8::from_str_radix(&input[1..3], 16).ok()?;
    let g = u8::from_str_radix(&input[3..5], 16).ok()?;
    let b = u8::from_str_radix(&input[5..7], 16).ok()?;

    Some(ThemeColor::Rgb(r, g, b))
}

/// Parses a named color string into a NamedColor.
fn parse_named_color(input: &str) -> Option<NamedColor> {
    match input {
        "black" => Some(NamedColor::Black),
        "red" => Some(NamedColor::Red),
        "green" => Some(NamedColor::Green),
        "yellow" => Some(NamedColor::Yellow),
        "blue" => Some(NamedColor::Blue),
        "magenta" => Some(NamedColor::Magenta),
        "cyan" => Some(NamedColor::Cyan),
        "white" => Some(NamedColor::White),
        "dark-gray" | "darkgray" => Some(NamedColor::DarkGray),
        "light-red" | "lightred" => Some(NamedColor::LightRed),
        "light-green" | "lightgreen" => Some(NamedColor::LightGreen),
        "light-yellow" | "lightyellow" => Some(NamedColor::LightYellow),
        "light-blue" | "lightblue" => Some(NamedColor::LightBlue),
        "light-magenta" | "lightmagenta" => Some(NamedColor::LightMagenta),
        "light-cyan" | "lightcyan" => Some(NamedColor::LightCyan),
        _ => None,
    }
}

/// Looks up a theme color by slot name, returning the fallback if not found.
///
/// This is a pure lookup function. The caller provides the fallback
/// ThemeColor to use when the key is absent from the theme.
pub fn lookup_color(theme: &Theme, key: &str, fallback: ThemeColor) -> ThemeColor {
    theme.get(key).copied().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Unit tests (10-02): parse_color
    // Test Budget: 6 behaviors x 2 = 12 max
    // -----------------------------------------------------------------------

    #[test]
    fn given_hex_color_when_parse_color_then_returns_rgb() {
        assert_eq!(parse_color("#ff5733"), Some(ThemeColor::Rgb(255, 87, 51)));
    }

    #[test]
    fn given_hex_all_zeros_when_parse_color_then_returns_rgb_black() {
        assert_eq!(parse_color("#000000"), Some(ThemeColor::Rgb(0, 0, 0)));
    }

    #[test]
    fn given_hex_all_ff_when_parse_color_then_returns_rgb_white() {
        assert_eq!(parse_color("#ffffff"), Some(ThemeColor::Rgb(255, 255, 255)));
    }

    #[test]
    fn given_hex_uppercase_when_parse_color_then_returns_rgb() {
        assert_eq!(parse_color("#FF5733"), Some(ThemeColor::Rgb(255, 87, 51)));
    }

    #[test]
    fn given_named_color_red_when_parse_color_then_returns_named_red() {
        assert_eq!(parse_color("red"), Some(ThemeColor::Named(NamedColor::Red)));
    }

    #[test]
    fn given_named_color_dark_gray_when_parse_color_then_returns_named_dark_gray() {
        assert_eq!(
            parse_color("dark-gray"),
            Some(ThemeColor::Named(NamedColor::DarkGray))
        );
    }

    #[test]
    fn given_named_color_darkgray_when_parse_color_then_returns_named_dark_gray() {
        assert_eq!(
            parse_color("darkgray"),
            Some(ThemeColor::Named(NamedColor::DarkGray))
        );
    }

    #[test]
    fn given_default_when_parse_color_then_returns_none() {
        assert_eq!(parse_color("default"), None);
    }

    #[test]
    fn given_invalid_string_when_parse_color_then_returns_none() {
        assert_eq!(parse_color("not-a-color"), None);
    }

    #[test]
    fn given_short_hex_when_parse_color_then_returns_none() {
        assert_eq!(parse_color("#fff"), None);
    }

    #[test]
    fn given_hex_with_invalid_chars_when_parse_color_then_returns_none() {
        assert_eq!(parse_color("#gggggg"), None);
    }

    // -----------------------------------------------------------------------
    // Unit tests (10-02): lookup_color
    // -----------------------------------------------------------------------

    #[test]
    fn given_theme_with_key_when_lookup_then_returns_theme_color() {
        let mut theme = new_theme();
        theme.insert("text-fg".to_string(), ThemeColor::Rgb(100, 200, 50));

        let result = lookup_color(&theme, "text-fg", ThemeColor::Named(NamedColor::White));
        assert_eq!(result, ThemeColor::Rgb(100, 200, 50));
    }

    #[test]
    fn given_theme_without_key_when_lookup_then_returns_fallback() {
        let theme = new_theme();

        let result = lookup_color(&theme, "missing-key", ThemeColor::Named(NamedColor::White));
        assert_eq!(result, ThemeColor::Named(NamedColor::White));
    }
}
