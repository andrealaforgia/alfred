# ADR-002: Plugin-First Architecture

## Status

Accepted

## Context

Alfred must decide how much functionality lives in the Rust kernel versus the Lisp extension layer. Three positions exist on this spectrum: (1) full-featured kernel with optional plugins for extras, (2) balanced split with some features in kernel and some as plugins, (3) thin kernel where everything beyond primitives is a plugin.

Evidence from editor case studies:
- Emacs: ~70% Lisp, ~30% C. Even fundamental features like cursor movement commands are Lisp
- Neovim: Built-in LSP client is a Lua plugin, proving non-trivial features work in the extension language
- Helix: No plugin system -- most-cited limitation by the community
- Kakoune: Minimal extensibility via shell -- limits ecosystem growth

The project's goal is to prove that AI agents can build architecturally sound, modular software. The strongest proof is a system where modal editing (a complex, stateful feature) works entirely as a plugin.

Quality attributes: **extensibility** (users can change any behavior), **testability** (plugins are isolated), **maintainability** (features are decoupled from kernel).

## Decision

Adopt a plugin-first architecture. The kernel provides only core primitives (buffer operations, cursor movement, keymap resolution, hook dispatch, rendering infrastructure). All user-visible features -- keybindings, line numbers, status bar, modal editing -- are Lisp plugins.

## Alternatives Considered

### Alternative 1: Full-featured kernel with optional plugins
- **What**: Build keybindings, status bar, line numbers, and basic editing directly in Rust. Plugins add extra features
- **Expected impact**: Faster initial development, simpler architecture
- **Why rejected**: Does not prove the architecture. If keybindings are in Rust, the plugin system is untested for its most important use case. This is the Helix approach -- and Helix's lack of extensibility is its most criticized limitation

### Alternative 2: Balanced split (some in kernel, some as plugins)
- **What**: Basic keybindings and status bar in Rust, modal editing and line numbers as plugins
- **Expected impact**: Moderate proof of architecture, moderate development speed
- **Why rejected**: Creates a blurry boundary. How to decide what goes where? Each feature in the kernel is a missed opportunity to validate the plugin API. The walking skeleton's purpose is to push the boundary as far toward plugins as possible

## Consequences

### Positive
- The plugin API is battle-tested by the walking skeleton itself (line numbers, status bar, basic keybindings, vim keybindings)
- Clean kernel boundary -- the kernel is small and focused
- Every feature is independently removable without modifying the kernel
- Forces the API to be sufficient for real use cases
- Proves the architecture claim ("everything is a plugin") end-to-end

### Negative
- More Lisp code to write for basic features
- Plugin API must be designed well enough for complex features (modal editing) -- higher upfront design cost
- Performance-sensitive operations (per-keystroke evaluation) run through the Lisp interpreter
- Debugging may require understanding both Rust and Lisp code paths
