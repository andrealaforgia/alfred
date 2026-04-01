# Walking Skeleton: Overlay File Search

## User Goal

A developer working in Alfred wants to quickly find and open a file by name
from anywhere in the project, without manually navigating the directory tree.

## The Scenario

```
Given the user has opened Alfred on a project directory
When the user searches for a file by name using the overlay
Then the selected file is opened in the editor
And the user can edit and save the file
```

The "When" step encompasses the full search interaction: opening the overlay
with Ctrl-p, typing a filename fragment, and pressing Enter to select. This is
a single user action ("search for a file") expressed at the business level.

## Minimal Vertical Slice

To make this single scenario pass, every layer must be wired end-to-end. This
is the thinnest possible slice that delivers observable user value.

### What Must Exist

1. **Overlay data model** (`alfred-core/src/overlay.rs`)
   - Struct with: visible flag, input text, items list, cursor index
   - Pure functions: create/reset, set input, set items, get selected item
   - EditorState gains an overlay field

2. **Overlay rendering** (`alfred-tui/src/renderer.rs`)
   - When overlay is visible, draw a centered box over existing content
   - Render the input line ("> " + query text)
   - Render the items list with cursor highlight on selected row
   - Place terminal cursor at end of overlay input

3. **Input routing** (`alfred-tui/src/app.rs` or `input.rs`)
   - When overlay is visible, route key events to the overlay keymap
   - Existing keymap dispatch system handles this via `set-active-keymap`

4. **Overlay bridge primitives** (`alfred-lisp/src/bridge.rs`)
   - `open-overlay` -- set visible, clear state, set dimensions
   - `close-overlay` -- set invisible
   - `overlay-set-input` -- set input field text
   - `overlay-set-items` -- set results list
   - `overlay-get-selected` -- return highlighted item

5. **Overlay search plugin** (`plugins/overlay-search/init.lisp`)
   - On `"overlay-file-search"` command: call `list-dir-recursive`, cache
     results, call `open-overlay`, populate items, switch keymap
   - On character input: append to query, filter cached list, update overlay
   - On Enter: get selected, call `open-file`, call `close-overlay`, restore
     mode
   - On Escape: call `close-overlay`, restore mode

6. **Browse-mode binding** (`plugins/browse-mode/init.lisp`)
   - Register `Ctrl:p` in browser-panel-mode keymap to trigger
     `"overlay-file-search"`

### What Can Be Deferred

- `overlay-cursor-down` / `overlay-cursor-up` (arrow navigation) -- the first
  result is auto-selected, so Enter works without navigation
- `overlay-set-title` -- the prompt can be hardcoded initially
- `overlay-visible?` -- not needed for the basic flow
- Case-insensitive filtering -- exact substring match is sufficient for WS
- Browser panel navigation to parent directory after selection
- `Ctrl:p` binding in normal mode (testing from browser mode is sufficient)
- Scroll offset for long result lists
- Cursor reset when query changes (M3-6)
- Input leak prevention on dismiss (M1-7 -- overlay keymap captures chars)
- Rapid type-delete-retype robustness (M2-8)

### Implementation Order

Build bottom-up to enable the top-level scenario:

```
1. overlay.rs        -- data model + pure functions
2. editor_state.rs   -- add overlay field
3. bridge.rs         -- register 5 primitives (open, close, set-input, set-items, get-selected)
4. renderer.rs       -- overlay rendering when visible
5. app.rs/input.rs   -- route keys to overlay keymap when visible
6. init.lisp         -- overlay-search plugin (command, char input, Enter, Escape)
7. browse-mode       -- add Ctrl:p binding
```

### How to Verify

The E2E test proves it works:

```python
def test_find_and_open_file_via_overlay_search(self):
    child = spawn_alfred(ALFRED_PROJECT)
    child.expect("crates/", timeout=5)

    # Open overlay
    child.send("\x10")  # Ctrl-p
    time.sleep(0.5)

    # Type filename fragment
    for ch in "main.rs":
        send_keys(child, ch)
        time.sleep(0.15)
    time.sleep(0.5)

    # Select
    child.send("\r")  # Enter
    time.sleep(1.0)

    # Prove file opened by editing and saving
    send_keys(child, "A")
    time.sleep(0.2)
    send_keys(child, " // overlay-found")
    time.sleep(0.2)
    child.send("\x1b")  # Escape
    time.sleep(0.3)
    send_colon_command(child, "wq")
    wait_for_exit(child)

    target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
    saved = read_file(target)
    assert "// overlay-found" in saved
```

When this test passes, the feature is demo-able: a stakeholder can watch
Ctrl-p open a search dialog, type a name, press Enter, and see the file
open in the editor.

## Definition of Done for Walking Skeleton

- [ ] Overlay data model exists with pure functions
- [ ] Overlay renders centered over editor content
- [ ] Input routing sends keys to overlay keymap when visible
- [ ] Bridge primitives expose overlay operations to Lisp
- [ ] Overlay search plugin handles: open, type, filter, select, dismiss
- [ ] Ctrl-p binding triggers overlay from browser mode
- [ ] Walking skeleton E2E test passes
- [ ] `make ci-local` and `make e2e` pass
