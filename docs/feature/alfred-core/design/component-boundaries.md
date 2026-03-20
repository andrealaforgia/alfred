# Alfred Editor -- Component Boundaries

**Feature**: alfred-core
**Date**: 2026-03-19

---

## 1. Crate Dependency Graph

```
                    alfred-bin
                   /    |    \     \
                  v     v     v     v
           alfred-tui  alfred-plugin  alfred-lisp
              |            |    \        |
              v            v     v       v
           alfred-core  alfred-core  alfred-core
              |
              v
            ropey
```

**Dependency rule**: All arrows point inward toward `alfred-core`. No crate depends on a crate at the same level or above.

```
alfred-core    <- depends on: ropey
alfred-lisp    <- depends on: alfred-core, rust_lisp
alfred-plugin  <- depends on: alfred-core, alfred-lisp
alfred-tui     <- depends on: alfred-core, crossterm, ratatui
alfred-bin     <- depends on: alfred-core, alfred-lisp, alfred-plugin, alfred-tui
```

---

## 2. Public API Surface Per Crate

### alfred-core

The pure kernel. No I/O, no terminal, no Lisp. Only ropey as external dependency.

**Public types**:
- `Buffer` -- Rope wrapper with metadata (id, filename, modified, version)
- `Cursor` -- Position (line, column) with movement logic
- `Viewport` -- Visible area tracking (top_line, height, width)
- `KeyEvent` -- Key code + modifiers (mirrors crossterm's type for decoupling)
- `KeySequence` -- Vec of KeyEvents for multi-key bindings
- `CommandRegistry` -- Register/lookup named commands
- `CommandHandler` -- Enum: Native(Rust fn) or Lisp(function reference)
- `KeymapRegistry` -- Create/manage/resolve keymaps
- `Keymap` -- Map from KeySequence to command name
- `HookRegistry` -- Register/dispatch named hooks
- `ModeManager` -- Current mode state, mode transitions
- `EditorState` -- Top-level state container aggregating all above
- `Position` -- (line, column) value type
- `AlfredError` -- Unified error type

**Public functions** (pure, no side effects):
- `buffer::insert_at(buffer, position, text) -> Buffer`
- `buffer::delete_range(buffer, start, end) -> Buffer`
- `buffer::get_line(buffer, line_num) -> Option<&str>`
- `buffer::line_count(buffer) -> usize`
- `cursor::move_cursor(cursor, direction, amount, buffer) -> Cursor`
- `cursor::ensure_within_bounds(cursor, buffer) -> Cursor`
- `keymap::resolve(keymaps, active_stack, key_sequence) -> ResolveResult`
- `command::execute(registry, command_name, state) -> Result<()>`
- `hook::dispatch(registry, hook_name, args) -> Result<()>`

### alfred-lisp

Lisp interpreter integration. The FFI bridge between Rust and Lisp.

**Public types**:
- `LispRuntime` -- Wrapper around the adopted interpreter
- `LispValue` -- Lisp value type (for conversion to/from Rust types)

**Public functions**:
- `runtime::new() -> LispRuntime`
- `runtime::eval(runtime, source: &str) -> Result<LispValue>`
- `runtime::eval_file(runtime, path: &Path) -> Result<LispValue>`
- `runtime::register_primitive(runtime, name: &str, fn) -> Result<()>`
- `bridge::register_core_primitives(runtime, state: &mut EditorState) -> Result<()>`

### alfred-plugin

Plugin discovery, loading, and lifecycle management.

**Public types**:
- `PluginRegistry` -- Track all discovered/loaded plugins
- `PluginMetadata` -- Name, version, description, dependencies
- `PluginStatus` -- Discovered, Loaded, Active, Error

**Public functions**:
- `discovery::scan(directory: &Path) -> Vec<PluginMetadata>`
- `registry::load_all(registry, runtime: &mut LispRuntime, state: &mut EditorState) -> Result<()>`
- `registry::unload(registry, plugin_name: &str, state: &mut EditorState) -> Result<()>`
- `registry::list_plugins(registry) -> &[PluginMetadata]`

### alfred-tui

Terminal UI. The imperative shell.

**Public types**:
- `App` -- Application state and event loop runner
- `Renderer` -- Rendering logic (calls hooks, composites frame)

**Public functions**:
- `app::new(state: EditorState, runtime: LispRuntime, registry: PluginRegistry) -> App`
- `app::run(app: &mut App) -> Result<()>` -- Main event loop (blocks until quit)

### alfred-bin

Binary entry point. Minimal code.

**Public**: None (binary crate, has `main()` only)

---

## 3. Kernel vs Plugin Boundary

### In the Kernel (Rust)

Everything that cannot be a plugin because it is the infrastructure plugins depend on:

| Component | Why in Kernel |
|-----------|--------------|
| Rope buffer (ropey wrapper) | Performance-critical data structure. Must be Rust for O(log n) guarantees |
| Cursor/position logic | Pure computation, performance-critical, used by every keystroke |
| Keymap resolver | Must resolve keys before Lisp evaluation. Performance-critical per-keystroke path |
| Command registry | Infrastructure for command dispatch. Plugins register into it |
| Hook registry | Infrastructure for hook dispatch. Plugins register into it |
| Mode manager | Stores mode state. Plugins set/read it |
| Event loop | Reads terminal input, drives render cycle. Cannot be in Lisp (blocks on I/O) |
| Terminal rendering | Writes to terminal via crossterm/ratatui. Cannot be in Lisp |
| Lisp runtime | The interpreter itself runs in Rust |
| Plugin loader | Discovers and loads Lisp files. Cannot be a Lisp plugin (bootstrap problem) |

### In Plugins (Lisp)

Everything that is a user-visible feature or behavior choice:

| Component | Why a Plugin |
|-----------|-------------|
| Line numbers | Rendering choice. Some users want them, some don't |
| Status bar | Content and layout are customizable |
| Basic keybindings | Different users want different bindings |
| Vim keybindings | Modal editing is a behavior choice, not a kernel concern |
| Future: Emacs keybindings | Alternative binding scheme as a plugin |
| Future: syntax highlighting | Language-specific, swappable |
| Future: autocomplete | Feature, not infrastructure |

### The Boundary Test

A component belongs in the kernel if and only if:
1. It is infrastructure that plugins depend on (command registry, hook system, keymap system), OR
2. It requires direct hardware/OS access (terminal I/O, file system), OR
3. It is performance-critical and must avoid Lisp evaluation overhead (rope operations, key resolution)

If none of these apply, it is a plugin.

---

## 4. Cross-Crate Communication Patterns

### alfred-tui -> alfred-core
- Reads `EditorState` fields for rendering (buffer content, cursor position, viewport)
- Calls `keymap::resolve()` to resolve key events to commands
- Calls `command::execute()` to run commands
- Calls `hook::dispatch()` for render hooks

### alfred-lisp -> alfred-core
- The bridge module accesses `EditorState` through a shared mutable reference
- Lisp primitives read/write buffer, cursor, mode, hooks, commands, keymaps
- All access through `alfred-core`'s public API, never through internal fields

### alfred-plugin -> alfred-lisp
- Calls `runtime::eval_file()` to load plugin source
- Calls `runtime::eval()` to invoke plugin init/cleanup functions

### alfred-plugin -> alfred-core
- Reads `EditorState` to register plugin-provided commands, hooks, keymaps
- On unload, removes plugin's registrations from command/hook/keymap registries

### alfred-bin -> all
- Creates `EditorState`, `LispRuntime`, `PluginRegistry`
- Calls `bridge::register_core_primitives()` to set up FFI
- Calls `registry::load_all()` to discover and load plugins
- Calls `app::run()` to enter the event loop

---

## 5. Shared State Model

`EditorState` is the single mutable state container. It is owned by the event loop (`alfred-tui::App`). Access patterns:

- **During event processing**: `&mut EditorState` passed to command execution and Lisp evaluation
- **During rendering**: `&EditorState` (immutable borrow) passed to renderer
- **No concurrent access**: Single-threaded, synchronous execution means no need for locks or atomics

This is deliberately simple. The single-owner model avoids Rust's borrow checker complexity for shared mutable state (no `Rc<RefCell<T>>`, no `Arc<Mutex<T>>`).
