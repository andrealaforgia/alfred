//! Alfred -- an Emacs-like text editor.
//!
//! Binary entry point that wires together all crates.
//! Parses CLI arguments, loads file into buffer, creates the Lisp runtime,
//! registers bridge primitives, discovers and loads plugins, then runs
//! the event loop.

use std::cell::RefCell;
use std::path::Path;
use std::process;
use std::rc::Rc;

use alfred_core::buffer::Buffer;
use alfred_core::editor_state;
use alfred_lisp::bridge;
use alfred_lisp::runtime::LispRuntime;
use alfred_plugin::{discovery, registry};

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
        let buffer = Buffer::from_file(path)?;
        state.borrow_mut().buffer = buffer;
    }

    // Create Lisp runtime and register bridge primitives
    let runtime = LispRuntime::new();
    bridge::register_core_primitives(&runtime, state.clone());
    bridge::register_define_command(&runtime, state.clone());
    bridge::register_hook_primitives(&runtime, state.clone());

    // Discover and load plugins
    let plugin_errors = load_plugins(&runtime);

    // Display plugin load errors as messages (not crashes)
    if !plugin_errors.is_empty() {
        let error_summary = plugin_errors.join("; ");
        state.borrow_mut().message = Some(format!("Plugin errors: {}", error_summary));
    }

    alfred_tui::app::run(&state, &runtime)?;

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
