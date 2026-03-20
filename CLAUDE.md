# Alfred — Emacs-like Text Editor

## Development Paradigm

This project follows the **functional programming** paradigm. Use @nw-functional-software-crafter for implementation.

Rust with functional-core / imperative-shell architecture:
- Types-first design: algebraic data types and domain models before components
- Composition pipelines: data flows through transformation chains
- Pure core / effect shell: domain logic is pure, IO lives at boundaries
- Immutable state: state changes produce new values in the domain
- Property-based testing as default testing strategy

## Mutation Testing Strategy

per-feature

## Project Structure

- 5-crate Cargo workspace: alfred-core, alfred-lisp, alfred-plugin, alfred-tui, alfred-bin
- Plugin-first architecture: everything beyond core primitives is an Alfred Lisp plugin
- Lisp interpreter: rust_lisp (adopted, not custom-built)
- TUI: crossterm + ratatui
- Text buffer: ropey (rope data structure)
