//! Alfred -- an Emacs-like text editor.
//!
//! Binary entry point that wires together all crates.
//! Parses CLI arguments, loads file into buffer, and runs the event loop.

use std::path::Path;
use std::process;

use alfred_core::buffer::Buffer;
use alfred_core::editor_state;

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
/// 2. Creates EditorState with appropriate dimensions
/// 3. Loads file into buffer (if path provided)
/// 4. Runs the event loop
fn run_editor(file_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = crossterm::terminal::size()?;

    let mut state = editor_state::new(width, height);

    if let Some(path_str) = file_path {
        let path = Path::new(path_str);
        let buffer = Buffer::from_file(path)?;
        state.buffer = buffer;
    }

    alfred_tui::app::run(&mut state)?;

    Ok(())
}
