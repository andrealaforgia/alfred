# ADR-006: Cargo Workspace Crate Structure

## Status

Accepted

## Context

Alfred needs a modular code organization that enforces architectural boundaries at compile time. Rust's Cargo workspaces provide crate-level visibility enforcement -- code in one crate cannot access private items in another. This makes crate boundaries an architectural enforcement mechanism, not just an organizational choice.

The key architectural constraint: `alfred-core` must have zero dependencies on other Alfred crates. All dependencies must point inward toward core.

## Decision

Organize Alfred as a Cargo workspace with 5 crates:

| Crate | Responsibility | Depends On |
|-------|---------------|-----------|
| `alfred-core` | Pure editing logic: buffer, cursor, commands, keymaps, hooks, modes, types | ropey only |
| `alfred-lisp` | Lisp interpreter integration, FFI bridge, primitive registration | alfred-core (types only), rust_lisp |
| `alfred-plugin` | Plugin discovery, loading, lifecycle, registry | alfred-core, alfred-lisp |
| `alfred-tui` | Event loop, terminal rendering, input handling | alfred-core, crossterm, ratatui |
| `alfred-bin` | Binary entry point, CLI parsing, initialization | alfred-tui, alfred-plugin, alfred-lisp, alfred-core |

## Alternatives Considered

### Alternative 1: Single crate with modules
- **What**: One crate (`alfred`) with modules (`buffer`, `lisp`, `tui`, etc.)
- **Expected impact**: Simpler build, no inter-crate dependency management
- **Why rejected**: Module-level visibility (`pub(crate)`) is weaker than crate-level boundaries. Nothing prevents the buffer module from importing terminal I/O types. The architectural constraint (core has no outward dependencies) is not compiler-enforced. For a project showcasing architecture quality, this is insufficient

### Alternative 2: More granular crates (6+ crates, separate keymap, hook, command crates)
- **What**: Separate crates for each alfred-core subsystem (alfred-keymap, alfred-hook, alfred-command)
- **Expected impact**: Maximum isolation, finest-grained boundaries
- **Why rejected**: Over-decomposition. The keymap, hook, and command systems are tightly related (a key resolves to a command, commands fire hooks). Separating them into crates creates circular dependency pressure and excessive inter-crate API surface. The 5-crate structure provides the right granularity: one crate per distinct concern (core logic, Lisp, plugins, UI, binary)

## Consequences

### Positive
- `alfred-core` purity enforced by Cargo -- cannot accidentally import crossterm or ratatui
- Each crate compiles independently, enabling parallel compilation
- Crate boundaries are visible in the dependency graph (see architecture doc)
- Aligns with Helix's proven crate organization pattern

### Negative
- Inter-crate type sharing requires careful public API design
- Types used across crates must be in `alfred-core/types.rs` (single source of truth)
- Refactoring across crate boundaries requires updating multiple Cargo.toml files
