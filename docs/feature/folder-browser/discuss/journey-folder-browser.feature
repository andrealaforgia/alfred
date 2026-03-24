Feature: Folder Browser
  As a terminal-native developer,
  when I open Alfred with a directory path,
  I want to browse the directory tree and select a file to open,
  so I can find and edit files without leaving Alfred or memorizing paths.

  Traces to: JS-01 (Navigate to file in unfamiliar project),
             JS-02 (Quickly open known file from project root),
             JS-03 (Explore project structure)

  # =========================================================================
  # Step 1: Detect Directory Argument
  # =========================================================================

  Scenario: Directory argument enters browser mode
    Given Kai runs "alfred ." in the directory ~/projects/webapi
    When Alfred starts
    Then Alfred enters folder browser mode
    And the browser displays the contents of ~/projects/webapi

  Scenario: Absolute directory path enters browser mode
    Given Kai runs "alfred /home/kai/projects/webapi"
    When Alfred starts
    Then Alfred enters folder browser mode
    And the browser displays the contents of /home/kai/projects/webapi

  Scenario: File argument opens editor normally
    Given Kai runs "alfred src/main.rs" in ~/projects/webapi
    When Alfred starts
    Then Alfred opens src/main.rs in the editor buffer
    And the mode is "NORMAL"

  Scenario: No argument opens empty buffer
    Given Kai runs "alfred" with no arguments
    When Alfred starts
    Then Alfred opens with an empty buffer
    And the mode is "NORMAL"

  Scenario: Nonexistent path shows error and exits
    Given Kai runs "alfred /nonexistent/path"
    When Alfred starts
    Then Alfred prints "alfred: no such file or directory: /nonexistent/path" to stderr
    And Alfred exits with a non-zero exit code

  Scenario: Permission denied on directory shows error and exits
    Given Kai runs "alfred /root/secrets" and Kai lacks read permission
    When Alfred starts
    Then Alfred prints "alfred: permission denied: /root/secrets" to stderr
    And Alfred exits with a non-zero exit code

  # =========================================================================
  # Step 2: Display Tree View
  # =========================================================================

  Scenario: Browser displays directory contents sorted correctly
    Given Kai has entered folder browser mode in ~/projects/webapi
    And the directory contains: src/ (dir), tests/ (dir), Cargo.toml (file), README.md (file), .gitignore (file)
    When the browser renders
    Then the entries are displayed in this order:
      | entry       | type |
      | src/        | dir  |
      | tests/      | dir  |
      | .gitignore  | file |
      | Cargo.toml  | file |
      | README.md   | file |
    And directories have a trailing "/" indicator
    And the cursor highlights the first entry "src/"

  Scenario: Status bar shows BROWSE mode and current path
    Given Kai has entered folder browser mode in ~/projects/webapi
    When the browser renders
    Then the status bar displays "BROWSE" as the current mode
    And the status bar displays "webapi/" as the current directory

  Scenario: Empty directory shows informative message
    Given Kai has entered folder browser mode in ~/projects/empty-dir
    And the directory contains no entries
    When the browser renders
    Then the browser displays "Directory is empty"
    And Kai can press "q" to quit or "h" to go to parent

  # =========================================================================
  # Step 3: Navigate and Select
  # =========================================================================

  Scenario: Move cursor down with j
    Given Kai is in the folder browser viewing ~/projects/webapi
    And the cursor is on the first entry "src/"
    When Kai presses "j"
    Then the cursor moves to the second entry "tests/"

  Scenario: Move cursor up with k
    Given Kai is in the folder browser viewing ~/projects/webapi
    And the cursor is on the second entry "tests/"
    When Kai presses "k"
    Then the cursor moves to the first entry "src/"

  Scenario: Cursor does not move past first entry
    Given Kai is in the folder browser viewing ~/projects/webapi
    And the cursor is on the first entry "src/"
    When Kai presses "k"
    Then the cursor remains on the first entry "src/"

  Scenario: Cursor does not move past last entry
    Given Kai is in the folder browser viewing ~/projects/webapi
    And the cursor is on the last entry "README.md"
    When Kai presses "j"
    Then the cursor remains on the last entry "README.md"

  Scenario: Jump to first entry with gg
    Given Kai is in the folder browser with cursor on "README.md" (last entry)
    When Kai presses "g" followed by "g"
    Then the cursor moves to the first entry "src/"

  Scenario: Jump to last entry with G
    Given Kai is in the folder browser with cursor on "src/" (first entry)
    When Kai presses "G"
    Then the cursor moves to the last entry "README.md"

  Scenario: Enter a subdirectory
    Given Kai is in the folder browser with cursor on "src/"
    When Kai presses Enter
    Then the browser displays the contents of ~/projects/webapi/src
    And the first entry is "../"
    And the status bar shows "BROWSE webapi/src/"

  Scenario: Enter subdirectory with l key
    Given Kai is in the folder browser with cursor on "src/"
    When Kai presses "l"
    Then the browser displays the contents of ~/projects/webapi/src
    And the first entry is "../"

  Scenario: Navigate to parent with h
    Given Kai is browsing ~/projects/webapi/src
    When Kai presses "h"
    Then the browser displays the contents of ~/projects/webapi
    And the cursor is restored to the "src/" entry

  Scenario: Navigate to parent with Backspace
    Given Kai is browsing ~/projects/webapi/src
    When Kai presses Backspace
    Then the browser displays the contents of ~/projects/webapi

  Scenario: Navigate to parent via ../ entry
    Given Kai is browsing ~/projects/webapi/src
    And the cursor is on "../"
    When Kai presses Enter
    Then the browser displays the contents of ~/projects/webapi
    And the cursor is restored to the "src/" entry

  Scenario: Quit browser with q
    Given Kai is in the folder browser
    When Kai presses "q"
    Then Alfred exits cleanly with exit code 0

  Scenario: Quit browser with Escape
    Given Kai is in the folder browser
    When Kai presses Escape
    Then Alfred exits cleanly with exit code 0

  Scenario: Permission denied on subdirectory shows error
    Given Kai is in the folder browser with cursor on "secrets/" (no read permission)
    When Kai presses Enter
    Then the status bar shows "Permission denied: secrets/"
    And Kai remains in the folder browser
    And the cursor stays on "secrets/"

  Scenario: Follow symlink to directory
    Given Kai is in the folder browser with cursor on "config/" which is a symlink to /etc/webapi
    When Kai presses Enter
    Then the browser displays the contents of /etc/webapi
    And the status bar shows the resolved path

  Scenario: Broken symlink shows error
    Given Kai is in the folder browser with cursor on "old-link" which is a broken symlink
    When Kai presses Enter
    Then the status bar shows "Broken symlink: old-link"
    And Kai remains in the folder browser

  # =========================================================================
  # Step 4: Open File in Buffer
  # =========================================================================

  Scenario: Open a Rust file from the browser
    Given Kai is in the folder browser at ~/projects/webapi/src with cursor on "main.rs"
    When Kai presses Enter
    Then the browser closes
    And the file ~/projects/webapi/src/main.rs is loaded into the editor buffer
    And the mode changes to "NORMAL"
    And the status bar shows "NORMAL" and the filename "src/main.rs"
    And the cursor is at line 1, column 1
    And syntax highlighting is active for Rust

  Scenario: Open a Python file from the browser
    Given Kai is in the folder browser at ~/projects/dataproc with cursor on "pipeline.py"
    When Kai presses Enter
    Then the file ~/projects/dataproc/pipeline.py is loaded into the editor buffer
    And syntax highlighting is active for Python

  Scenario: Open file with l key
    Given Kai is in the folder browser with cursor on "Cargo.toml"
    When Kai presses "l"
    Then the file is loaded into the editor buffer
    And the mode changes to "NORMAL"

  Scenario: Opened file buffer is unmodified
    Given Kai opens "main.rs" from the folder browser
    When the file is loaded into the editor buffer
    Then the buffer modified flag is false

  Scenario: Attempting to open a binary file stays in browser
    Given Kai is in the folder browser with cursor on "logo.png"
    When Kai presses Enter
    Then the status bar shows "Cannot open binary file: logo.png"
    And Kai remains in the folder browser
    And the cursor stays on "logo.png"

  Scenario: Permission denied on file stays in browser
    Given Kai is in the folder browser with cursor on "secret.key" (no read permission)
    When Kai presses Enter
    Then the status bar shows "Permission denied: secret.key"
    And Kai remains in the folder browser

  # =========================================================================
  # Properties (ongoing qualities)
  # =========================================================================

  @property
  Scenario: Browser renders within perceptible time
    Given a directory with fewer than 10000 entries
    When the browser displays the directory
    Then the initial render completes in under 100 milliseconds

  @property
  Scenario: Navigation input responsiveness
    Given Kai is navigating the folder browser
    When Kai presses any navigation key (j, k, Enter, h)
    Then visual feedback appears in under 50 milliseconds

  @property
  Scenario: Vim key binding consistency
    Given Alfred has j/k/h/l/gg/G bindings in normal mode
    When Kai uses j/k/h/l/gg/G in the folder browser
    Then the behavior matches the vim directional convention
    And no normal-mode keybindings conflict with browser-mode bindings
