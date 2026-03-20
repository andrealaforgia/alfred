# Walking Skeleton Test Strategy -- Alfred Editor

**Feature**: alfred-core
**Date**: 2026-03-20
**Phase**: DISTILL (Acceptance Test Design)

---

## Walking Skeleton Definition

A walking skeleton is the simplest end-to-end test that proves the system delivers observable user value. It answers: "Can a user accomplish their goal and see the result?"

For Alfred, walking skeletons trace thin vertical slices from user input through the kernel, Lisp layer, and plugin system to observable editor state changes.

---

## Walking Skeleton Inventory

### WS-1: User opens a file and sees its content (M1)

**What it proves**: The kernel can load a file into a rope buffer, and the buffer contains the file's content.

```
Given a file exists with known text content
When the user opens the file in Alfred
Then the buffer contains the file's full text
And the cursor is positioned at the beginning of the file
```

**Driving port**: `EditorState` (constructed with file path, buffer populated from file)
**Observable outcome**: Buffer content matches file content, cursor at (0, 0)

This is the first test to enable. It is the simplest possible proof that the system boots and does something useful.

### WS-2: User evaluates a Lisp expression that modifies the buffer (M2)

**What it proves**: Lisp expressions can call Rust primitives and produce observable changes in editor state.

```
Given the editor has a buffer with text "Hello"
When the user evaluates the expression (buffer-insert " World")
Then the buffer content becomes "Hello World"
```

**Driving port**: `LispRuntime::eval()` operating on `EditorState`
**Observable outcome**: Buffer text changed by Lisp evaluation

### WS-3: Plugin loads and registers a callable command (M3)

**What it proves**: The plugin system discovers, loads, and initializes a Lisp plugin whose command is callable.

```
Given a plugin directory contains a test plugin with an init function
When the editor starts and loads plugins
Then the test plugin's command is registered
And executing the command produces the expected effect on the buffer
```

**Driving port**: `PluginRegistry::load_all()`, then `CommandRegistry::execute()`
**Observable outcome**: Plugin-defined command modifies editor state

### WS-4: User navigates with Lisp-defined keybindings and edits text (M6)

**What it proves**: Plugins can define keybindings that intercept input and mutate the buffer -- no hardcoded key handling.

```
Given the basic keybinding plugin is loaded
And the buffer contains text "Hello"
When the user presses the 'a' key
Then the character 'a' is inserted into the buffer at the cursor position
```

**Driving port**: Keymap resolution pipeline (key event -> keymap resolve -> command execute -> buffer mutate)
**Observable outcome**: Buffer content changed via plugin-defined keybinding

### WS-5: User performs modal editing with Vim keybindings (M7)

**What it proves**: Full modal editing works as a Lisp plugin. The architecture is proven end-to-end.

```
Given the Vim keybinding plugin is loaded
And the buffer contains the text "Hello World"
And the editor is in Normal mode
When the user presses 'i' to enter Insert mode
And the user types "Brave "
And the user presses Escape to return to Normal mode
Then the buffer contains "Brave Hello World"
And the editor is in Normal mode
```

**Driving port**: Full pipeline (key event -> mode-aware keymap -> command -> buffer mutation -> mode transition)
**Observable outcome**: Text inserted only in Insert mode, mode transitions work, buffer reflects edits

---

## Implementation Sequence

Enable one walking skeleton at a time. Each must pass before enabling the next.

| Order | Skeleton | Milestone | First Test to Enable |
|-------|----------|-----------|---------------------|
| 1 | WS-1 | M1 | `given_a_file_when_opened_then_buffer_contains_content` |
| 2 | WS-2 | M2 | `given_buffer_with_text_when_lisp_inserts_then_buffer_updated` |
| 3 | WS-3 | M3 | `given_test_plugin_when_loaded_then_command_is_registered_and_callable` |
| 4 | WS-4 | M6 | `given_keybinding_plugin_when_key_pressed_then_character_inserted` |
| 5 | WS-5 | M7 | `given_vim_plugin_when_mode_switch_and_type_then_text_inserted` |

Note: M4 and M5 do not have walking skeletons because they are focused feature milestones (line numbers and status bar). They have focused scenarios that test specific rendering hook behavior.

---

## Walking Skeleton Litmus Test

Each skeleton above passes the four-point litmus test:

1. **Title describes user goal** -- "User opens a file and sees its content", not "Event loop reads file into rope"
2. **Given/When describe user actions** -- "When the user opens the file", not "When Buffer::from_file() is called"
3. **Then describe user observations** -- "buffer contains the file's full text", not "Rope node count equals expected"
4. **Non-technical stakeholder can confirm** -- A product person can say "Yes, that is what the editor should do"

---

## Test Level Strategy

Walking skeletons test at the API/state level, not at the terminal pixel level:

- **Do**: Assert on `EditorState` fields (buffer content, cursor position, mode, registered commands)
- **Do**: Construct `EditorState` programmatically, simulate key events as data
- **Do not**: Assert on terminal output, ANSI escape sequences, or rendered frames
- **Do not**: Require an actual terminal to run tests

This ensures tests run in CI without a TTY and remain stable across rendering changes.
