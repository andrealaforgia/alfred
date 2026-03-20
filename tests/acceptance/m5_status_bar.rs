//! M5 Acceptance Tests: Status Bar Plugin
//!
//! What M5 proves: A plugin can render dynamic UI that reflects editor state
//! (filename, cursor position, modified flag, mode).
//!
//! Driving ports exercised:
//!   - PluginRegistry::load_all() — load status-bar plugin
//!   - HookRegistry::dispatch() — fire render-status-hook
//!   - EditorState — observe status fields, cursor, buffer metadata
//!
//! Tests verify that the render-status-hook produces correct status bar
//! data. They do NOT test visual rendering.

mod helpers;

// ---------------------------------------------------------------------------
// Happy Path
// ---------------------------------------------------------------------------

/// M5-H1: Status bar displays the filename.
///
/// Given the status bar plugin is loaded and a file named "example.txt" is open
/// When the render-status-hook is dispatched
/// Then the status bar data includes the filename "example.txt"
#[test]
#[ignore]
fn given_status_bar_plugin_and_file_when_render_hook_then_shows_filename() {
    // Given: status bar plugin loaded, file "example.txt" open
    let (_dir, _file_path) = helpers::create_temp_file("example.txt", "Some content");

    // Load status-bar plugin, open the file
    // state.buffer.filename = Some(PathBuf::from("example.txt"));

    // When: render-status-hook dispatched
    // let status_data = hook::dispatch(&state.hooks, "render-status-hook", &[]).unwrap();

    // Then: status includes "example.txt"
    // assert!(status_data.contains("example.txt"));

    todo!("Implement when status-bar plugin exists");
}

/// M5-H2: Status bar updates cursor position when cursor moves.
///
/// Given the status bar plugin is loaded and the cursor is at line 5, column 10
/// When the render-status-hook is dispatched
/// Then the status bar shows the position "5:10" (or "6:11" if one-indexed)
#[test]
#[ignore]
fn given_status_bar_plugin_when_cursor_at_5_10_then_status_shows_position() {
    // Given: cursor at (5, 10) zero-indexed
    // state.cursor = Cursor { line: 5, column: 10 };

    // When: render-status-hook dispatched

    // Then: status shows position (6:11 in one-indexed display)
    // let status = get_status_bar_content(&state);
    // assert!(status.contains("6:11"));

    todo!("Implement when status-bar plugin exists");
}

/// M5-H3: Status bar displays the correct filename for the current buffer.
///
/// Given the status bar plugin is loaded
/// And the buffer has filename "main.rs"
/// When the status bar renders
/// Then it displays "main.rs"
#[test]
#[ignore]
fn given_status_bar_plugin_and_buffer_named_main_rs_then_shows_main_rs() {
    // Given: buffer with filename "main.rs"
    let (_dir, _file_path) = helpers::create_temp_file("main.rs", "fn main() {}");

    // state.buffer.filename = Some(PathBuf::from("main.rs"));

    // When: status hook fires
    // Then: shows "main.rs"
    // let status = get_status_bar_content(&state);
    // assert!(status.contains("main.rs"));

    todo!("Implement when status-bar plugin exists");
}

// ---------------------------------------------------------------------------
// Error Path
// ---------------------------------------------------------------------------

/// M5-E1: No status bar plugin means no status bar, no crash.
///
/// Given the status bar plugin is not present
/// When the editor starts
/// Then no status bar is rendered and the editor functions normally
#[test]
#[ignore]
fn given_no_status_bar_plugin_when_editor_starts_then_no_status_bar_and_functional() {
    // Given: no status-bar plugin loaded
    let (_dir, plugins_path) = helpers::create_empty_plugins_dir();
    let _ = plugins_path;

    // When: editor starts
    // Then: no status fields set, editor runs fine
    // assert!(state.status_fields.is_empty());
    // assert!(state.running);

    todo!("Implement when status-bar plugin exists");
}

/// M5-E2: Missing status field handled gracefully.
///
/// Given the status bar plugin is loaded but the buffer has no filename
/// When the status bar renders
/// Then it displays gracefully without crashing
#[test]
#[ignore]
fn given_status_bar_plugin_and_unnamed_buffer_when_renders_then_graceful() {
    // Given: buffer with no filename
    // state.buffer.filename = None;

    // When: render-status-hook fires
    // Then: no crash, status shows a placeholder or omits filename

    todo!("Implement when status-bar plugin exists");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

/// M5-EC1: Unnamed buffer shows a placeholder in the status bar.
///
/// Given the status bar plugin is loaded and the buffer has no file
/// When the status bar renders
/// Then it shows a placeholder like "[No Name]"
#[test]
#[ignore]
fn given_unnamed_buffer_with_status_plugin_then_shows_no_name_placeholder() {
    // Given: unnamed buffer
    // state.buffer.filename = None;

    // When: status rendered
    // let status = get_status_bar_content(&state);

    // Then: shows placeholder
    // assert!(status.contains("[No Name]") || status.contains("[Untitled]"));

    todo!("Implement when status-bar plugin exists");
}

/// M5-EC2: Cursor at (0, 0) shows "1:1" in the status bar.
///
/// Given the status bar plugin is loaded and the cursor is at (0, 0)
/// When the status bar renders
/// Then it shows "1:1" (one-indexed for user display)
#[test]
#[ignore]
fn given_cursor_at_origin_with_status_plugin_then_shows_1_1() {
    // Given: cursor at (0, 0)
    // state.cursor = Cursor { line: 0, column: 0 };

    // When: status rendered
    // let status = get_status_bar_content(&state);

    // Then: shows "1:1"
    // assert!(status.contains("1:1"));

    todo!("Implement when status-bar plugin exists");
}
