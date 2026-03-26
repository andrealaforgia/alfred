# Coding Instructions for Alfred Development

## Commit Strategy
- Commit in small steps — each commit should be a single logical change
- Every commit must pass: `cargo fmt`, `cargo clippy --workspace -- -D warnings`, `cargo test`
- Push after each commit

## Testing Requirements
- NEVER skip any tests
- All unit tests must pass after every change
- Run E2E tests (`make e2e`) after each significant phase of development
- Run the alf test design reviewer agent after significant chunks to verify:
  - Mock tautology percentage == 0%
  - Farley Index is in the "Excellent" range (>= 7.5/10)
- Add E2E tests for every scenario not currently covered

## rust_lisp Constraints (CRITICAL)
- No `""` empty string literals — use `(str-concat (list))` or a variable like `browser-empty-str`
- No `#f` — use `nil` for false
- No `"\n"` — use `newline` variable (registered as a Lisp constant)
- No `(define ...)` inside `(begin ...)` inside lambdas — use top-level defines or helper functions
- Equality operator is `=` (registered alias for `==`)
- Boolean true is any non-nil value; false is `nil`

## Architecture Principles
- Functional-core / imperative-shell: domain logic is pure, IO at boundaries
- Plugin-first: everything configurable via Lisp plugins
- Core has ZERO knowledge of any specific plugin
- All plugin-specific behavior lives in Lisp plugins at `plugins/`
- Panel system is generic — no hardcoded panel names in Rust

## Quality Gates Before Merge
1. `cargo fmt --check` — formatting clean
2. `cargo clippy --workspace -- -D warnings` — zero warnings
3. `cargo test` — all unit tests pass
4. `make e2e` — all E2E tests pass
5. No mock tautology
6. Farley Index >= 7.5
