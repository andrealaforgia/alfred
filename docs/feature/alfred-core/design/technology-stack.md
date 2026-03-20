# Alfred Editor -- Technology Stack

**Feature**: alfred-core
**Date**: 2026-03-19

---

## Stack Overview

All components are open source with permissive licenses. No proprietary dependencies.

| Component | Technology | Version | License | GitHub Stars | Last Release | Rationale |
|-----------|-----------|---------|---------|-------------|-------------|-----------|
| Language | Rust | stable 1.82+ | MIT/Apache-2.0 | N/A | Continuous | Safety + performance. Helix, Zed, Xi validate Rust for editors |
| Text buffer | ropey | 1.x | MIT | ~2.2k | Active | O(log n) all operations. Used by Helix in production |
| Terminal I/O | crossterm | 0.28+ | MIT | ~3.2k | Active | Cross-platform (Win/Mac/Linux). Default ratatui backend |
| TUI framework | ratatui | 0.29+ | MIT | ~10k+ | Active | Immediate-mode rendering, diff-based updates, rich widget set |
| Lisp interpreter | See ADR-004 | - | MIT or similar | - | - | Adopted, not built. Janet or rust_lisp |
| Build system | Cargo workspaces | stable | MIT/Apache-2.0 | N/A | N/A | Standard Rust multi-crate organization |

---

## Detailed Rationale

### Rust

**Why**: Rust is the only language that provides both memory safety and C-level performance without a garbage collector. For an editor that will run untrusted plugin code, safety is critical. The borrow checker prevents entire categories of bugs (null pointers, buffer overflows, data races) at compile time.

**Alternatives rejected**:
- **C**: No memory safety. Every buffer operation is a potential vulnerability
- **Go**: GC causes latency spikes unsuitable for low-latency interactive applications
- **Zig**: Promising but smaller ecosystem, fewer available libraries for TUI/rope/terminal
- **C++**: Memory safety opt-in, not enforced. Higher risk for a single-developer project

**Evidence**: Helix, Zed, and Xi are all written in Rust, demonstrating the language's suitability for editor development.

### ropey

**Why**: Provides O(log n) guarantees for all text operations (insert, delete, line access) regardless of edit pattern. Critical for large files and non-localized editing patterns. Cheap cloning via reference counting enables future snapshot-based undo.

**Alternatives rejected**:
- **Gap buffer**: O(1) local but O(n) distant edits. Poor multi-cursor performance. Used by Emacs but predates modern requirements
- **Piece table**: Viable but higher implementation complexity (VS Code uses a piece tree variant). No mature Rust crate available
- **Vec\<String\>**: Simple but O(n) insertions, no sub-line operations, unsuitable for anything beyond toy editors

**Evidence**: Used by Helix editor in production handling multi-GB files.

### crossterm

**Why**: Cross-platform terminal manipulation (Windows, macOS, Linux). Handles raw mode, key events, mouse events, ANSI escape sequences. The default and most tested backend for ratatui.

**Alternatives rejected**:
- **termion**: Unix-only, no Windows support
- **termwiz**: Less community adoption, primarily used by wezterm
- **Raw ANSI escape sequences**: Unreliable across terminal emulators, no abstraction

### ratatui

**Why**: Immediate-mode TUI framework. Does not impose an application structure -- Alfred manages its own state and event loop. Double-buffering with diff-based updates means rendering scales with changes, not screen size. Rich widget set (layouts, text, blocks, tables) available when needed.

**Alternatives rejected**:
- **cursive**: Retained-mode, imposes callback-heavy structure unsuitable for editor's tight event loop
- **Custom rendering**: Unnecessary complexity. ratatui handles the hard parts (buffering, diffing, escape sequence generation)
- **No framework (raw crossterm)**: Would require reimplementing double-buffering and diff-based rendering

### Lisp Interpreter (ADR-004)

Two candidates. Final selection at M2 start based on hands-on evaluation.

**Janet**: Entire language <1MB. Single C source + header. Built-in green threads, event loop, PEG parser, C FFI. Clojure-inspired data structures. Risk: C dependency requires FFI bridge from Rust (cc crate or bindgen).

**rust_lisp**: Native Rust. No FFI bridge. `lisp!` macro embeds Lisp syntax in Rust. `Value::Foreign()` wraps Rust types. Risk: Smaller community, less battle-tested.

Both are MIT-licensed.
