//! Alfred -- an Emacs-like text editor.
//!
//! Binary entry point that wires together all crates.
//! Parses CLI arguments, loads file into buffer, creates the Lisp runtime,
//! and runs the event loop.

use std::cell::RefCell;
use std::path::Path;
use std::process;
use std::rc::Rc;

use alfred_core::buffer::Buffer;
use alfred_core::editor_state;
use alfred_lisp::bridge;
use alfred_lisp::runtime::LispRuntime;

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
/// 4. Creates LispRuntime and registers bridge primitives
/// 5. Runs the event loop with the runtime
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

    alfred_tui::app::run(&state, &runtime)?;

    Ok(())
}
