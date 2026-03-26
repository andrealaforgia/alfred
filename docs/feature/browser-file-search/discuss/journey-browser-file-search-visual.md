# Journey: Browser File Search -- Visual

## Journey Flow

```
[Browse Dir]  --/-->  [Search Mode]  --type-->  [Filter Live]  --Enter-->  [Open File]
  Feels:               Feels:                    Feels:                     Feels:
  Oriented but         Focused,                  Engaged,                   Satisfied,
  overwhelmed by       expectant                 narrowing in               found it
  long list                                      on target

                       --Esc-->  [Full Listing Restored]
                                   Feels: Safe, back to normal
```

## Emotional Arc

```
Confidence
    ^
    |                                              * Open file
    |                                           *    (confident)
    |                                        *
    |                                     * See 2-3 matches
    |                                  *    (engaged)
    |                               *
    |            * Search active  *
    |         *    (focused)
    |      *
    |   * Press /
    |  (expectant)
    |
    | * Browsing long list
    |   (mildly frustrated)
    +---------------------------------------------------> Time
```

**Pattern**: Confidence Building
- Start: Mildly frustrated (too many entries to scroll through)
- Middle: Focused and engaged (typing narrows the list, each keystroke shows progress)
- End: Confident and satisfied (target file found and opened)

**Alternative arc (Escape)**:
- Middle divergence: No matches found (briefly uncertain)
- Recovery: Press Escape, full list restored (relieved, safe to try again)

---

## Step 1: Browsing a Large Directory (Pre-Search)

### Full-Screen Browser

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|                                                                       |
|    app.rs                                                             |
|    auth.rs                                                            |
|  > bridge.rs                                                          |
|    bridge_helpers.rs                                                  |
|    cache.rs                                                           |
|    config.rs                                                          |
|    database.rs                                                        |
|    error.rs                                                           |
|    handler.rs                                                         |
|    input.rs                                                           |
|    lib.rs                                                             |
|    logger.rs                                                          |
|    main.rs                                                            |
|    middleware.rs                                                      |
|    models/                                                            |
|    routes/                                                            |
|    runtime.rs                                                         |
|    server.rs                                                          |
|    state.rs                                                           |
|    tests/                                                             |
|    utils.rs                                                           |
|    validator.rs                                                       |
+-----------------------------------------------------------------------+
```

Kai sees 23 entries. He wants `runtime.rs`. Currently at line 5 (`bridge.rs`). He would need to press `j` 13 times to reach it.

### Sidebar

```
+----- filetree ------+
| /home/kai/.../src    |
|                      |
|    app.rs            |
|    auth.rs           |
|  > bridge.rs         |
|    bridge_helpers.rs |
|    cache.rs          |
|    config.rs         |
|    database.rs       |
|    error.rs          |
|    handler.rs        |
|    input.rs          |
|    lib.rs            |
|    logger.rs         |
|    main.rs           |
|    middleware.rs      |
|    models/           |
|    routes/           |
|    runtime.rs        |
|    server.rs         |
|    state.rs          |
|    tests/            |
|    utils.rs          |
|    validator.rs      |
+----------------------+
```

---

## Step 2: Enter Search Mode (Press `/`)

### Full-Screen Browser

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|  /                                                                    |
|    app.rs                                                             |
|    auth.rs                                                            |
|  > bridge.rs                                                          |
|    bridge_helpers.rs                                                  |
|    cache.rs                                                           |
|    config.rs                                                          |
|    database.rs                                                        |
|    ...                                                                |
+-----------------------------------------------------------------------+
```

- Line 2 (formerly blank separator) now shows the search prompt: `/`
- Cursor blinks after the `/` indicating text input is active
- The full file listing remains visible -- nothing filtered yet
- The prompt is styled distinctly (e.g., yellow/amber) to signal mode change

### Sidebar

```
+----- filetree ------+
| /home/kai/.../src    |
| /                    |
|    app.rs            |
|    auth.rs           |
|  > bridge.rs         |
|    bridge_helpers.rs |
|    ...               |
+----------------------+
```

---

## Step 3: Type Query -- Incremental Filtering

### After typing "run" in full-screen browser

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|  / run                                                                |
|  > runtime.rs                                                         |
+-----------------------------------------------------------------------+
```

- The search prompt shows `/ run`
- Only entries whose names contain "run" (case-insensitive) are displayed
- Cursor auto-selects the first match
- Directory path header (line 1) is unchanged
- Entry count reduced from 23 to 1

### After typing "br" in full-screen browser

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|  / br                                                                 |
|  > bridge.rs                                                          |
|    bridge_helpers.rs                                                  |
+-----------------------------------------------------------------------+
```

- Two matches: both files containing "br"
- Cursor on first match; j/k still works to navigate filtered results

### After typing "br" in sidebar

```
+----- filetree ------+
| /home/kai/.../src    |
| / br                 |
|  > bridge.rs         |
|    bridge_helpers.rs |
+----------------------+
```

---

## Step 4: No Matches

### Typing "xyz" in full-screen browser

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|  / xyz                                                                |
|    (no matches)                                                       |
+-----------------------------------------------------------------------+
```

- Clear message: "(no matches)" rather than blank screen
- User can: press Backspace to edit query, or Escape to dismiss search entirely
- The message is styled in dim/gray to indicate absence, not error

---

## Step 5: Open File from Filtered List (Enter)

User presses Enter on `runtime.rs`:
- Search mode is dismissed
- `runtime.rs` opens in the editor buffer
- Mode transitions to "normal" (editing mode)
- Status bar shows the filename

This is identical to the existing browser-enter behavior -- search simply changes which entries are visible.

---

## Step 6: Dismiss Search (Escape)

User presses Escape during search:

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|                                                                       |
|    app.rs                                                             |
|    auth.rs                                                            |
|  > bridge.rs                                                          |
|    bridge_helpers.rs                                                  |
|    ...                                                                |
+-----------------------------------------------------------------------+
```

- Search prompt disappears
- Full directory listing restored
- Cursor returns to the position it was at before search began
- User is back in normal browse/filetree navigation mode

---

## Step 7: Enter Directory from Filtered List

User types "mod" and sees:

```
+-----------------------------------------------------------------------+
|  /home/kai/projects/webapi/src                                        |
|  / mod                                                                |
|  > models/                                                            |
+-----------------------------------------------------------------------+
```

User presses Enter:
- Search mode is dismissed
- Browser navigates into the `models/` directory
- Full listing of `models/` is shown (no search filter carried over)
- This matches existing directory-enter behavior

---

## Key Interaction Summary

| Key | Context | Action |
|-----|---------|--------|
| `/` | Browse mode / Filetree mode (not in search) | Enter search mode; show search prompt |
| Any printable char | Search mode active | Append to query; re-filter entries |
| Backspace | Search mode, query non-empty | Delete last char; re-filter entries |
| Backspace | Search mode, query empty | Dismiss search; restore full listing |
| Escape | Search mode | Dismiss search; restore full listing; restore cursor |
| j / Down | Search mode | Move cursor down in filtered list |
| k / Up | Search mode | Move cursor up in filtered list |
| Enter | Search mode, on a file | Open the file; dismiss search |
| Enter | Search mode, on a directory | Enter the directory; dismiss search |
| g | Search mode | Jump to first entry in filtered list |
| G | Search mode | Jump to last entry in filtered list |
