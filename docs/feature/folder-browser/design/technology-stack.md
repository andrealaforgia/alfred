# Technology Stack: Folder Browser

## Principle

No new dependencies. The folder browser uses only existing workspace dependencies and Rust standard library. This aligns with the constraint "no new external dependencies preferred."

---

## Dependencies Used

| Component | Dependency | Version | License | Justification |
|-----------|-----------|---------|---------|--------------|
| Directory reading | `std::fs` (Rust stdlib) | N/A | MIT/Apache-2.0 | Standard library filesystem operations. `read_dir()`, `metadata()`, `canonicalize()` |
| Path handling | `std::path` (Rust stdlib) | N/A | MIT/Apache-2.0 | `Path`, `PathBuf`, `is_dir()`, `is_file()`, `parent()` |
| Sorting | `std::cmp` (Rust stdlib) | N/A | MIT/Apache-2.0 | Custom sort: directories first, case-insensitive alphabetical |
| Text buffer | `ropey` (existing) | workspace | MIT | Used for `Buffer::from_file()` when opening a file from the browser |
| TUI rendering | `ratatui` (existing) | workspace | MIT | Browser view rendering in the terminal |
| Terminal IO | `crossterm` (existing) | workspace | MIT | Raw mode, key events, cursor control |
| State management | `alfred-core` (workspace) | N/A | project | EditorState, commands, keymaps, panels |
| Lisp keymap | `alfred-lisp` (workspace) | N/A | project | browse-mode plugin uses existing make-keymap/define-key primitives |

## New Dependencies

None.

## Rejected Alternatives

### tree-sitter for directory structure
- **What**: Use tree-sitter to parse directory trees into a structured representation
- **Why rejected**: Massive overkill. `std::fs::read_dir()` is sufficient. Tree-sitter is for source code parsing.

### walkdir crate
- **What**: Recursive directory walking crate (MIT license, well-maintained)
- **Why rejected**: The browser navigates one directory level at a time (not recursive). `std::fs::read_dir()` does exactly this. Adding a dependency for unused recursive capability is unnecessary.

### ignore crate (gitignore-aware walking)
- **What**: Crate that respects `.gitignore` patterns when walking directories
- **Why rejected**: Initial scope shows all files including dotfiles. Gitignore filtering is a future enhancement ("Could Have" in MoSCoW). If needed later, this could be added without architectural changes.
