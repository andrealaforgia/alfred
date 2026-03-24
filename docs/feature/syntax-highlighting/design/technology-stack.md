# Syntax Highlighting -- Technology Stack

**Feature**: syntax-highlighting
**Date**: 2026-03-24

---

## New Dependencies

| Crate | Version | License | GitHub Stars | Last Release | Purpose | Alternative Considered |
|-------|---------|---------|-------------|-------------|---------|----------------------|
| `tree-sitter` | 0.24+ | MIT | 18k+ | Active (monthly) | Parser runtime: incremental, error-tolerant parsing | syntect (regex-based, no incremental parse) |
| `tree-sitter-rust` | 0.23+ | MIT | 300+ | Active | Compiled Rust grammar | None (only official grammar) |
| `tree-sitter-python` | 0.23+ | MIT | 400+ | Active | Compiled Python grammar | None (only official grammar) |
| `tree-sitter-javascript` | 0.23+ | MIT | 300+ | Active | Compiled JavaScript grammar | None (only official grammar) |

**All dependencies are MIT-licensed open source.** No proprietary additions.

---

## Existing Dependencies (Unchanged)

| Crate | Used By | License | Role |
|-------|---------|---------|------|
| `ropey` 1.x | alfred-core | MIT | Rope-based text buffer |
| `crossterm` 0.28 | alfred-tui, alfred-bin | MIT | Terminal I/O |
| `ratatui` 0.29 | alfred-tui | MIT | TUI rendering |
| `rust_lisp` | alfred-lisp | MIT | Lisp interpreter |
| `thiserror` 1.x | alfred-core | MIT/Apache-2.0 | Error types |

---

## Binary Size Impact

| Component | Estimated Size |
|-----------|---------------|
| tree-sitter runtime | ~100KB |
| Rust grammar | ~300KB |
| Python grammar | ~250KB |
| JavaScript grammar | ~350KB |
| Highlight query strings | ~10KB |
| **Total addition** | **~1.0MB** |

Current binary size is ~5-10MB. Addition of ~1MB is a 10-20% increase. Acceptable for a desktop application.

---

## Build Impact

Grammar crates contain C source code compiled via `cc` crate. This adds:
- C compiler requirement (cc/gcc/clang) -- already standard for Rust development
- ~5-10 seconds additional compile time for grammar C sources (first build only, cached after)
- No runtime C library dependency -- grammars are statically linked

---

## Future Language Additions

Adding a new language requires:
1. Add grammar crate to `alfred-syntax/Cargo.toml` (e.g., `tree-sitter-go = "0.23"`)
2. Add `highlights.scm` file to `queries/{lang}/`
3. Register language in the language config registry

No architectural changes needed. Each additional grammar adds ~200-500KB.
