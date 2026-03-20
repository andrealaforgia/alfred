# Alfred Editor -- Data Models

**Feature**: alfred-core
**Date**: 2026-03-19

---

## 1. Core Types

### 1.1 Buffer

The central data structure. Wraps a ropey `Rope` with editor-specific metadata.

| Field | Type | Description |
|-------|------|-------------|
| id | BufferId (usize) | Unique buffer identifier |
| text | Rope | The text content (ropey) |
| filename | Option\<PathBuf\> | Associated file path, if any |
| modified | bool | True if buffer has unsaved changes |
| version | u64 | Monotonically increasing edit counter |
| saved_version | u64 | Version at last save |

**Operations**: insert_at, delete_range, get_line, line_count, char_count, content_slice

### 1.2 Cursor (Position)

Represents the editing position within a buffer.

| Field | Type | Description |
|-------|------|-------------|
| line | usize | Zero-indexed line number |
| column | usize | Zero-indexed column (byte offset within line) |

**Derived**: char_index (absolute position in rope, computed from line+column)

### 1.3 Viewport

Tracks the visible portion of the buffer in the terminal.

| Field | Type | Description |
|-------|------|-------------|
| top_line | usize | First visible line |
| height | u16 | Number of visible rows |
| width | u16 | Number of visible columns |
| gutter_width | u16 | Width reserved for gutter (line numbers, etc.) |

### 1.4 KeyEvent

Represents a single key input from the terminal.

| Field | Type | Description |
|-------|------|-------------|
| code | KeyCode | The key (Char, Enter, Escape, Arrow, F-key, etc.) |
| modifiers | Modifiers | Bitflags: Ctrl, Alt, Shift |

KeyCode and Modifiers align with crossterm's event types. The keymap system works with these directly.

### 1.5 KeySequence

A sequence of one or more KeyEvents, used for multi-key bindings (e.g., `dd`, `C-x C-s`).

| Field | Type | Description |
|-------|------|-------------|
| keys | Vec\<KeyEvent\> | Ordered key events |

### 1.6 Command

A named, callable action.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Unique command identifier (e.g., "delete-line") |
| handler | CommandHandler | Either a Rust function or a Lisp function reference |

CommandHandler is an enum:
- `Native(fn(&mut EditorState) -> Result<()>)` -- Rust-implemented
- `Lisp(LispValue)` -- A Lisp function to evaluate

### 1.7 Keymap

A map from key sequences to command names.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Keymap identifier (e.g., "vim-normal-map") |
| bindings | HashMap\<KeySequence, String\> | Key sequence to command name |
| parent | Option\<String\> | Parent keymap for fallthrough (optional) |

### 1.8 Mode

An editing mode (e.g., normal, insert).

| Field | Type | Description |
|-------|------|-------------|
| name | String | Mode identifier (e.g., "normal") |
| keymap | String | Associated keymap name |

### 1.9 Hook

A named extension point.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Hook identifier (e.g., "after-change-hook") |
| callbacks | Vec\<HookCallback\> | Registered callbacks (Lisp functions) |

### 1.10 EditorState

The top-level mutable state container, passed through the event loop.

| Field | Type | Description |
|-------|------|-------------|
| buffer | Buffer | The current buffer (single buffer for walking skeleton) |
| cursor | Cursor | Current cursor position |
| viewport | Viewport | Current viewport |
| mode | String | Current mode name |
| commands | CommandRegistry | All registered commands |
| keymaps | KeymapRegistry | All defined keymaps |
| active_keymaps | Vec\<String\> | Active keymap stack (top = highest priority) |
| hooks | HookRegistry | All registered hooks |
| message | Option\<String\> | Current message line content |
| status_fields | HashMap\<String, String\> | Named status fields for status bar |
| running | bool | False when editor should quit |

---

## 2. Plugin API Types

### 2.1 PluginMetadata

Parsed from the plugin's init.lisp source.

| Field | Type | Description |
|-------|------|-------------|
| name | String | Plugin name |
| version | String | Semver string |
| description | String | Human-readable description |
| dependencies | Vec\<String\> | Names of required plugins |

### 2.2 PluginState

Runtime state per loaded plugin.

| Field | Type | Description |
|-------|------|-------------|
| metadata | PluginMetadata | Parsed metadata |
| source_path | PathBuf | Path to init.lisp |
| status | PluginStatus | Discovered, Loaded, Active, Error |
| registered_commands | Vec\<String\> | Commands this plugin registered (for cleanup) |
| registered_hooks | Vec\<(String, HookId)\> | Hooks this plugin registered (for cleanup) |
| registered_keymaps | Vec\<String\> | Keymaps this plugin created (for cleanup) |

### 2.3 PluginStatus

Enum tracking plugin lifecycle state.

- `Discovered` -- Found on disk, metadata not yet parsed
- `Loaded` -- Metadata parsed, dependencies resolved
- `Active` -- init() called, commands/hooks/keymaps registered
- `Error(String)` -- Failed to load or init, with reason

---

## 3. Lisp Value Types and FFI Bridge

### 3.1 Lisp Value Representation

The adopted Lisp interpreter will have its own value type. The bridge must convert between Lisp values and Rust types. The mapping for core primitives:

| Lisp Type | Rust Type | Usage |
|-----------|-----------|-------|
| Integer | i64 | Line numbers, counts, positions |
| String | String | Text content, command names, mode names |
| Symbol | String (interned) | Keywords (:normal, :insert), hook names |
| Boolean | bool | Predicate results (buffer-modified?) |
| Nil | () / Option::None | Absent values, void returns |
| List | Vec\<LispValue\> | Arguments, collections |
| Function | LispFunction | Callbacks, command handlers |

### 3.2 Bridge Direction: Rust -> Lisp

Core primitives are registered as native functions in the Lisp environment. Each primitive:
1. Receives `Vec<LispValue>` arguments
2. Validates argument count and types
3. Accesses `EditorState` through a shared reference
4. Performs the operation on Rust types
5. Returns a `LispValue` result

### 3.3 Bridge Direction: Lisp -> Rust

Plugin-defined commands and hook callbacks are stored as `LispValue::Function` references in the command/hook registries. When invoked:
1. The command/hook system retrieves the `LispValue::Function`
2. Passes Rust values converted to `LispValue` arguments
3. Calls the Lisp evaluator
4. Converts the return value back to Rust types if needed

### 3.4 Error Handling

Lisp evaluation errors are caught at the bridge boundary and:
- Displayed as a message to the user (via the message line)
- Logged (when logging exists)
- Never crash the editor process

The bridge converts Lisp errors to `Result<LispValue, AlfredError>`, where `AlfredError` is the unified error type for the editor.
