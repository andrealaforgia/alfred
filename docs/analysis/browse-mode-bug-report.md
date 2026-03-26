# Browse-Mode Exhaustive Bug Report

Root Cause Analysis of "alfred is completely bugged" -- browse-mode plugin

Investigator: Rex (RCA Specialist)
Date: 2026-03-25
Scope: `plugins/browse-mode/init.lisp` and its interactions with the Rust bridge, panel system, and vim-keybindings plugin.

---

## Critical Finding: `set-mode` vs `set-active-keymap` Redundancy and Conflict

Before detailing individual bugs, the systemic issue: the Rust `set-mode` function (bridge.rs:826-834) sets BOTH `editor.mode` AND `editor.active_keymaps`:

```rust
fn register_set_mode(...) {
    editor.mode = mode_name.clone();
    editor.active_keymaps = vec![format!("{}-mode", mode_name)];
}
```

Meanwhile, `set-active-keymap` (bridge.rs:489-495) ALSO sets `active_keymaps`:

```rust
fn register_set_active_keymap(...) {
    editor.active_keymaps = vec![keymap_name];
}
```

This means calling `(set-mode "X")` already sets keymap to `"X-mode"`. Calling `(set-active-keymap "X-mode")` after is redundant. But calling them with MISMATCHED values creates silent desync. This is the foundation of multiple bugs.

---

## BUG 1: `set-mode "browse"` creates keymap name `"browse-mode"` -- which works, but `set-mode "panel-filetree"` creates keymap name `"panel-filetree-mode"` which DOES NOT EXIST

**User symptom**: After pressing Ctrl-e to open the sidebar, all keys stop working (no j/k/Enter/Escape navigation in the sidebar). The sidebar appears but is completely unresponsive.

**Code path**:
1. User presses Ctrl-e in normal mode
2. `toggle-sidebar` command executes (line 335)
3. Line 362: `(set-mode "panel-filetree")` -- sets `editor.active_keymaps = ["panel-filetree-mode"]`
4. Line 363: `(set-active-keymap "filetree-mode")` -- sets `editor.active_keymaps = ["filetree-mode"]`

**Root cause**: Line 362 `(set-mode "panel-filetree")` makes the Rust bridge set `active_keymaps` to `["panel-filetree-mode"]`. This is immediately overwritten by line 363 `(set-active-keymap "filetree-mode")`. So the keymap works -- BUT the mode is `"panel-filetree"`, and there is no cursor shape defined for mode `"panel-filetree"`. This means the cursor shape lookup may fail or fall back to a default.

**Actual severity**: LOW -- the keymap override on line 363 corrects the keymap, so keys DO work. But the mode name `"panel-filetree"` is cosmetic noise and causes cursor shape lookup to miss.

**Fix**: Either:
- Use a mode name that matches: `(set-mode "filetree")` which would auto-set keymap to `"filetree-mode"` and remove the redundant `set-active-keymap` call, OR
- Define cursor shape for the panel mode: `(set-cursor-shape "panel-filetree" "block")`

---

## BUG 2: `toggle-sidebar` saves mode AFTER setting mode to `"panel-filetree"` -- `sidebar-saved-mode` is always `"panel-filetree"`

**User symptom**: After closing the sidebar (Ctrl-e again), the user is forced into "normal" mode regardless of what mode they were in before opening the sidebar. If they were in browse mode, they lose their browse-mode context and keymaps.

**Code path**:
1. User is in browse mode (mode = `"browse"`, keymap = `"browse-mode"`)
2. User presses Ctrl-e to open sidebar
3. Line 360: `(set sidebar-saved-mode (current-mode))` -- but wait...
4. Line 361: `(focus-panel "filetree")`
5. Line 362: `(set-mode "panel-filetree")`

Let me re-read the exact order.

```lisp
;; Lines 356-363 (the "open sidebar" branch):
(set-panel-size "filetree" sidebar-width)
(sidebar-load ...)
(set sidebar-saved-mode (current-mode))   ;; line 360
(focus-panel "filetree")                   ;; line 361
(set-mode "panel-filetree")               ;; line 362
(set-active-keymap "filetree-mode")       ;; line 363
```

**Root cause**: Line 360 calls `(current-mode)` BEFORE `(set-mode "panel-filetree")` on line 362. So `sidebar-saved-mode` correctly captures the current mode (e.g. `"normal"` or `"browse"`). This is actually CORRECT ordering.

However, `sidebar-saved-mode` is NEVER USED for restoration. Look at the close path:

```lisp
;; Lines 340-345 (the "close sidebar" branch):
(set sidebar-visible nil)
(set-panel-size "filetree" 0)
(unfocus-panel)
(set-mode "normal")              ;; HARDCODED "normal"!
(set-active-keymap "normal-mode") ;; HARDCODED "normal-mode"!
```

**The variable `sidebar-saved-mode` is set but never read.** The close path hardcodes `"normal"` mode. If the user was in browse mode before opening the sidebar, closing it puts them in normal mode with the wrong keymap.

**Severity**: HIGH

**Fix**: Replace lines 344-345 with:
```lisp
(set-mode sidebar-saved-mode)
```
This would restore the mode AND set the matching keymap (since `set-mode` auto-sets keymaps). The redundant `set-active-keymap` call on line 345 should be removed.

---

## BUG 3: `sidebar-enter` opens a file but does NOT close/hide the sidebar panel

**User symptom**: User navigates the sidebar, presses Enter on a file. The file opens but the sidebar panel remains visible at width 30, eating screen space. The sidebar shows stale content. There is no way to dismiss it except pressing Ctrl-e (which toggles it back on because `sidebar-visible` was not set to nil).

**Code path**:
1. User presses Enter on a file in sidebar
2. `sidebar-enter` executes (line 399)
3. Lines 407-412:
   ```lisp
   (unfocus-panel)
   (set-mode "normal")
   (set-active-keymap "normal-mode")
   (clear-line-styles)
   (open-file (path-join sidebar-current-dir (sidebar-current-name)))
   ```
4. `unfocus-panel` clears `focused_panel` in Rust
5. `open-file` loads the file and sets mode to `"normal"` (redundantly)
6. But `sidebar-visible` is still `1` (truthy)
7. `set-panel-size "filetree" 0` is never called

**Root cause**: `sidebar-enter` unfocuses and changes mode but does NOT set `sidebar-visible` to `nil` and does NOT call `(set-panel-size "filetree" 0)` to hide the panel.

**Consequence**: The sidebar panel remains visible with width 30. The file content is displayed in a reduced area. When the user presses Ctrl-e again, `toggle-sidebar` checks `sidebar-visible` (which is `1`/truthy) and executes the CLOSE branch -- so the first Ctrl-e closes it. But the user has to press Ctrl-e once to clean up the mess, which is confusing.

**Severity**: HIGH

**Fix**: In `sidebar-enter`, before or after `unfocus-panel`, add:
```lisp
(set sidebar-visible nil)
(set-panel-size "filetree" 0)
```

---

## BUG 4: `sidebar-enter` calls `(open-file ...)` AFTER `(set-mode "normal")` -- but `open-file` ALSO sets mode to `"normal"` and resets keymaps, creating redundancy and masking the sidebar-visible state bug

**User symptom**: No direct user-visible symptom beyond BUG 3, but this is a code quality issue that makes the interaction harder to reason about.

**Code path**: The Rust `open-file` function (bridge.rs:2442-2443) hardcodes:
```rust
editor.mode = "normal".to_string();
editor.active_keymaps = vec!["normal-mode".to_string()];
editor.focused_panel = None;
```

So `sidebar-enter` sets mode/keymap/unfocus, then `open-file` does it again.

**Severity**: LOW (redundancy, not a bug per se)

**Fix**: Either let `open-file` handle all state transitions (remove lines 408-410 from `sidebar-enter`), or make `open-file` not touch mode/keymaps and let the caller decide.

---

## BUG 5: `browser-cursor-down` allows moving beyond the last entry when `browser-entries` is empty

**User symptom**: If `browser-entries` is empty (empty directory), pressing `j` would evaluate `(- (length browser-entries) 1)` = `(- 0 1)` = `-1`. The comparison `(< browser-cursor -1)` with `browser-cursor = 0` is false, so nothing happens. This is SAFE but relies on signed integer comparison behavior.

However, `browser-jump-last` (line 163) does:
```lisp
(set browser-cursor (- (length browser-entries) 1))
```
If the directory is empty, this sets `browser-cursor` to `-1`.

**Code path**:
1. User navigates to empty directory
2. `browser-load-dir` sets `browser-entries` to result of `browser-add-parent-entry`
3. If at root and directory is empty: `browser-entries = (list)`
4. `browser-cursor = 0`
5. User presses `G` (jump-last): `browser-cursor` = -1
6. Next render: `browser-build-lines` with empty entries returns `browser-empty-str` (safe)
7. But `browser-cursor` is now -1, and any subsequent `nth -1 browser-entries` would crash

**Root cause**: `browser-jump-last` does not guard against empty list.

**Severity**: MEDIUM (crash on `G` in empty directory)

**Fix**: Guard `browser-jump-last`:
```lisp
(define-command "browser-jump-last"
  (lambda ()
    (if (> (length browser-entries) 0)
      (set browser-cursor (- (length browser-entries) 1))
      nil)
    (browser-render)))
```

---

## BUG 6: `browser-enter` crashes on empty directory

**User symptom**: If somehow `browser-entries` is empty and the user presses Enter, `(nth browser-cursor browser-entries)` with `browser-cursor = 0` on an empty list will produce a runtime error.

**Code path**:
1. Empty directory, `browser-entries = (list)`, `browser-cursor = 0`
2. User presses Enter
3. `browser-enter` (line 167): `(nth 1 (nth browser-cursor browser-entries))`
4. `(nth 0 (list))` -- runtime error, `nth` on empty list

**Root cause**: `browser-enter` does not check if `browser-entries` is empty.

**Severity**: MEDIUM (crash on Enter in empty directory)

**Fix**: Add guard at start of `browser-enter`:
```lisp
(define-command "browser-enter"
  (lambda ()
    (if (= (length browser-entries) 0)
      nil
      ;; ... existing body
```

---

## BUG 7: `browser-parent` calls `browser-do-go-parent` (different from `browser-do-parent`)

**User symptom**: When pressing `h` or Backspace in browse mode, the behavior differs from when navigating to `..` via Enter. The `h` key calls `browser-do-go-parent` which restores history, while Enter on `..` calls `browser-do-parent` which pushes to history. This is actually intentional design (h = back, Enter on .. = navigate). NOT a bug per se, but the existence of two near-identical parent-navigation paths is confusing.

**Severity**: NONE (design choice, not a bug)

---

## BUG 8: `sidebar-cursor-up` allows cursor to stay at `sidebar-header-offset` (line 2), but `panel-cursor-up` in Rust clamps at 0

**User symptom**: The Lisp guard on line 379 prevents cursor from going above `sidebar-header-offset` (2):
```lisp
(if (> (panel-cursor-line "filetree") sidebar-header-offset)
  (panel-cursor-up "filetree")
  nil)
```

This means the cursor stops at line 2 (first entry). The Rust `panel-cursor-up` would stop at 0 (header line), which would be wrong. The Lisp guard is correct.

**Severity**: NONE (correctly guarded)

---

## BUG 9: `sidebar-cursor-down` has NO upper bound check -- cursor can move past the last entry

**User symptom**: User presses `j` repeatedly in the sidebar. The Rust `panel-cursor-down` clamps at `max_line` (the highest key in the `lines` HashMap). But: after `sidebar-load`, lines are set for indices 0 through `(length sidebar-entries) + sidebar-header-offset - 1`. So the clamping works correctly based on the number of panel lines set.

However, consider: if the panel has lines 0, 1, 2, 3, 4 (header, separator, entry0, entry1, entry2) and there are 3 entries, max_line = 4. The cursor can reach line 4, which maps to entry index `4 - 2 = 2` (last entry). Pressing down again: Rust clamps at 4. This is correct.

**Severity**: NONE (correctly bounded by Rust side)

---

## BUG 10: `toggle-sidebar` from browse mode (Ctrl-e) -- the `"browse-mode"` keymap does NOT have Ctrl-e bound

**User symptom**: When the user runs `alfred .` (opens directory), they are in browse mode with keymap `"browse-mode"`. Pressing Ctrl-e does NOTHING because Ctrl-e is only bound in `"normal-mode"` (line 421):
```lisp
(define-key "normal-mode" "Ctrl:e" "toggle-sidebar")
```

The `"browse-mode"` keymap (lines 10-22) does not include Ctrl-e.

**Code path**:
1. `alfred .` -> mode = `"browse"`, keymap = `"browse-mode"`
2. User presses Ctrl-e
3. `resolve_key` looks up Ctrl-e in `"browse-mode"` keymap
4. Not found -> key is dropped silently

**Root cause**: Ctrl-e is not bound in `"browse-mode"`, only in `"normal-mode"`.

**Severity**: HIGH (user cannot access sidebar from browse mode at all)

**Fix**: Add to browse-mode keymap:
```lisp
(define-key "browse-mode" "Ctrl:e" "toggle-sidebar")
```

---

## BUG 11: `toggle-sidebar` from browse mode -- closing sidebar hardcodes return to "normal" mode instead of "browse" mode

**User symptom**: Even if BUG 10 is fixed (Ctrl-e added to browse-mode), opening and closing the sidebar from browse mode would leave the user in normal mode instead of returning to browse mode. This is a restatement of BUG 2 from the browse-mode perspective.

**Severity**: HIGH (duplicate of BUG 2, included for completeness of the browse-mode scenario)

---

## BUG 12: `sidebar-unfocus` (Escape/q in sidebar) hardcodes return to "normal" mode

**User symptom**: User is in browse mode, opens sidebar (if BUG 10 fixed), presses Escape. `sidebar-unfocus` (line 414) sets mode to "normal". User has lost their browse mode context. The buffer still shows the directory listing, but the keymap is now `"normal-mode"` so j/k/Enter do vim navigation instead of browser navigation.

**Code path**:
```lisp
(define-command "sidebar-unfocus"
  (lambda ()
    (unfocus-panel)
    (set-mode "normal")              ;; HARDCODED
    (set-active-keymap "normal-mode"))) ;; HARDCODED
```

**Root cause**: Same as BUG 2. `sidebar-saved-mode` is never used for restoration.

**Severity**: HIGH

**Fix**: Use `sidebar-saved-mode`:
```lisp
(define-command "sidebar-unfocus"
  (lambda ()
    (unfocus-panel)
    (set-mode sidebar-saved-mode)))
```

---

## BUG 13: After `open-file` from browse mode (Enter on file), pressing Ctrl-b to return to browser may show stale content

**User symptom**: User opens a file from the browser. Later presses Ctrl-b to return to browse mode. The `browse` command (line 215) calls `(browser-load-dir browser-current-dir)` which refreshes. This is actually correct -- it reloads the directory. NOT a bug.

**Severity**: NONE

---

## BUG 14: `open-file` from browser mode sets `editor.line_styles.clear()` but does NOT trigger hooks

**User symptom**: After opening a file from the browser, line numbers may not update immediately because the `buffer-changed` hook is not triggered by `open-file`. The gutter panel (line-numbers plugin) updates via hooks `cursor-moved` and `buffer-changed`.

**Code path**: The Rust `open-file` (bridge.rs:2434-2446) replaces the buffer, resets cursor, resets viewport, sets mode, clears line_styles, but does NOT fire any hooks.

**Root cause**: `open-file` is a programmatic buffer replacement that bypasses the hook system.

**Severity**: MEDIUM (line numbers may not display until the user moves the cursor, which triggers `cursor-moved`)

**Fix**: Either fire `buffer-changed` hook from within `open-file` in the Rust bridge, or have the Lisp caller fire it explicitly.

---

## BUG 15: `browser-do-go-parent` restores cursor from history but does NOT restore the directory from history

**User symptom**: When pressing `h` (browser-parent) to go up a directory, the behavior is subtly different from expected.

**Code path** for `browser-do-go-parent` (line 195):
```lisp
(define browser-do-go-parent
  (lambda ()
    ;; Restore cursor from history (if available)
    (if (> (length browser-history) 0)
      (set browser-cursor (nth 1 (first browser-history)))
      nil)
    ;; Pop history
    (if (> (length browser-history) 0)
      (set browser-history (rest browser-history))
      nil)
    ;; Navigate to parent
    (set browser-current-dir (path-parent browser-current-dir))
    (set browser-entries
      (browser-add-parent-entry browser-current-dir (list-dir browser-current-dir)))
    ;; Clamp cursor
    (if (> browser-cursor (- (length browser-entries) 1))
      (set browser-cursor (- (length browser-entries) 1))
      nil)
    (browser-render)))
```

The history entry stores `(dir cursor)` as `(list browser-current-dir browser-cursor)` (line 180). When going back via `h`, the function restores the cursor position from history but navigates to `(path-parent browser-current-dir)` instead of the directory stored in history. This is correct IF the user is doing simple up-navigation. But `browser-do-parent` (called from Enter on `..`) ALSO pushes to history (line 185-186), creating inconsistency:
- Enter on `..`: pushes history, then navigates to parent via `browser-load-dir`
- `h` key: pops history, navigates to parent via inline code

The discrepancy: if the user went `A -> B -> C`, then presses `..` (Enter) from C back to B (pushing C to history), then presses `h` from B (which pops history entry for B -- but wait, `browser-do-parent` pushed `(C, cursor)` not `(B, cursor)`).

Actually tracing more carefully:
- In dir B, press Enter on subdir C -> `browser-do-enter-dir` pushes `(B, cursor_in_B)` to history
- In dir C, press Enter on `..` -> `browser-do-parent` pushes `(C, cursor_in_C)` to history, then loads parent of C = B
- In dir B, press `h` -> `browser-do-go-parent` pops history, gets `(C, cursor_in_C)`, sets cursor to cursor_in_C, then navigates to parent of B = A

The cursor restored is from directory C, but we are navigating to directory A. The cursor position from C has no relation to directory A's listing.

**Root cause**: `browser-do-go-parent` restores cursor from the top of history stack, but the history entry may be for a completely different directory context. The history should be checked to see if the stored directory matches where we are navigating.

**Severity**: MEDIUM (cursor position may be wrong or out of bounds after navigating back via `h`, though the clamping on line 206-208 prevents crashes)

**Fix**: When `browser-do-go-parent` pops history, it should check if the stored directory matches `(path-parent browser-current-dir)`. If it does, use the stored cursor. If not, default to 0.

---

## BUG 16: `sidebar-apply-styles` computes cursor as `(- (panel-cursor-line "filetree") sidebar-header-offset)` every time -- may produce negative value

**User symptom**: If somehow `panel-cursor-line` returns a value less than `sidebar-header-offset` (2), the subtraction produces a negative number, which is then passed to the style comparison functions.

**Code path**:
1. `sidebar-load` calls `(panel-set-cursor "filetree" sidebar-header-offset)` (line 322) -- sets cursor to 2
2. `sidebar-load` calls `sidebar-apply-styles` (line 330)
3. `sidebar-apply-styles` (line 310): `(- (panel-cursor-line "filetree") sidebar-header-offset)` = `(- 2 2)` = 0 -- correct

But what about `clear-panel-lines`? Line 319: `(clear-panel-lines "filetree")` -- the Rust `clear_lines` resets `cursor_line` to 0.
Then line 322: `(panel-set-cursor "filetree" sidebar-header-offset)` -- sets it back to 2.
Then lines populate. Then line 330 calls `sidebar-apply-styles`.

The ordering within `sidebar-load` is safe. But what about `sidebar-refresh`? (line 367):
```lisp
(define sidebar-refresh
  (lambda ()
    (sidebar-populate-with-offset sidebar-entries 0
      (- (panel-cursor-line "filetree") sidebar-header-offset))
    (sidebar-apply-styles)))
```

This is called after `sidebar-cursor-down` and `sidebar-cursor-up`. The `sidebar-cursor-up` guard (line 379) ensures cursor stays >= `sidebar-header-offset`. So the value is always >= 0.

**Severity**: NONE (properly guarded)

---

## BUG 17: `set-mode "browse"` followed by `set-active-keymap "browse-mode"` is redundant

**User symptom**: No user-visible symptom. `set-mode "browse"` already sets `active_keymaps` to `["browse-mode"]`. The subsequent `set-active-keymap "browse-mode"` call is redundant.

**Code locations**: Lines 221-222, lines 436-437.

**Severity**: NONE (code smell, not a bug)

---

## BUG 18: Line numbers gutter panel has priority 100, filetree sidebar has priority 10 -- sidebar renders LEFT of gutter

**User symptom**: When the sidebar is open, it appears to the left of the line number gutter, which is visually correct for a file tree. The gutter (priority 100) renders to the right of the filetree (priority 10). This is actually CORRECT behavior.

**Severity**: NONE (working as designed)

---

## BUG 19: `sidebar-load` calls `clear-panel-lines` which resets cursor to 0, then immediately calls `panel-set-cursor` to set cursor to `sidebar-header-offset` -- but lines are not yet populated at that point

**User symptom**: No visible symptom because `panel-set-cursor` in Rust does NOT clamp to existing line count (it just sets the value directly -- see panel.rs line 268-270). The cursor is set to 2 before lines are populated. Then lines are populated starting from index 0. The Rust `panel_cursor_down` uses `max_line` from `lines.keys().max()`, but `panel-set-cursor` bypasses this check entirely.

**Root cause**: `set_panel_cursor` in Rust (panel.rs:263-271) does no bounds validation:
```rust
pub fn set_panel_cursor(...) -> Result<(), String> {
    find_panel_mut(registry, name).map(|panel| {
        panel.cursor_line = line;
    })
}
```

This is fine because lines are populated immediately after. But if `sidebar-entries` is empty, the cursor would be at line 2 with no entry there.

**Severity**: LOW (cosmetic -- cursor would point at empty space if directory is empty)

---

## BUG 20: `sidebar-enter` on a directory does NOT change mode/keymap -- stays in `"filetree-mode"` which is correct

Verifying: `sidebar-enter` for directories calls `sidebar-load` which does not touch mode/keymaps. The sidebar remains focused with `"filetree-mode"` active. Correct.

**Severity**: NONE

---

## Summary: Bugs by Severity

### CRITICAL / HIGH (must fix)

| # | Bug | User Impact |
|---|-----|-------------|
| 2 | `sidebar-saved-mode` is set but never used; close path hardcodes `"normal"` | Closing sidebar from any non-normal mode loses user's mode context |
| 3 | `sidebar-enter` on file does not hide sidebar panel | Sidebar stays visible with width 30 after opening a file, eating screen space |
| 10 | Ctrl-e not bound in `"browse-mode"` keymap | Sidebar is completely inaccessible from browse mode |
| 12 | `sidebar-unfocus` hardcodes `"normal"` mode | Pressing Escape in sidebar from browse mode loses browse context |

### MEDIUM (should fix)

| # | Bug | User Impact |
|---|-----|-------------|
| 5 | `browser-jump-last` on empty directory sets cursor to -1 | Crash or undefined behavior on `G` in empty directory |
| 6 | `browser-enter` on empty directory crashes | Runtime error on Enter in empty directory |
| 14 | `open-file` does not fire hooks | Line numbers may not render until cursor moves |
| 15 | `browser-do-go-parent` restores wrong cursor from history | Wrong cursor position after h-key navigation through deep hierarchies |

### LOW (nice to fix)

| # | Bug | User Impact |
|---|-----|-------------|
| 1 | Mode `"panel-filetree"` has no cursor shape defined | Cursor shape undefined when sidebar focused |
| 4 | `sidebar-enter` redundantly sets mode before `open-file` | Code clarity issue |
| 19 | `sidebar-load` sets cursor before populating lines | Cosmetic issue with empty directories |

---

## Scenario Traces

### Scenario 1: `alfred .` (open on directory)

1. **Mode and keymaps active**: mode=`"browse"`, keymaps=`["browse-mode"]` -- CORRECT
2. **Can navigate (j/k)**: YES, `"browse-mode"` has j/k bound -- CORRECT
3. **Can open file (Enter)**: YES, calls `open-file` which sets mode=`"normal"`, keymaps=`["normal-mode"]` -- CORRECT
4. **After opening file**: mode=`"normal"`, keymaps=`["normal-mode"]` -- CORRECT
5. **Press Ctrl-e**: YES, `"normal-mode"` has Ctrl-e bound to `toggle-sidebar` -- sidebar opens -- CORRECT
6. **Press Ctrl-b**: YES, `"normal-mode"` has Ctrl-b bound to `browse` command -- returns to browser -- CORRECT
7. **From browse mode, press Ctrl-e**: **BUG 10** -- Ctrl-e NOT bound in `"browse-mode"`. Silent failure. User cannot open sidebar from browse mode.
8. **From browse mode, press Ctrl-b**: NOT bound in `"browse-mode"`. `Ctrl:b` is only in `"normal-mode"`. **ADDITIONAL FINDING**: Ctrl-b does nothing in browse mode. But the user is already in browse mode, so this is expected -- Ctrl-b is for returning TO browse mode from normal mode.

### Scenario 2: `alfred file.rs` (open on file)

1. **Mode and keymaps**: vim-keybindings sets mode=`"normal"`, keymaps=`["normal-mode"]` at end of load. browse-mode activation (line 429-441): `browser-cli-arg` is `"file.rs"`, `is-dir?` returns false, so lines 439-441 set `browser-root-dir` and `browser-current-dir` to parent of file, and fall through without setting browse mode. Final state: mode=`"normal"`, keymaps=`["normal-mode"]` -- CORRECT
2. **Press Ctrl-e**: `toggle-sidebar` fires. `browser-root-dir` is set (parent dir). Sidebar opens, loads parent directory listing. -- CORRECT
3. **Navigate sidebar**: j/k work via `"filetree-mode"` -- CORRECT
4. **Open file from sidebar**: `sidebar-enter` -> `open-file`. File loads. **BUG 3**: sidebar panel remains visible.
5. **Syntax highlighting**: `open-file` clears `line_styles`. No syntax highlighting plugin exists, so N/A.
6. **Press Ctrl-e to close sidebar**: Since `sidebar-visible` is still `1` (BUG 3), this CLOSES the sidebar. Mode goes to `"normal"`. -- WORKS but only by accident.
7. **Press Ctrl-e to reopen**: `sidebar-visible` is now `nil`. Opens sidebar again. -- CORRECT

### Scenario 3: Edge Cases

1. **Empty `sidebar-entries`**: `sidebar-enter` guard (line 401) checks `(= (length sidebar-entries) 0)` and returns nil. SAFE.
2. **Empty `sidebar-current-dir`**: If `browser-empty-str`, `toggle-sidebar` checks `browser-root-dir` first (line 337). If it is empty, shows message. SAFE.
3. **`panel-cursor-line` >= length of `sidebar-entries`**: `sidebar-entry-index` returns `cursor - 2`. If cursor is at 2 and entries has 1 item, index is 0. If cursor somehow exceeds, `nth` would crash. But `panel-cursor-down` in Rust clamps at max populated line. SAFE.
4. **`sidebar-cursor-up` at position 0 (after header offset)**: The guard `(> (panel-cursor-line "filetree") sidebar-header-offset)` prevents movement. SAFE.
5. **`open-file` fails**: Rust bridge sets error message and returns NIL. No crash. SAFE, but user gets no visual feedback beyond the message bar.

---

## Cross-Validation

All HIGH bugs are independently verifiable:
- BUG 2: If `sidebar-saved-mode` is never read -> closing sidebar always sets "normal" -- CONFIRMED by reading all code paths that reference `sidebar-saved-mode` (only set, never read).
- BUG 3: If `sidebar-enter` does not call `set-panel-size 0` -> panel remains visible -- CONFIRMED by reading the complete `sidebar-enter` function.
- BUG 10: If `"browse-mode"` keymap does not contain `Ctrl:e` -> key is silently dropped -- CONFIRMED by listing all `define-key "browse-mode"` calls (lines 12-22, none include Ctrl:e).
- BUG 12: If `sidebar-unfocus` hardcodes "normal" -> same issue as BUG 2 -- CONFIRMED by reading function (lines 414-418).

Root causes collectively explain the "completely bugged" experience:
- From browse mode, the sidebar is inaccessible (BUG 10)
- Opening files from the sidebar leaves the sidebar visible (BUG 3)
- Closing the sidebar from any non-normal mode loses context (BUGs 2, 12)
- Empty directories can crash the browser (BUGs 5, 6)
