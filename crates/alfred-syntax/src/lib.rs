//! Alfred Syntax -- tree-sitter syntax highlighting for the Alfred editor.
//!
//! This crate owns all tree-sitter interaction: parsing, grammar management,
//! highlight query execution, and production of highlight ranges.
//! It depends on alfred-core for Buffer and ThemeColor types.

pub mod highlighter;
pub mod language;
