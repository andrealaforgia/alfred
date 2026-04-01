# Overlay Search -- Acceptance Test Scenarios

Test scenarios for the floating overlay search dialog feature. Each scenario is
described in Given-When-Then format, followed by a pytest test skeleton matching
the patterns in `tests/e2e/test_alfred.py`.

All tests spawn Alfred via `spawn_alfred()`, send keystrokes via `send_keys()`,
and verify outcomes through file content (`read_file()`) or clean exit codes.
Ctrl-p is `\x10`, Escape is `\x1b`, Enter is `\r`.

Tests use the real Alfred project at `/alfred` inside the Docker container as
sample data, consistent with existing browser/search tests.

---

## Walking Skeleton

The simplest end-to-end path proving a user can accomplish their goal: find and
open a file using the overlay search.

### WS-1: User finds and opens a file via overlay search

```
Given the user has opened Alfred on a project directory
When the user searches for a file by name using the overlay
Then the selected file is opened in the editor
And the user can edit and save the file
```

```python
class TestOverlaySearchWalkingSkeleton:
    """Overlay search: find a file by name and open it."""

    def test_find_and_open_file_via_overlay_search(self):
        """Ctrl-p, type filename fragment, Enter opens file, edit+save proves it."""
        # Given: Alfred opened on the project directory
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)

        # When: search for a file by name using the overlay
        # child.send("\x10")  # Ctrl-p opens overlay
        # time.sleep(0.5)
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")  # Enter selects the result
        # time.sleep(1.0)

        # Then: file is opened -- prove by editing and saving
        # send_keys(child, "A")  # append mode
        # send_keys(child, " // overlay-found")
        # send_escape(child)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)

        # Verify: file on disk contains the edit marker
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "// overlay-found" in saved
        pass
```

---

## Milestone 1: Overlay Lifecycle

Tests verifying the overlay opens, renders, and dismisses correctly.

### M1-1: Ctrl-p opens the search overlay from browser mode

```
Given the user has opened Alfred on a project directory
And the browser panel is visible
When the user presses Ctrl-p
Then the search overlay appears with a prompt for typing
```

```python
class TestOverlayLifecycle:
    """Verify the overlay opens, renders, and dismisses."""

    def test_ctrl_p_opens_overlay_from_browser(self):
        """Ctrl-p from browser mode opens the floating search overlay."""
        # Given: Alfred opened on project directory, browser visible
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)

        # When: press Ctrl-p
        # child.send("\x10")
        # time.sleep(0.5)

        # Then: overlay appears -- verify by reading PTY output for the prompt
        # screen = child.read_nonblocking(size=16384, timeout=2)
        # assert ">" in screen  # overlay prompt character

        # Cleanup: Escape, quit
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_keys(child, "q")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M1-2: Escape closes the overlay without opening a file

```
Given the user has opened the search overlay with Ctrl-p
When the user presses Escape
Then the overlay disappears
And the editor returns to its previous state
```

```python
    def test_escape_closes_overlay_without_action(self):
        """Escape dismisses overlay; no file opened, editor state unchanged."""
        # Given: Alfred on project, open overlay
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: press Escape
        # child.send("\x1b")
        # time.sleep(0.5)

        # Then: overlay gone, back in browse mode -- verify by using
        # browse navigation (j then q to unfocus) and quitting cleanly
        # send_keys(child, "j")
        # time.sleep(0.2)
        # send_keys(child, "q")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M1-3: Ctrl-p works from normal mode (editor focused)

```
Given the user has a file open in the editor in normal mode
When the user presses Ctrl-p
Then the search overlay appears
```

```python
    def test_ctrl_p_opens_overlay_from_normal_mode(self):
        """Ctrl-p from normal mode (editing a file) opens overlay."""
        # Given: open a file in editor, in normal mode
        # spawn_alfred(ALFRED_BIN_CRATE)
        # child.expect("Cargo.toml", timeout=5)
        # send_keys(child, "j")
        # time.sleep(0.2)
        # child.send("\r")
        # time.sleep(1.0)
        # now in normal mode editing a file

        # When: press Ctrl-p
        # child.send("\x10")
        # time.sleep(0.5)

        # Then: overlay appears -- verify prompt visible
        # screen = child.read_nonblocking(size=16384, timeout=2)
        # assert ">" in screen

        # Cleanup: Escape, quit
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M1-4: Ctrl-p works from the browser panel

```
Given the user is focused on the browser panel
When the user presses Ctrl-p
Then the search overlay appears
And the browser panel remains visible underneath
```

```python
    def test_ctrl_p_opens_overlay_from_browser_panel(self):
        """Ctrl-p while browser panel is focused opens overlay."""
        # Given: Alfred opened on project, browser panel focused
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)

        # When: press Ctrl-p (browser panel is focused by default)
        # child.send("\x10")
        # time.sleep(0.5)

        # Then: overlay appears
        # screen = child.read_nonblocking(size=16384, timeout=2)
        # assert ">" in screen

        # Verify: after Escape, browser is still there
        # child.send("\x1b")
        # time.sleep(0.5)
        # try:
        #     child.expect("crates/", timeout=3)
        # except pexpect.TIMEOUT:
        #     pytest.fail("Browser panel not visible after overlay dismiss")

        # Cleanup
        # send_keys(child, "q")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M1-5: Opening overlay twice in a row works correctly (error path)

```
Given the user has opened and dismissed the overlay with Escape
When the user presses Ctrl-p again
Then the overlay appears fresh with empty query
And previous search state is cleared
```

```python
    def test_overlay_opens_fresh_after_previous_dismiss(self):
        """Re-opening overlay after Escape shows clean state, no stale results."""
        # Given: open overlay, type something, dismiss
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # send_keys(child, "xyz")
        # time.sleep(0.3)
        # child.send("\x1b")
        # time.sleep(0.5)

        # When: open overlay again
        # child.send("\x10")
        # time.sleep(0.5)

        # Then: overlay shows clean prompt (no leftover "xyz")
        # Verify by searching for a valid file -- if state was stale,
        # the "xyz" prefix would prevent matches
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

### M1-6: Ctrl-p pressed while overlay is open does not crash (error path)

```
Given the user has opened the search overlay with Ctrl-p
When the user presses Ctrl-p again while the overlay is visible
Then the editor does not crash
And the overlay remains functional
```

```python
    def test_ctrl_p_while_overlay_open_no_crash(self):
        """Pressing Ctrl-p again while overlay is open: no crash."""
        # Given: overlay open
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: press Ctrl-p again
        # child.send("\x10")
        # time.sleep(0.5)

        # Then: no crash -- Escape and quit cleanly
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_keys(child, "q")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M1-7: Overlay dismissed after typing partial query does not leak input (error path)

```
Given the user has opened the search overlay and typed a partial query
When the user presses Escape to dismiss
Then the typed characters do not appear in the editor buffer
And no file is modified
```

```python
    def test_overlay_dismiss_does_not_leak_typed_chars(self):
        """Characters typed in overlay do not leak into editor buffer on dismiss."""
        # Given: open a file first, then open overlay and type
        # path = create_temp_file("clean content")
        # child = spawn_alfred(path)
        # time.sleep(0.5)
        # child.send("\x10")  # Ctrl-p to open overlay
        # time.sleep(0.5)
        # send_keys(child, "xyz123")
        # time.sleep(0.3)

        # When: dismiss overlay
        # child.send("\x1b")
        # time.sleep(0.5)

        # Then: save the file and verify no "xyz123" leaked into buffer
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # saved = read_file(path)
        # assert "xyz123" not in saved
        # assert "clean content" in saved
        # os.unlink(path)
        pass
```

---

## Milestone 2: Search and Filtering

Tests verifying the search input filters the file list correctly.

### M2-1: Typing filters results to matching files

```
Given the user has opened the search overlay
When the user types "main.rs"
Then only files matching "main.rs" are shown in the results
```

```python
class TestOverlaySearch:
    """Verify search input filters results in the overlay."""

    def test_typing_filters_results(self):
        """Typing a fragment in overlay shows only matching files."""
        # Given: Alfred on project, open overlay
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: type "main.rs"
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)

        # Then: results show main.rs matches -- verify by selecting and
        # opening the file, then confirming content contains "fn main()"
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

### M2-2: Backspace removes last character and updates results

```
Given the user has typed "mainx" in the overlay search
When the user presses Backspace
Then the query becomes "main"
And the results update to show files matching "main"
```

```python
    def test_backspace_updates_filter(self):
        """Backspace removes last char; results update accordingly."""
        # Given: overlay open, type "mainx" (no matches expected)
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "mainx":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.3)

        # When: press Backspace to remove 'x'
        # child.send("\x7f")  # Backspace
        # time.sleep(0.5)

        # Then: query is "main", results show main.rs matches
        # Verify by selecting and opening -- should get a valid file
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

### M2-3: Empty query shows all files

```
Given the user has opened the search overlay
When the user has not typed anything
Then all project files are listed in the results
```

```python
    def test_empty_query_shows_all_files(self):
        """Opening overlay without typing shows all project files."""
        # Given: overlay open with no query
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: no typing -- immediately press Enter to select first item

        # Then: a file opens (any file from the project)
        # child.send("\r")
        # time.sleep(1.0)

        # Verify we're in editor mode by quitting cleanly
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M2-4: Search is case-insensitive

```
Given the user has opened the search overlay
When the user types "MAIN" in uppercase
Then files with "main" in their path are shown
```

```python
    def test_search_is_case_insensitive(self):
        """Typing uppercase matches lowercase filenames."""
        # Given: overlay open
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: type "MAIN" in uppercase
        # for ch in "MAIN":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)

        # Then: results include main.rs -- verify by selecting and opening
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

### M2-5: Search with no matches shows empty results (error path)

```
Given the user has opened the search overlay
When the user types a query that matches no files
Then the results list is empty
And pressing Enter does nothing
And the overlay remains open
```

```python
    def test_no_matches_enter_does_nothing(self):
        """Query with no matches: Enter does not open a file or crash."""
        # Given: overlay open
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: type nonsense query
        # for ch in "zzz_nonexistent_zzz":
        #     send_keys(child, ch)
        #     time.sleep(0.1)
        # time.sleep(0.3)

        # And: press Enter on empty results
        # child.send("\r")
        # time.sleep(0.5)

        # Then: overlay stays open (or dismisses safely) -- verify no crash
        # by pressing Escape and quitting cleanly
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_keys(child, "q")
        # time.sleep(0.3)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M2-6: Search skips target/ directory files (edge case)

```
Given the project contains files inside target/ build directory
When the user searches in the overlay
Then files inside target/ are not shown in results
```

```python
    def test_overlay_search_skips_target_directory(self):
        """Overlay search does not list files from target/ directory."""
        # Given: plant marker files in visible and hidden locations
        # visible = os.path.join(ALFRED_PROJECT, "crates", "e2e_overlay_marker.rs")
        # hidden = os.path.join(ALFRED_PROJECT, "target", "e2e_overlay_marker.rs")
        # write marker content to both files

        # When: open overlay, search for marker name
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "e2e_overlay_marker":
        #     send_keys(child, ch)
        #     time.sleep(0.1)
        # time.sleep(0.5)

        # Then: selecting the result opens the visible file, not the hidden one
        # child.send("\r")
        # time.sleep(1.0)
        # send_keys(child, "A")
        # time.sleep(0.2)
        # send_keys(child, " // overlay-verified")
        # time.sleep(0.2)
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)

        # visible file should be edited, hidden file untouched
        # assert "// overlay-verified" in read_file(visible)
        # assert "// overlay-verified" not in read_file(hidden)

        # Cleanup marker files
        # os.remove(visible)
        # os.remove(hidden)
        pass
```

### M2-7: Backspace on empty query does not crash (error path)

```
Given the user has opened the search overlay
And the search query is empty
When the user presses Backspace
Then the overlay remains open with empty query
And the editor does not crash
```

```python
    def test_backspace_on_empty_query_no_crash(self):
        """Backspace with empty query: overlay stays open, no crash."""
        # Given: overlay open, no typing
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: press Backspace on empty query
        # child.send("\x7f")  # Backspace
        # time.sleep(0.3)
        # child.send("\x7f")  # Backspace again for good measure
        # time.sleep(0.3)

        # Then: overlay still works -- type a query and select
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Verify: file opened correctly
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M2-8: Rapidly typing and deleting does not cause stale results (error path)

```
Given the user has opened the search overlay
When the user types a query, backspaces all of it, and types a new query rapidly
Then the results match the final query, not any intermediate state
```

```python
    def test_rapid_type_delete_retype_shows_correct_results(self):
        """Rapid type-delete-retype cycle shows results for the final query."""
        # Given: overlay open
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: type "xyz", backspace three times, type "main.rs"
        # for ch in "xyz":
        #     send_keys(child, ch)
        #     time.sleep(0.05)
        # for _ in range(3):
        #     child.send("\x7f")
        #     time.sleep(0.05)
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.1)
        # time.sleep(0.5)

        # Then: results match "main.rs" -- verify by selecting
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

---

## Milestone 3: Navigation and Selection

Tests verifying cursor movement and file opening from the results list.

### M3-1: Arrow down moves highlight to next result

```
Given the user has opened the search overlay with results listed
When the user presses the down arrow key
Then the highlight moves to the next result
```

```python
class TestOverlayNavigation:
    """Verify cursor navigation and file selection in overlay results."""

    def test_arrow_down_selects_next_result(self):
        """Down arrow moves highlight; Enter opens the newly highlighted file."""
        # Given: overlay open with results (empty query = all files)
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: press down arrow to move to second result
        # child.send("\x1b[B")  # Arrow Down
        # time.sleep(0.3)

        # Then: Enter opens the second file (not the first)
        # child.send("\r")
        # time.sleep(1.0)

        # Verify a file opened by quitting cleanly
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M3-2: Arrow up moves highlight to previous result

```
Given the user has moved the highlight down in the overlay
When the user presses the up arrow key
Then the highlight moves back to the previous result
```

```python
    def test_arrow_up_selects_previous_result(self):
        """Down then up returns to first item; Enter opens it."""
        # Given: overlay open, move down
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # child.send("\x1b[B")  # Down
        # time.sleep(0.3)

        # When: press up arrow
        # child.send("\x1b[A")  # Up
        # time.sleep(0.3)

        # Then: back at first result -- Enter opens first file
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M3-3: Enter opens the highlighted file and its content is correct

```
Given the user has the search overlay open with "main.rs" typed
When the user presses Enter
Then the file main.rs is opened in the editor
And the file content contains the main function
```

```python
    def test_enter_opens_highlighted_file_with_correct_content(self):
        """Enter opens highlighted file; content matches expected file."""
        # Given: overlay open, type "main.rs"
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)

        # When: press Enter
        # child.send("\r")
        # time.sleep(1.0)

        # Then: verify file content by editing and saving
        # send_keys(child, "A")
        # time.sleep(0.2)
        # send_keys(child, " // nav-verified")
        # time.sleep(0.2)
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        # assert "// nav-verified" in saved
        pass
```

### M3-4: Arrow up at top of list does not wrap or crash (boundary)

```
Given the user has the overlay open with highlight on the first item
When the user presses the up arrow key
Then the highlight stays on the first item
And the overlay does not crash
```

```python
    def test_arrow_up_at_top_stays_at_first(self):
        """Up arrow at first item: highlight stays, no crash."""
        # Given: overlay open, highlight on first item
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)

        # When: press up arrow (already at top)
        # child.send("\x1b[A")  # Up
        # time.sleep(0.3)

        # Then: no crash, Enter still opens first item
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M3-5: Arrow down at bottom of filtered list does not wrap or crash (boundary)

```
Given the user has filtered to a single result
When the user presses the down arrow key
Then the highlight stays on the only result
And the overlay does not crash
```

```python
    def test_arrow_down_at_bottom_stays_at_last(self):
        """Down arrow past last item: highlight stays, no crash."""
        # Given: overlay open, filter to unique file
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "Cargo.lock":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)

        # When: press down arrow multiple times (only one result)
        # child.send("\x1b[B")  # Down
        # time.sleep(0.2)
        # child.send("\x1b[B")  # Down again
        # time.sleep(0.2)

        # Then: no crash, Enter opens the single result
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # saved = read_file(os.path.join(ALFRED_PROJECT, "Cargo.lock"))
        # assert len(saved) > 0  # file has content
        pass
```

### M3-6: Navigation resets when search query changes (boundary)

```
Given the user has moved the highlight down to the third result
When the user types an additional character to narrow the results
Then the highlight resets to the first result in the new list
```

```python
    def test_navigation_resets_on_query_change(self):
        """Typing after navigating resets cursor to first result."""
        # Given: overlay open, navigate down twice
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # child.send("\x1b[B")  # Down
        # time.sleep(0.2)
        # child.send("\x1b[B")  # Down again
        # time.sleep(0.2)

        # When: type a character to narrow results
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)

        # Then: first result is highlighted -- selecting opens main.rs
        # child.send("\r")
        # time.sleep(1.0)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "fn main()" in saved
        pass
```

---

## Milestone 4: Integration with Browser

Tests verifying the overlay interacts correctly with the browser panel
and editor state after file selection.

### M4-1: After selecting a file, browser reflects the file's parent directory

```
Given the user opens the overlay and searches for a deeply nested file
When the user selects the file
Then the browser panel navigates to the file's parent directory
```

```python
class TestOverlayBrowserIntegration:
    """Verify overlay search integrates with browser panel and editor."""

    def test_browser_navigates_to_parent_after_selection(self):
        """After overlay selection, browser shows the file's parent directory."""
        # Given: Alfred on project root, open overlay
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)

        # When: search for deeply nested file and select it
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Then: browser should show the parent dir (src/) contents
        # Toggle focus to browser with Ctrl-e twice or check screen
        # The browser panel should reflect crates/alfred-bin/src/
        # containing main.rs and ../

        # Verify by quitting cleanly (proves no crash in nav update)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M4-2: Editor shows the selected file content

```
Given the user has selected a file via overlay search
When the file opens in the editor
Then the editor buffer displays the selected file's content
And the user can edit the file normally
```

```python
    def test_editor_shows_selected_file_content(self):
        """Selected file content appears in editor; edits save correctly."""
        # Given: open overlay, search for Cargo.toml, select it
        # spawn_alfred(ALFRED_BIN_CRATE)
        # child.expect("Cargo.toml", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "Cargo.toml":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Then: file content is in editor -- edit and save
        # send_keys(child, "A")
        # time.sleep(0.2)
        # send_keys(child, " # integration-test")
        # time.sleep(0.2)
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)

        # Verify file on disk
        # target = os.path.join(ALFRED_BIN_CRATE, "Cargo.toml")
        # saved = read_file(target)
        # assert "# integration-test" in saved
        pass
```

### M4-3: Mode returns to normal after file selection

```
Given the user has selected a file via overlay search
Then the editor is in normal mode
And the user can use normal mode commands
```

```python
    def test_mode_returns_to_normal_after_selection(self):
        """After overlay selection, editor is in normal mode -- i enters insert."""
        # Given: open overlay, select a file
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "Cargo.lock":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Then: in normal mode -- i should enter insert mode
        # send_keys(child, "i")
        # time.sleep(0.3)
        # send_keys(child, "NORMAL_MODE_OK")
        # time.sleep(0.3)
        # send_escape(child)
        # time.sleep(0.3)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)

        # Verify the insert worked (proves we were in normal mode)
        # saved = read_file(os.path.join(ALFRED_PROJECT, "Cargo.lock"))
        # assert "NORMAL_MODE_OK" in saved
        pass
```

### M4-4: Overlay search works after switching files (regression guard)

```
Given the user has already opened a file via overlay search
When the user presses Ctrl-p again to search for a different file
And the user selects the new file
Then the new file opens in the editor
```

```python
    def test_overlay_search_works_after_file_switch(self):
        """Opening a second file via overlay after the first works correctly."""
        # Given: open first file via overlay
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "Cargo.lock":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # When: open overlay again, search for different file
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "main.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Then: second file is open -- verify by editing and saving
        # send_keys(child, "A")
        # time.sleep(0.2)
        # send_keys(child, " // second-switch")
        # time.sleep(0.2)
        # child.send("\x1b")
        # time.sleep(0.3)
        # send_colon_command(child, "wq")
        # wait_for_exit(child)
        # target = os.path.join(ALFRED_PROJECT, "crates", "alfred-bin", "src", "main.rs")
        # saved = read_file(target)
        # assert "// second-switch" in saved
        pass
```

### M4-5: Overlay does not corrupt display when file changes gutter width (regression guard)

```
Given the user has opened a large file via overlay
When the user opens a small file via overlay
Then the editor renders correctly without display corruption
```

```python
    def test_overlay_file_switch_no_display_corruption(self):
        """Switching from large to small file via overlay doesn't corrupt display."""
        # Given: open a large file (buffer.rs) via overlay
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "buffer.rs":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # When: open a small file (Makefile) via overlay
        # child.send("\x10")
        # time.sleep(0.5)
        # for ch in "makefile":
        #     send_keys(child, ch)
        #     time.sleep(0.15)
        # time.sleep(0.5)
        # child.send("\r")
        # time.sleep(1.0)

        # Then: verify clean rendering by checking Makefile content appears
        # try:
        #     child.expect(".PHONY", timeout=5)
        # except pexpect.TIMEOUT:
        #     send_colon_command(child, "q!")
        #     wait_for_exit(child)
        #     pytest.fail("Display corrupted: .PHONY not visible")

        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

### M4-6: Mode returns to browse after Escape from overlay opened in browser mode (error path)

```
Given the user is in browser mode and opens the overlay with Ctrl-p
When the user presses Escape without selecting a file
Then the user returns to browser mode
And the user can navigate the browser panel normally
```

```python
    def test_escape_returns_to_browse_mode(self):
        """Escape from overlay returns to browser mode when opened from browser."""
        # Given: browser mode focused
        # spawn_alfred(ALFRED_PROJECT)
        # child.expect("crates/", timeout=5)

        # Open overlay from browser
        # child.send("\x10")
        # time.sleep(0.5)
        # type something
        # send_keys(child, "abc")
        # time.sleep(0.3)

        # When: Escape
        # child.send("\x1b")
        # time.sleep(0.5)

        # Then: back in browse mode -- verify by navigating with j and selecting
        # send_keys(child, "j")
        # time.sleep(0.2)
        # child.send("\r")
        # time.sleep(1.0)

        # Verify: a file opened (proves browser navigation worked)
        # send_colon_command(child, "q")
        # exit_code = wait_for_exit(child)
        # assert exit_code == 0
        pass
```

---

## Scenario Summary

| ID    | Scenario                                         | Type       |
|-------|--------------------------------------------------|------------|
| WS-1  | Find and open file via overlay search            | Walking skeleton |
| M1-1  | Ctrl-p opens overlay from browser mode           | Happy path |
| M1-2  | Escape closes overlay without action             | Happy path |
| M1-3  | Ctrl-p works from normal mode                    | Happy path |
| M1-4  | Ctrl-p works from browser panel                  | Happy path |
| M1-5  | Overlay opens fresh after previous dismiss       | Error path |
| M1-6  | Ctrl-p while overlay open does not crash         | Error path |
| M1-7  | Overlay dismiss does not leak typed chars         | Error path |
| M2-1  | Typing filters results                           | Happy path |
| M2-2  | Backspace updates filter                         | Happy path |
| M2-3  | Empty query shows all files                      | Boundary   |
| M2-4  | Search is case-insensitive                       | Happy path |
| M2-5  | No matches -- Enter does nothing                 | Error path |
| M2-6  | Search skips target/ directory                   | Edge case  |
| M2-7  | Backspace on empty query does not crash          | Error path |
| M2-8  | Rapid type-delete-retype shows correct results   | Error path |
| M3-1  | Arrow down selects next result                   | Happy path |
| M3-2  | Arrow up selects previous result                 | Happy path |
| M3-3  | Enter opens highlighted file, correct content    | Happy path |
| M3-4  | Arrow up at top stays (boundary)                 | Boundary   |
| M3-5  | Arrow down at bottom stays (boundary)            | Boundary   |
| M3-6  | Navigation resets on query change                | Boundary   |
| M4-1  | Browser navigates to parent after selection      | Integration |
| M4-2  | Editor shows selected file content               | Happy path |
| M4-3  | Mode returns to normal after selection           | Happy path |
| M4-4  | Overlay works after file switch                  | Error path |
| M4-5  | No display corruption on gutter width change     | Error path |
| M4-6  | Escape returns to browse mode                    | Error path |

**Counts**: 28 scenarios total
- Walking skeleton: 1 (4%)
- Happy path: 12 (43%)
- Error path: 9 (32%)
- Boundary: 4 (14%)
- Edge case: 1 (4%)
- Integration: 1 (4%)

**Error+boundary+edge ratio**: 14/28 = 50% (exceeds 40% target).

---

## Test Classes for test_alfred.py

When implementing, add these classes to `tests/e2e/test_alfred.py`:

```
TestOverlaySearchWalkingSkeleton  (1 test)  -- WS-1
TestOverlayLifecycle              (7 tests) -- M1-1 through M1-7
TestOverlaySearch                 (8 tests) -- M2-1 through M2-8
TestOverlayNavigation             (6 tests) -- M3-1 through M3-6
TestOverlayBrowserIntegration     (6 tests) -- M4-1 through M4-6
```

## Implementation Sequence (One at a Time)

Enable tests in this order, implementing production code to make each pass
before enabling the next:

1. **WS-1** -- Walking skeleton (Ctrl-p, type, Enter, file opens)
2. M1-1 -- Overlay renders with prompt
3. M1-2 -- Escape dismisses
4. M2-1 -- Typing filters results
5. M3-3 -- Enter opens highlighted file
6. M2-2 -- Backspace
7. M3-1 -- Arrow down navigation
8. M3-2 -- Arrow up navigation
9. M2-3 -- Empty query shows all
10. M2-4 -- Case-insensitive search
11. M1-3 -- Ctrl-p from normal mode
12. M1-4 -- Ctrl-p from browser panel
13. M4-2 -- Editor shows file content
14. M4-3 -- Mode returns to normal
15. M4-1 -- Browser navigates to parent
16. M2-5 -- No matches (error path)
17. M2-7 -- Backspace on empty query (error path)
18. M3-4 -- Arrow up at top (boundary)
19. M3-5 -- Arrow down at bottom (boundary)
20. M3-6 -- Navigation resets on query change (boundary)
21. M1-5 -- Fresh state after dismiss
22. M1-6 -- Ctrl-p while overlay open (error path)
23. M1-7 -- Overlay dismiss does not leak chars (error path)
24. M2-8 -- Rapid type-delete-retype (error path)
25. M2-6 -- Skips target/ directory
26. M4-4 -- File switch via overlay
27. M4-5 -- No display corruption on switch
28. M4-6 -- Escape returns to browse mode
