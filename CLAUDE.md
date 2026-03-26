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

- 6-crate Cargo workspace: alfred-core, alfred-syntax, alfred-lisp, alfred-plugin, alfred-tui, alfred-bin
- Plugin-first architecture: everything beyond core primitives is an Alfred Lisp plugin
- Lisp interpreter: rust_lisp (adopted, not custom-built)
- Syntax highlighting: tree-sitter (Rust, Python, JavaScript grammars)
- TUI: crossterm + ratatui
- Text buffer: ropey (rope data structure)

## Development Instructions

### Commits
- Commit frequently in small change sets
- Each commit must pass: `cargo fmt`, `cargo clippy --workspace -- -D warnings`, `cargo test`

### Testing
- DO NOT SKIP any tests
- Add E2E tests to cover ALL implemented scenarios
- Every new feature must have corresponding E2E test coverage

### Plugin Development Rules
- Everything should be written in Alfred Lisp as much as possible, fully decoupled from Rust code
- Any Rust code required should be limited to providing basic generic API functionality, with ZERO knowledge of the specific plugin
- The core must never reference plugin names, plugin-specific types, or plugin behavior

### Quality Gates (after each significant phase)
1. Run `make ci-local` and `make e2e` — all tests must pass
2. Run a review using the @nw-software-crafter-reviewer agent and fix any issues using @nw-functional-software-crafter
3. Run the alf test design reviewer agent and verify:
   - Farley Index stays in the "Excellent" range (>= 7.5/10)
   - Mock tautology count is 0%

### rust_lisp Constraints
- No `""` empty string literals — use `(str-concat (list))` or a variable
- No `#f` — use `nil` for false
- No `"\n"` — use `newline` variable (registered constant)
- No `(define ...)` inside `(begin ...)` inside lambdas — use top-level defines or helper functions
- Equality operator is `=` (registered alias for `==`)
