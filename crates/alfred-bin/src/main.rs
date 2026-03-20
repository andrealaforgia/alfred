//! Alfred -- an Emacs-like text editor.
//!
//! Binary entry point that wires together all crates.

fn main() {
    println!(
        "Alfred v{} (core: {}, tui: {}, lisp: {}, plugin: {})",
        env!("CARGO_PKG_VERSION"),
        alfred_core::version(),
        alfred_tui::version(),
        if alfred_lisp::available() { "ready" } else { "stub" },
        if alfred_plugin::available() { "ready" } else { "stub" },
    );
}
