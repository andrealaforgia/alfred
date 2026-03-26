Feature: Browser File Search
  As Kai Nakamura, a backend developer browsing project directories in Alfred,
  I want to type a partial filename to filter the browser listing,
  so I can find and open files quickly without tedious j/k scrolling.

  Background:
    Given Alfred is open in folder browser mode
    And Kai is browsing the directory /home/kai/projects/webapi/src
    And the directory contains the following entries:
      | Name               | Type |
      | app.rs             | file |
      | auth.rs            | file |
      | bridge.rs          | file |
      | bridge_helpers.rs  | file |
      | cache.rs           | file |
      | config.rs          | file |
      | database.rs        | file |
      | error.rs           | file |
      | handler.rs         | file |
      | input.rs           | file |
      | lib.rs             | file |
      | logger.rs          | file |
      | main.rs            | file |
      | middleware.rs       | file |
      | models             | dir  |
      | routes             | dir  |
      | runtime.rs         | file |
      | server.rs          | file |
      | state.rs           | file |
      | tests              | dir  |
      | utils.rs           | file |
      | validator.rs       | file |

  # --- Step 2: Enter Search Mode ---

  Scenario: Activate search mode in full-screen browser
    Given Kai is in the full-screen browser with keymap "browse-mode"
    When Kai presses /
    Then a search prompt "/" appears on line 2 of the browser display
    And the full directory listing remains visible below the prompt
    And the browser accepts text input for the search query

  Scenario: Activate search mode in sidebar
    Given Kai has the sidebar open with keymap "filetree-mode"
    When Kai presses /
    Then a search prompt "/" appears on line 2 of the sidebar panel
    And the full directory listing remains visible below the prompt
    And the sidebar accepts text input for the search query

  # --- Step 3: Incremental Filtering ---

  Scenario: Single character filters entries
    Given Kai is in search mode in the full-screen browser
    When Kai types "r"
    Then the listing shows entries containing "r":
      | Name               | Type |
      | bridge.rs          | file |
      | bridge_helpers.rs  | file |
      | error.rs           | file |
      | handler.rs         | file |
      | logger.rs          | file |
      | middleware.rs       | file |
      | routes             | dir  |
      | runtime.rs         | file |
      | server.rs          | file |
      | validator.rs       | file |
    And the cursor is on the first matching entry "bridge.rs"

  Scenario: Multiple characters narrow the results further
    Given Kai is in search mode in the full-screen browser
    And Kai has typed "run"
    Then the listing shows only "runtime.rs"
    And the cursor is on "runtime.rs"

  Scenario: Case-insensitive matching
    Given Kai is in search mode in the full-screen browser
    And the directory also contains "README.md"
    When Kai types "rea"
    Then "README.md" is visible in the filtered list

  Scenario: Navigate filtered results with j/k
    Given Kai is in search mode in the full-screen browser
    And the search query is "br"
    And the filtered list shows "bridge.rs" and "bridge_helpers.rs"
    And the cursor is on "bridge.rs"
    When Kai presses j
    Then the cursor moves to "bridge_helpers.rs"

  Scenario: Backspace edits the query
    Given Kai is in search mode in the full-screen browser
    And the search query is "bri"
    And the filtered list shows "bridge.rs" and "bridge_helpers.rs"
    When Kai presses Backspace
    Then the search query becomes "br"
    And the filtered list updates to show all entries containing "br"

  Scenario: Directories are included in search results
    Given Kai is in search mode in the full-screen browser
    When Kai types "mod"
    Then the filtered list shows "models/" as a directory entry
    And the cursor is on "models/"

  # --- Step 4: No Matches ---

  Scenario: No entries match the search query
    Given Kai is in search mode in the full-screen browser
    When Kai types "xyz"
    Then the listing shows "(no matches)" instead of entries
    And pressing Enter does nothing

  Scenario: Recover from no matches with Backspace
    Given Kai is in search mode in the full-screen browser
    And the search query is "xyz" showing no matches
    When Kai presses Backspace
    Then the search query becomes "xy"
    And the listing re-filters (possibly still no matches, or entries appear)

  # --- Step 5: Open File from Search Results ---

  Scenario: Open a file from filtered results in full-screen browser
    Given Kai is in search mode in the full-screen browser
    And the search query is "run"
    And the cursor is on "runtime.rs"
    When Kai presses Enter
    Then Alfred opens /home/kai/projects/webapi/src/runtime.rs in the editor buffer
    And the editor mode transitions to "normal"
    And search mode is dismissed

  Scenario: Enter a directory from filtered results
    Given Kai is in search mode in the full-screen browser
    And the search query is "mod"
    And the cursor is on "models/"
    When Kai presses Enter
    Then the browser navigates into /home/kai/projects/webapi/src/models
    And the full listing of the models directory is displayed
    And search mode is dismissed
    And no search filter is carried into the new directory

  Scenario: Open a file from filtered results in sidebar
    Given Kai is in search mode in the sidebar
    And the search query is "run"
    And the cursor is on "runtime.rs"
    When Kai presses Enter
    Then Alfred opens /home/kai/projects/webapi/src/runtime.rs in the editor buffer
    And the sidebar unfocuses
    And the editor mode transitions to "normal"
    And search mode is dismissed

  # --- Step 6: Dismiss Search ---

  Scenario: Escape dismisses search and restores full listing
    Given Kai is in search mode in the full-screen browser
    And the search query is "br" showing 2 filtered entries
    And the cursor was on entry 5 before search was activated
    When Kai presses Escape
    Then the search prompt disappears
    And the full directory listing with all 23 entries is restored
    And the cursor returns to entry 5 (its pre-search position)

  Scenario: Escape dismisses search in sidebar
    Given Kai is in search mode in the sidebar
    And the search query is "br"
    When Kai presses Escape
    Then the search prompt disappears from the sidebar panel
    And the full directory listing is restored in the sidebar
    And the sidebar cursor returns to its pre-search position

  Scenario: Backspace on empty query dismisses search
    Given Kai is in search mode in the full-screen browser
    And the search query is empty (just the "/" prompt)
    When Kai presses Backspace
    Then search mode is dismissed
    And the full directory listing is restored
    And the cursor returns to its pre-search position

  # --- Properties ---

  @property
  Scenario: Search filtering is instant
    Given Kai is in search mode in either the full-screen browser or sidebar
    When Kai types any character
    Then the filtered listing updates within one render cycle
    And there is no perceptible delay between keystroke and display update

  @property
  Scenario: Behavior consistency between full-screen and sidebar
    Given a search query produces a set of matching entries in the full-screen browser
    When the same search query is applied in the sidebar for the same directory
    Then the same set of matching entries is displayed
    And the same keyboard interactions (j/k/Enter/Escape/Backspace) produce the same results
