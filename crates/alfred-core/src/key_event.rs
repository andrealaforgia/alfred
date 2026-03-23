//! KeyEvent: represents a single key input with modifier keys.
//!
//! KeyEvent captures both the key pressed (KeyCode) and any modifier keys
//! held simultaneously (Ctrl, Alt, Shift). This is a pure data type with
//! no I/O dependencies.

/// The key that was pressed, independent of modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A character key (e.g. 'a', 'Z', '3', '!').
    Char(char),
    /// Arrow keys.
    Up,
    Down,
    Left,
    Right,
    /// Special keys.
    Enter,
    Escape,
    Backspace,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
}

/// Modifier keys held during a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    /// No modifier keys held.
    pub fn none() -> Self {
        Modifiers {
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    /// Only the Ctrl key held.
    pub fn ctrl() -> Self {
        Modifiers {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }
}

/// A single key input event combining the key pressed and any modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Creates a KeyEvent with the given key code and modifiers.
    pub fn new(code: KeyCode, modifiers: Modifiers) -> Self {
        KeyEvent { code, modifiers }
    }

    /// Creates a KeyEvent with no modifier keys.
    pub fn plain(code: KeyCode) -> Self {
        KeyEvent {
            code,
            modifiers: Modifiers::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Unit tests: Modifiers
    // -----------------------------------------------------------------------

    #[test]
    fn modifiers_none_has_all_false() {
        let mods = Modifiers::none();
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.shift);
    }

    #[test]
    fn modifiers_ctrl_has_only_ctrl_true() {
        let mods = Modifiers::ctrl();
        assert!(mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.shift);
    }

    // -----------------------------------------------------------------------
    // Unit tests: KeyEvent construction
    // -----------------------------------------------------------------------

    #[test]
    fn key_event_new_stores_code_and_modifiers() {
        let event = KeyEvent::new(KeyCode::Char('q'), Modifiers::ctrl());
        assert_eq!(event.code, KeyCode::Char('q'));
        assert!(event.modifiers.ctrl);
    }

    #[test]
    fn key_event_plain_creates_event_with_no_modifiers() {
        let event = KeyEvent::plain(KeyCode::Up);
        assert_eq!(event.code, KeyCode::Up);
        assert!(!event.modifiers.ctrl);
        assert!(!event.modifiers.alt);
        assert!(!event.modifiers.shift);
    }
}
