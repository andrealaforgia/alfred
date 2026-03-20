//! Shared test helpers for Alfred acceptance tests.
//!
//! These helpers provide test fixture construction and assertion utilities
//! that abstract away implementation details. All tests interact with
//! the editor through its public API surface (driving ports), never
//! through internal modules.
//!
//! # Test Isolation
//!
//! Each test uses temporary directories for file fixtures and plugin
//! directories. No shared mutable state between tests.
//!
//! # Driving Ports Used
//!
//! - `EditorState` — top-level state container (alfred-core)
//! - `LispRuntime` — Lisp evaluation (alfred-lisp)
//! - `PluginRegistry` — plugin lifecycle (alfred-plugin)
//! - `App` — application event processing (alfred-tui)

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Re-exports from production crates (driving ports only)
// ---------------------------------------------------------------------------
// These imports will resolve once the production crates exist.
// Until then, tests compile but fail at the import level, which is correct
// for acceptance tests written before implementation.
//
// use alfred_core::{
//     Buffer, Cursor, EditorState, KeyEvent, KeyCode, Modifiers,
//     Position, Viewport, CommandRegistry, KeymapRegistry, HookRegistry,
//     ModeManager,
// };
// use alfred_lisp::LispRuntime;
// use alfred_plugin::PluginRegistry;
// use alfred_tui::App;

// ---------------------------------------------------------------------------
// File fixture helpers
// ---------------------------------------------------------------------------

/// Creates a temporary directory with a file containing the given content.
/// Returns the TempDir (must be kept alive for the duration of the test)
/// and the path to the created file.
pub fn create_temp_file(filename: &str, content: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = dir.path().join(filename);
    fs::write(&file_path, content).expect("Failed to write temp file");
    (dir, file_path)
}

/// Creates an empty temporary directory for use as a plugins directory.
/// Returns the TempDir (must be kept alive) and its path.
pub fn create_empty_plugins_dir() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp directory");
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Creates a temporary plugins directory with a single test plugin.
/// The plugin has an init.lisp that registers a command with the given name.
/// Returns the TempDir and the path to the plugins directory.
pub fn create_test_plugin_dir(plugin_name: &str, init_lisp_content: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp directory");
    let plugin_dir = dir.path().join(plugin_name);
    fs::create_dir_all(&plugin_dir).expect("Failed to create plugin directory");
    let init_path = plugin_dir.join("init.lisp");
    fs::write(&init_path, init_lisp_content).expect("Failed to write init.lisp");
    (dir, dir.path().to_path_buf())
}

/// Creates a temporary plugins directory with multiple plugins.
/// Each entry is (plugin_name, init_lisp_content).
pub fn create_multi_plugin_dir(plugins: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp directory");
    for (name, content) in plugins {
        let plugin_dir = dir.path().join(name);
        fs::create_dir_all(&plugin_dir).expect("Failed to create plugin directory");
        let init_path = plugin_dir.join("init.lisp");
        fs::write(&init_path, content).expect("Failed to write init.lisp");
    }
    (dir, dir.path().to_path_buf())
}

// ---------------------------------------------------------------------------
// Key event simulation helpers
// ---------------------------------------------------------------------------

/// Represents a simulated key press for testing.
/// This mirrors the shape of alfred_core::KeyEvent but is defined here
/// so tests compile before the production crate exists.
#[derive(Debug, Clone, PartialEq)]
pub struct TestKeyEvent {
    pub code: TestKeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Key codes for test simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum TestKeyCode {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Tab,
}

impl TestKeyEvent {
    pub fn char(c: char) -> Self {
        Self {
            code: TestKeyCode::Char(c),
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn ctrl(c: char) -> Self {
        Self {
            code: TestKeyCode::Char(c),
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    pub fn escape() -> Self {
        Self {
            code: TestKeyCode::Escape,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn backspace() -> Self {
        Self {
            code: TestKeyCode::Backspace,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn arrow_up() -> Self {
        Self {
            code: TestKeyCode::Up,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn arrow_down() -> Self {
        Self {
            code: TestKeyCode::Down,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn arrow_left() -> Self {
        Self {
            code: TestKeyCode::Left,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn arrow_right() -> Self {
        Self {
            code: TestKeyCode::Right,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
}

/// Converts a string into a sequence of character key events.
/// Useful for simulating typing a word.
pub fn type_string(text: &str) -> Vec<TestKeyEvent> {
    text.chars().map(TestKeyEvent::char).collect()
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Asserts that the buffer content matches the expected string.
/// Placeholder: will delegate to EditorState.buffer.content() once available.
pub fn assert_buffer_content(_state: &(), _expected: &str) {
    // Will be: assert_eq!(state.buffer.content(), expected);
    unimplemented!("Requires alfred-core::EditorState");
}

/// Asserts the cursor is at the expected line and column (zero-indexed).
pub fn assert_cursor_position(_state: &(), _expected_line: usize, _expected_col: usize) {
    // Will be: assert_eq!(state.cursor.line, expected_line);
    //          assert_eq!(state.cursor.column, expected_col);
    unimplemented!("Requires alfred-core::EditorState");
}

/// Asserts the editor is in the expected mode.
pub fn assert_mode(_state: &(), _expected_mode: &str) {
    // Will be: assert_eq!(state.mode, expected_mode);
    unimplemented!("Requires alfred-core::EditorState");
}

/// Asserts that a command with the given name is registered.
pub fn assert_command_registered(_state: &(), _command_name: &str) {
    // Will be: assert!(state.commands.has(command_name));
    unimplemented!("Requires alfred-core::EditorState");
}

/// Asserts that the editor is signaling quit.
pub fn assert_editor_quit(_state: &()) {
    // Will be: assert!(!state.running);
    unimplemented!("Requires alfred-core::EditorState");
}

/// Asserts that the message area contains the expected text.
pub fn assert_message(_state: &(), _expected: &str) {
    // Will be: assert_eq!(state.message.as_deref(), Some(expected));
    unimplemented!("Requires alfred-core::EditorState");
}
