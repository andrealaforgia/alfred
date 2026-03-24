//! Alfred -- an Emacs-like text editor.
//!
//! Binary entry point that wires together all crates.
//! Parses CLI arguments, loads file into buffer, creates the Lisp runtime,
//! registers bridge primitives, discovers and loads plugins, then runs
//! the event loop.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

use alfred_core::browser;
use alfred_core::buffer::Buffer;
use alfred_core::editor_state;
use alfred_lisp::bridge;
use alfred_lisp::runtime::LispRuntime;
use alfred_plugin::{discovery, registry};
use alfred_syntax::highlighter::SyntaxHighlighter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = args.get(1).map(|s| s.as_str());

    let result = run_editor(file_path);

    if let Err(err) = result {
        eprintln!("alfred: {}", err);
        process::exit(1);
    }
}

/// Runs the editor with an optional file path.
///
/// This is the composition root: it wires together all crates.
/// 1. Queries terminal size
/// 2. Creates EditorState wrapped in Rc<RefCell> (for shared Lisp access)
/// 3. Loads file into buffer (if path provided)
/// 4. Creates LispRuntime and registers bridge primitives (including define-command)
/// 5. Discovers plugins from `plugins/` directory
/// 6. Resolves plugin load order
/// 7. Loads each plugin in order, collecting errors as messages
/// 8. Runs the event loop with the runtime
fn run_editor(file_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = crossterm::terminal::size()?;

    let state = Rc::new(RefCell::new(editor_state::new(width, height)));

    if let Some(path_str) = file_path {
        let path = Path::new(path_str);
        if path.is_dir() {
            // Directory argument: enter browse mode
            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let entries = read_directory_entries(&canonical);
            let browser_state = browser::new_browser_state(canonical.clone(), canonical, entries);
            let mut s = state.borrow_mut();
            s.browser = Some(browser_state);
            s.mode = browser::MODE_BROWSE.to_string();
            s.active_keymaps = vec!["browse-mode".to_string()];
        } else {
            let buffer = Buffer::from_file(path)?;
            state.borrow_mut().buffer = buffer;
        }
    }

    // Register built-in native commands (cursor movement, delete-backward)
    editor_state::register_builtin_commands(&mut state.borrow_mut());

    // Create Lisp runtime and register bridge primitives
    let runtime = LispRuntime::new();
    bridge::register_core_primitives(&runtime, state.clone());
    bridge::register_define_command(&runtime, state.clone());
    bridge::register_hook_primitives(&runtime, state.clone());
    bridge::register_keymap_primitives(&runtime, state.clone());
    bridge::register_theme_primitives(&runtime, state.clone());
    bridge::register_rendering_primitives(&runtime, state.clone());
    bridge::register_rainbow_csv_primitives(&runtime, state.clone());
    bridge::register_panel_primitives(&runtime, state.clone());
    bridge::register_string_primitives(&runtime);
    bridge::register_list_primitives(&runtime);

    // Discover and load plugins
    let plugin_errors = load_plugins(&runtime);

    // Display plugin load errors as messages (not crashes)
    if !plugin_errors.is_empty() {
        let error_summary = plugin_errors.join("; ");
        state.borrow_mut().message = Some(format!("Plugin errors: {}", error_summary));
    }

    // Load user config file (~/.config/alfred/init.lisp) if it exists
    let home_dir = std::env::var("HOME").unwrap_or_default();
    if !home_dir.is_empty() {
        let config_path = config_file_path(&home_dir);
        if let Some(error_msg) = load_user_config(&runtime, &config_path) {
            // If there were also plugin errors, append; otherwise set
            let mut editor = state.borrow_mut();
            match &editor.message {
                Some(existing) => {
                    editor.message = Some(format!("{}; {}", existing, error_msg));
                }
                None => {
                    editor.message = Some(error_msg);
                }
            }
        }
    }

    let mut highlighter = SyntaxHighlighter::new();
    alfred_tui::app::run(&state, &runtime, &mut highlighter)?;

    Ok(())
}

/// Discovers plugins from `plugins/` relative to CWD, resolves load order,
/// and loads each plugin. Returns a list of error messages for any failures.
fn load_plugins(runtime: &LispRuntime) -> Vec<String> {
    let plugins_dir = Path::new("plugins");

    let (discovered, discovery_errors) = discovery::scan(plugins_dir);

    let mut errors: Vec<String> = discovery_errors.iter().map(|e| e.to_string()).collect();

    let ordered = match registry::resolve_load_order(&discovered) {
        Ok(order) => order,
        Err(e) => {
            errors.push(e.to_string());
            return errors;
        }
    };

    let mut reg = registry::PluginRegistry::new();

    for plugin_meta in ordered {
        if let Err(e) = registry::load_plugin(&mut reg, plugin_meta.clone(), runtime) {
            errors.push(e.to_string());
        }
    }

    // Note: PluginRegistry is consumed here and not stored on EditorState yet.
    // It will be moved to EditorState in a future step when plugin lifecycle
    // management (reload, unload) is needed at runtime.
    let _ = reg;

    errors
}

/// Reads directory entries from the filesystem and converts them to DirEntry values.
fn read_directory_entries(dir: &Path) -> Vec<browser::DirEntry> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    read_dir
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().ok();
            let kind = match file_type {
                Some(ft) if ft.is_dir() => browser::EntryKind::Directory,
                Some(ft) if ft.is_symlink() => {
                    let target_is_dir = entry.path().is_dir();
                    browser::EntryKind::Symlink { target_is_dir }
                }
                _ => browser::EntryKind::File,
            };
            let is_hidden = name.starts_with('.');
            browser::DirEntry {
                name,
                kind,
                is_hidden,
            }
        })
        .collect()
}

/// Computes the path to the user config file from the home directory.
///
/// Returns `~/.config/alfred/init.lisp` using the provided home directory.
/// Pure function: no IO, no environment variable access.
fn config_file_path(home_dir: &str) -> PathBuf {
    Path::new(home_dir)
        .join(".config")
        .join("alfred")
        .join("init.lisp")
}

/// Loads the user config file if it exists, evaluating it via the Lisp runtime.
///
/// Returns `None` if the config file does not exist (silently skipped).
/// Returns `Some(error_message)` if the config file exists but evaluation fails.
fn load_user_config(runtime: &LispRuntime, config_path: &Path) -> Option<String> {
    if !config_path.exists() {
        return None;
    }

    match runtime.eval_file(config_path) {
        Ok(_) => None,
        Err(e) => Some(format!("Config error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unit: config path computation (pure function) --

    #[test]
    fn config_file_path_returns_init_lisp_under_config_alfred() {
        let path = config_file_path("/home/testuser");

        assert_eq!(
            path,
            PathBuf::from("/home/testuser/.config/alfred/init.lisp")
        );
    }

    #[test]
    fn config_file_path_handles_trailing_slash_in_home() {
        // PathBuf::join handles this correctly by design
        let path = config_file_path("/home/testuser/");

        assert_eq!(
            path,
            PathBuf::from("/home/testuser/.config/alfred/init.lisp")
        );
    }

    // -- Integration: load_user_config with real runtime --

    #[test]
    fn load_user_config_returns_none_when_file_does_not_exist() {
        let runtime = LispRuntime::new();
        let nonexistent = Path::new("/nonexistent/path/init.lisp");

        let result = load_user_config(&runtime, nonexistent);

        assert!(result.is_none(), "Should silently skip missing config file");
    }

    #[test]
    fn load_user_config_evaluates_valid_config_file() {
        let runtime = LispRuntime::new();

        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("init.lisp");
        std::fs::write(&config_path, "(define config-loaded 42)").unwrap();

        let result = load_user_config(&runtime, &config_path);

        assert!(result.is_none(), "Valid config should not produce an error");

        // Verify the config was actually evaluated: the variable should be defined
        let value = runtime.eval("config-loaded").unwrap();
        assert_eq!(value.as_integer(), Some(42));
    }

    #[test]
    fn load_user_config_returns_error_message_for_invalid_config() {
        let runtime = LispRuntime::new();

        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("init.lisp");
        std::fs::write(&config_path, "(undefined-function 1 2 3)").unwrap();

        let result = load_user_config(&runtime, &config_path);

        assert!(result.is_some(), "Invalid config should produce an error");
        let error_msg = result.unwrap();
        assert!(
            error_msg.starts_with("Config error:"),
            "Error should be prefixed with 'Config error:', got: {}",
            error_msg
        );
    }

    #[test]
    fn load_user_config_with_bridge_primitives_affects_state() {
        use alfred_core::editor_state;
        use alfred_lisp::bridge;

        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        let runtime = LispRuntime::new();
        bridge::register_core_primitives(&runtime, state.clone());

        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("init.lisp");
        std::fs::write(&config_path, "(message \"hello from config\")").unwrap();

        let result = load_user_config(&runtime, &config_path);

        assert!(result.is_none(), "Valid config should not produce an error");
        assert_eq!(
            state.borrow().message,
            Some("hello from config".to_string()),
            "Config should be able to use bridge primitives like (message ...)"
        );
    }
}
