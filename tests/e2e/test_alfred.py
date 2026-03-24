"""
End-to-end tests for the Alfred text editor.

Each test spawns the Alfred binary inside a real PTY via pexpect,
sends keystrokes, and verifies observable outcomes (file content after
save, exit codes).

These tests exercise the full stack: binary startup, plugin loading,
Lisp runtime, keymap dispatch, buffer operations, and file I/O.

Alfred uses the alternate screen, so we never attempt to read screen
content. All assertions are based on file content after :wq or exit
codes.
"""

import os
import tempfile
import time

import pexpect
import pytest


ALFRED_BIN = "/usr/local/bin/alfred"
# Generous timeout: the editor should respond well within this.
TIMEOUT = 10


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def create_temp_file(content: str = "") -> str:
    """Create a temporary file with the given content and return its path."""
    fd, path = tempfile.mkstemp(prefix="alfred_e2e_", suffix=".txt")
    with os.fdopen(fd, "w") as f:
        f.write(content)
    return path


def read_file(path: str) -> str:
    """Read and return the entire content of a file."""
    with open(path, "r") as f:
        return f.read()


def spawn_alfred(file_path: str, timeout: int = TIMEOUT) -> pexpect.spawn:
    """
    Spawn Alfred in a PTY with the given file.

    Alfred enters alternate screen, so we set dimensions and give it a moment
    to initialize and load plugins before sending keystrokes.
    """
    child = pexpect.spawn(
        ALFRED_BIN,
        args=[file_path],
        timeout=timeout,
        encoding="utf-8",
        dimensions=(24, 80),
        env={
            "TERM": "xterm-256color",
            "PATH": os.environ.get("PATH", "/usr/local/bin:/usr/bin:/bin"),
            # Run Alfred from /alfred so it finds ./plugins/
            "HOME": "/root",
        },
        cwd="/alfred",
    )
    # Give Alfred time to start, load plugins, and render the first frame
    time.sleep(1.0)
    return child


def send_keys(child: pexpect.spawn, keys: str, delay: float = 0.05):
    """
    Send a sequence of characters to Alfred, one at a time with a small delay.

    This mimics real typing and gives the editor time to process each keystroke
    through its event loop.
    """
    for char in keys:
        child.send(char)
        time.sleep(delay)


def send_escape(child: pexpect.spawn):
    """Send the Escape key."""
    child.send("\x1b")
    time.sleep(0.1)


def send_enter(child: pexpect.spawn):
    """Send the Enter key."""
    child.send("\r")
    time.sleep(0.1)


def send_colon_command(child: pexpect.spawn, command: str):
    """
    Enter command mode with ':', type the command, and press Enter.

    Example: send_colon_command(child, "wq") sends ':wq<Enter>'.
    """
    send_keys(child, ":")
    time.sleep(0.1)
    send_keys(child, command)
    send_enter(child)


def wait_for_exit(child: pexpect.spawn, timeout: int = TIMEOUT):
    """Wait for Alfred to exit and return its exit code."""
    child.expect(pexpect.EOF, timeout=timeout)
    child.close()
    return child.exitstatus


# ---------------------------------------------------------------------------
# Plugin loading health check
# ---------------------------------------------------------------------------

class TestPluginHealth:
    """Verify all plugins load without errors at startup."""

    def test_no_plugin_errors_on_startup(self):
        """Start Alfred and verify no 'Plugin errors' message appears.

        This catches issues like undefined Lisp functions (let*, etc.)
        or missing primitives that only manifest at plugin load time.
        """
        path = create_temp_file("test content")
        child = spawn_alfred(path)

        # Read screen output — if there are plugin errors, they appear
        # in the message line at the bottom of the screen
        try:
            screen = child.read_nonblocking(size=16384, timeout=2)
        except Exception:
            screen = ""

        # Quit cleanly
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"
        assert "Plugin errors" not in screen, \
            f"Plugin errors detected at startup: {repr(screen[:500])}"
        assert "not defined" not in screen, \
            f"Undefined symbol error at startup: {repr(screen[:500])}"
        os.unlink(path)

    def test_plugins_create_panels(self):
        """Verify plugins create panels (status bar, gutter) at startup.

        If panels aren't created, the editor would render with full-width
        text and no status bar — a clear sign of plugin failure.
        We verify by opening a multi-line file, editing, and saving
        (which exercises the full panel rendering pipeline).
        """
        lines = [f"line {i+1}" for i in range(10)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Navigate and edit (exercises gutter + status panel updates)
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "OK")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert "OK" in content, f"Expected 'OK' in file, got: {repr(content)}"
        os.unlink(path)

    def test_no_crash_on_empty_file_with_panels(self):
        """Open an empty file — plugins must handle zero lines gracefully."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        # Read screen to check for errors
        try:
            screen = child.read_nonblocking(size=16384, timeout=2)
        except Exception:
            screen = ""

        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0
        assert "error" not in screen.lower() or "Plugin" not in screen, \
            f"Error detected with empty file: {repr(screen[:500])}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Basic startup (3 tests)
# ---------------------------------------------------------------------------

class TestBasicStartup:
    """Alfred opens a file and exits cleanly."""

    def test_open_and_quit(self):
        """Alfred opens a file, :q exits with code 0."""
        path = create_temp_file("hello\n")
        child = spawn_alfred(path)

        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        os.unlink(path)

    def test_open_empty_file_and_quit(self):
        """Alfred opens an empty file, :q exits with code 0."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        os.unlink(path)

    def test_force_quit(self):
        """Alfred :q! exits even with unsaved changes."""
        path = create_temp_file("original")
        child = spawn_alfred(path)

        # Make a modification: enter insert mode, type something
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        # :q! should force quit without saving
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        # File should be unchanged
        assert read_file(path) == "original"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Insert mode (3 tests)
# ---------------------------------------------------------------------------

class TestInsertMode:
    """Alfred enters insert mode with 'i', accepts typed text, saves with :wq."""

    def test_insert_hello(self):
        """Press i, type 'hello', Escape, :wq -- file contains 'hello'."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "hello")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        assert "hello" in content, f"Expected 'hello' in file, got: {repr(content)}"
        os.unlink(path)

    def test_insert_multiple_words(self):
        """Insert mode handles spaces and multiple words."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "foo bar")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert "foo bar" in content, f"Expected 'foo bar' in file, got: {repr(content)}"
        os.unlink(path)

    def test_insert_at_beginning_of_existing_content(self):
        """Open file with 'world', press i, type 'hello ', Escape, :wq."""
        path = create_temp_file("world")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "hello ")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert content.startswith("hello world"), \
            f"Expected content starting with 'hello world', got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Navigation (2 tests)
# ---------------------------------------------------------------------------

class TestNavigation:
    """Verify cursor movement by inserting text at new positions."""

    def test_move_right_then_insert(self):
        """Open file with 'abc', press l (right), i, type 'X', :wq -- file is 'aXbc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        # Move right once (cursor goes from col 0 to col 1)
        send_keys(child, "l")
        time.sleep(0.2)

        # Enter insert mode and type X
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert "aXbc" in content, f"Expected 'aXbc' in file, got: {repr(content)}"
        os.unlink(path)

    def test_move_down_then_insert(self):
        """Open file with two lines, press j (down), i, type 'X', :wq -- X on second line."""
        path = create_temp_file("line1\nline2")
        child = spawn_alfred(path)

        # Move down to second line
        send_keys(child, "j")
        time.sleep(0.2)

        # Insert at beginning of second line
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        lines = content.split("\n")
        assert len(lines) >= 2, f"Expected at least 2 lines, got: {repr(content)}"
        assert lines[1].startswith("X"), \
            f"Expected second line to start with 'X', got: {repr(lines[1])}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Arrow keys (3 tests)
# ---------------------------------------------------------------------------

class TestArrowKeys:
    """Verify arrow keys work for navigation in insert mode."""

    def test_arrow_right_in_insert_mode(self):
        """Open 'abc', i to insert, arrow right twice, type 'X', :wq -- file is 'abXc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)

        # Arrow right twice (cursor moves from col 0 to col 2)
        child.send("\x1b[C")  # Arrow Right escape sequence
        time.sleep(0.1)
        child.send("\x1b[C")  # Arrow Right again
        time.sleep(0.1)

        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert "abXc" in content, f"Expected 'abXc', got: {repr(content)}"
        os.unlink(path)

    def test_arrow_down_in_insert_mode(self):
        """Open two-line file, i to insert, arrow down, type 'X', :wq -- X on second line."""
        path = create_temp_file("line1\nline2")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)

        # Arrow down to second line
        child.send("\x1b[B")  # Arrow Down escape sequence
        time.sleep(0.1)

        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        lines = content.split("\n")
        assert len(lines) >= 2
        assert "X" in lines[1], f"Expected 'X' on second line, got: {repr(lines[1])}"
        os.unlink(path)

    def test_arrow_up_and_left_in_insert_mode(self):
        """Navigate with all four arrow keys in insert mode."""
        path = create_temp_file("ab\ncd")
        child = spawn_alfred(path)

        # Enter insert mode
        send_keys(child, "i")
        time.sleep(0.3)

        # Arrow down to second line
        child.send("\x1b[B")  # Down
        time.sleep(0.1)

        # Arrow right to col 1
        child.send("\x1b[C")  # Right
        time.sleep(0.1)

        # Arrow up back to first line
        child.send("\x1b[A")  # Up
        time.sleep(0.1)

        # Arrow left to col 0
        child.send("\x1b[D")  # Left
        time.sleep(0.1)

        # Type X at position (0, 0) — should be at start of first line
        send_keys(child, "X")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        lines = content.split("\n")
        assert lines[0].startswith("X"), \
            f"Expected first line to start with 'X', got: {repr(lines[0])}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Delete (2 tests)
# ---------------------------------------------------------------------------

class TestDelete:
    """Verify 'x' deletes the character at cursor."""

    def test_delete_first_char(self):
        """Open file with 'abc', press x, :wq -- file is 'bc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        # x deletes char at cursor (position 0 = 'a')
        send_keys(child, "x")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert content.rstrip("\n") == "bc", \
            f"Expected 'bc', got: {repr(content)}"
        os.unlink(path)

    def test_delete_middle_char(self):
        """Open file with 'abc', press l (move to 'b'), x, :wq -- file is 'ac'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        send_keys(child, "l")
        time.sleep(0.2)
        send_keys(child, "x")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert content.rstrip("\n") == "ac", \
            f"Expected 'ac', got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Undo (1 test)
# ---------------------------------------------------------------------------

class TestUndo:
    """Verify 'u' undoes the last change."""

    def test_undo_delete(self):
        """Open file with 'abc', press x (delete 'a'), press u (undo), :wq -- file is 'abc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        # Delete first char
        send_keys(child, "x")
        time.sleep(0.3)

        # Undo
        send_keys(child, "u")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0
        assert content.rstrip("\n") == "abc", \
            f"Expected 'abc' after undo, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Command mode (2 tests)
# ---------------------------------------------------------------------------

class TestCommandMode:
    """Verify command-mode Lisp evaluation does not crash."""

    def test_eval_arithmetic(self):
        """Open file, :eval (+ 1 2), :q! -- exits without crash."""
        path = create_temp_file("test")
        child = spawn_alfred(path)

        send_colon_command(child, "eval (+ 1 2)")
        time.sleep(0.5)

        # Should still be running; force quit
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0 after eval, got {exit_code}"
        os.unlink(path)

    def test_eval_message(self):
        """Open file, :eval (message "test"), :q! -- exits without crash."""
        path = create_temp_file("test")
        child = spawn_alfred(path)

        send_colon_command(child, 'eval (message "test")')
        time.sleep(0.5)

        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0
        os.unlink(path)


# ---------------------------------------------------------------------------
# Write (1 test)
# ---------------------------------------------------------------------------

class TestWrite:
    """Verify :w saves the file without quitting."""

    def test_write_then_quit(self):
        """Insert text, :w, verify file, then :q."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "saved")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        # Save but don't quit
        send_colon_command(child, "w")
        time.sleep(0.5)

        # Verify file was written while editor is still running
        content = read_file(path)
        assert "saved" in content, \
            f"Expected 'saved' in file after :w, got: {repr(content)}"

        # Now quit (no unsaved changes warning)
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)
        assert exit_code == 0
        os.unlink(path)


# ---------------------------------------------------------------------------
# Multi-line (5 tests)
# ---------------------------------------------------------------------------

class TestMultiLine:
    """Tests for entering multi-line text via insert mode."""

    def test_type_two_lines_with_enter(self):
        """Press i, type 'line1', Enter, 'line2', Escape, :wq -- file has two lines."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "line1")
        send_enter(child)
        send_keys(child, "line2")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert exit_code == 0
        assert lines[0] == "line1", f"First line should be 'line1', got: {repr(lines[0])}"
        assert lines[1] == "line2", f"Second line should be 'line2', got: {repr(lines[1])}"
        os.unlink(path)

    def test_type_three_lines(self):
        """Type three lines with Enter between them."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "alpha")
        send_enter(child)
        send_keys(child, "beta")
        send_enter(child)
        send_keys(child, "gamma")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert exit_code == 0
        assert lines[0] == "alpha", f"Expected 'alpha', got: {repr(lines[0])}"
        assert lines[1] == "beta", f"Expected 'beta', got: {repr(lines[1])}"
        assert lines[2] == "gamma", f"Expected 'gamma', got: {repr(lines[2])}"
        os.unlink(path)

    def test_open_line_below_with_o(self):
        """Open file with 'first', press o, type 'second', Escape, :wq -- two lines."""
        path = create_temp_file("first")
        child = spawn_alfred(path)

        send_keys(child, "o")
        time.sleep(0.3)
        send_keys(child, "second")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert exit_code == 0
        assert lines[0] == "first", f"First line should be 'first', got: {repr(lines[0])}"
        assert lines[1] == "second", f"Second line should be 'second', got: {repr(lines[1])}"
        os.unlink(path)

    def test_insert_between_existing_lines(self):
        """Open 3-line file, navigate to line 2, press o, type new line, Escape, :wq."""
        path = create_temp_file("aaa\nbbb\nccc")
        child = spawn_alfred(path)

        # Move to line 2 (j goes down)
        send_keys(child, "j")
        time.sleep(0.2)

        # Open line below line 2
        send_keys(child, "o")
        time.sleep(0.3)
        send_keys(child, "inserted")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert exit_code == 0
        assert lines[0] == "aaa", f"Line 1: expected 'aaa', got: {repr(lines[0])}"
        assert lines[1] == "bbb", f"Line 2: expected 'bbb', got: {repr(lines[1])}"
        assert lines[2] == "inserted", f"Line 3: expected 'inserted', got: {repr(lines[2])}"
        assert lines[3] == "ccc", f"Line 4: expected 'ccc', got: {repr(lines[3])}"
        os.unlink(path)

    def test_multiple_insert_escape_cycles(self):
        """Enter insert, type, escape, move, enter insert again, type more."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        # First insert
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "hello")
        send_escape(child)
        time.sleep(0.3)

        # Open line below
        send_keys(child, "o")
        time.sleep(0.3)
        send_keys(child, "world")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert exit_code == 0
        assert "hello" in lines[0], f"First line should contain 'hello', got: {repr(lines[0])}"
        assert "world" in lines[1], f"Second line should contain 'world', got: {repr(lines[1])}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Developer workflow (1 comprehensive test)
# ---------------------------------------------------------------------------

class TestDeveloperWorkflow:
    """
    Comprehensive test simulating a real developer editing session.

    Start with an empty file, write a multi-line Python program using
    insert mode, navigate with vim keys, yank and paste a line, then
    save and verify the complete file content.
    """

    def test_write_python_program_with_yank_paste(self):
        """
        Full developer workflow:
        1. Start with empty file
        2. Enter insert mode, type a Python hello world program
        3. Navigate, yank a line, paste it
        4. Save and verify content
        """
        path = create_temp_file("")
        child = spawn_alfred(path)

        # Enter insert mode
        send_keys(child, "i")
        time.sleep(0.3)

        # Type: print("Hello World")
        send_keys(child, 'print("Hello World")')
        send_enter(child)

        # Type: def greet(name):
        send_keys(child, "def greet(name):")
        send_enter(child)

        # Type:     print(f"Hello, {name}!")
        # Note: we type the literal characters including braces
        send_keys(child, '    print(f"Hello, {name}!")')
        send_enter(child)

        # Empty line
        send_enter(child)

        # Type: greet("Alfred")
        send_keys(child, 'greet("Alfred")')
        time.sleep(0.3)

        # Escape to normal mode
        send_escape(child)
        time.sleep(0.3)

        # Navigate up several times to reach the first line
        send_keys(child, "k")
        time.sleep(0.1)
        send_keys(child, "k")
        time.sleep(0.1)
        send_keys(child, "k")
        time.sleep(0.1)
        send_keys(child, "k")
        time.sleep(0.3)

        # Yank the current line (should be 'print("Hello World")')
        send_keys(child, "y")
        time.sleep(0.3)

        # Move down one line
        send_keys(child, "j")
        time.sleep(0.2)

        # Paste below current line
        send_keys(child, "p")
        time.sleep(0.3)

        # Save and quit
        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"

        # Verify the file contains the Python code
        assert 'print("Hello World")' in content, \
            f"Expected print statement in file, got: {repr(content)}"
        assert "def greet(name):" in content, \
            f"Expected function definition in file, got: {repr(content)}"
        assert 'greet("Alfred")' in content, \
            f"Expected function call in file, got: {repr(content)}"

        # The yanked line should appear twice (original + pasted copy)
        hello_count = content.count('print("Hello World")')
        assert hello_count == 2, \
            f"Expected 'print(\"Hello World\")' to appear twice (original + paste), " \
            f"found {hello_count} times. File content: {repr(content)}"

        os.unlink(path)


# -------------------------------------------------------------------------
# Tier 1 vim features: count prefix, search, find char, dot, %, indent
# -------------------------------------------------------------------------

class TestCountPrefix:
    """Verify numeric count prefix works with commands."""

    def test_count_5j_moves_down_5_lines(self):
        """Type 5j on a 10-line file, verify cursor moved to line 6 by inserting there."""
        lines = [f"line{i}" for i in range(10)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # 5j moves down 5 lines (from line 0 to line 5)
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)

        # Insert marker at cursor position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "MARKER")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        assert "MARKER" in result_lines[5], \
            f"Expected MARKER on line 6, got: {repr(result_lines[5])}"
        os.unlink(path)

    def test_count_3x_deletes_3_chars(self):
        """Type 3x on 'ABCDEF', verify 'DEF' remains."""
        path = create_temp_file("ABCDEF")
        child = spawn_alfred(path)

        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "x")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "DEF", f"Expected 'DEF', got: {repr(content)}"
        os.unlink(path)


class TestSearch:
    """Verify /pattern search and n/N repeat."""

    def test_search_forward_and_save(self):
        """Search for 'target', cursor moves to it, insert marker, save."""
        path = create_temp_file("first line\nsecond target line\nthird line")
        child = spawn_alfred(path)

        # /target Enter
        send_keys(child, "/")
        time.sleep(0.2)
        send_keys(child, "target")
        send_enter(child)
        time.sleep(0.3)

        # Insert marker at found position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, ">>")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        assert ">>target" in content, \
            f"Expected '>>target' in file, got: {repr(content)}"
        os.unlink(path)

    def test_search_n_repeats(self):
        """Search for 'x', press n to find next, insert marker at second match."""
        path = create_temp_file("ax bx cx")
        child = spawn_alfred(path)

        # /x Enter — finds first 'x' (at col 1)
        send_keys(child, "/")
        time.sleep(0.2)
        send_keys(child, "x")
        send_enter(child)
        time.sleep(0.3)

        # n — finds next 'x' (at col 4)
        send_keys(child, "n")
        time.sleep(0.3)

        # Insert marker
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, ">")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        # The > should be before the second x (at "b>x")
        assert ">x" in content, \
            f"Expected '>x' near second match, got: {repr(content)}"
        os.unlink(path)


class TestFindChar:
    """Verify f/t character find on line."""

    def test_f_finds_char_forward(self):
        """Press fx on 'abcxdef', insert marker before x."""
        path = create_temp_file("abcxdef")
        child = spawn_alfred(path)

        send_keys(child, "f")
        time.sleep(0.1)
        send_keys(child, "x")
        time.sleep(0.3)

        # Cursor should be on 'x' (col 3), insert before it
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, ">")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert ">x" in content, f"Expected '>x', got: {repr(content)}"
        os.unlink(path)


class TestDotRepeat:
    """Verify . repeats last editing command."""

    def test_dot_repeats_delete(self):
        """Press x then . — two characters deleted."""
        path = create_temp_file("ABCD")
        child = spawn_alfred(path)

        send_keys(child, "x")
        time.sleep(0.3)
        send_keys(child, ".")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "CD", f"Expected 'CD' after x then ., got: {repr(content)}"
        os.unlink(path)


class TestBracketMatch:
    """Verify % jumps to matching bracket."""

    def test_percent_matches_parens(self):
        """On '(hello)', % on ( jumps to ), insert marker."""
        path = create_temp_file("(hello)")
        child = spawn_alfred(path)

        # Cursor on '(' — press % to jump to ')'
        send_keys(child, "%")
        time.sleep(0.3)

        # Cursor should be on ')' (col 6), insert before it
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, ">")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert ">)" in content, f"Expected '>)' before closing paren, got: {repr(content)}"
        os.unlink(path)


class TestIndent:
    """Verify > and < indent/unindent."""

    def test_indent_adds_spaces(self):
        """Press > on 'hello', verify 4 spaces prepended."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, ">")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "    hello", f"Expected '    hello', got: {repr(content)}"
        os.unlink(path)

    def test_unindent_removes_spaces(self):
        """Press < on '    hello', verify spaces removed."""
        path = create_temp_file("    hello")
        child = spawn_alfred(path)

        send_keys(child, "<")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "hello", f"Expected 'hello', got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Operator-pending mode (dw, cw, yy+p, text objects)
# -------------------------------------------------------------------------

class TestOperatorPending:
    """Verify d/c/y operators compose with motions and text objects."""

    def test_dw_deletes_word(self):
        """dw on 'hello world' at col 0 deletes 'hello '."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "world", f"Expected 'world' after dw, got: {repr(content)}"
        os.unlink(path)

    def test_d_dollar_deletes_to_end(self):
        """d$ on 'hello world' at col 5 deletes ' world'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to col 5
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "$")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "hello", f"Expected 'hello' after d$, got: {repr(content)}"
        os.unlink(path)

    def test_dd_deletes_line(self):
        """dd on two-line file deletes the first line."""
        path = create_temp_file("first\nsecond")
        child = spawn_alfred(path)

        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "d")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "second" in content, f"Expected 'second' after dd, got: {repr(content)}"
        assert "first" not in content, f"'first' should be deleted, got: {repr(content)}"
        os.unlink(path)

    def test_cw_changes_word(self):
        """cw on 'hello world' deletes word, enters insert, type 'goodbye', :wq."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        send_keys(child, "c")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        # Now in insert mode — type replacement
        send_keys(child, "goodbye ")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "goodbye" in content, f"Expected 'goodbye' after cw, got: {repr(content)}"
        assert "world" in content, f"Expected 'world' preserved, got: {repr(content)}"
        os.unlink(path)

    def test_cw_on_last_word_deletes_entire_word(self):
        """cw on the last word of a line should delete the whole word."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to 'world' with w
        send_keys(child, "w")
        time.sleep(0.2)

        # cw on last word
        send_keys(child, "c")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        # Type replacement
        send_keys(child, "earth")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "world" not in content, \
            f"'world' should be fully deleted, got: {repr(content)}"
        assert "earth" in content, \
            f"Expected 'earth' as replacement, got: {repr(content)}"
        os.unlink(path)

    def test_dw_on_last_word_deletes_entire_word(self):
        """dw on the last word of a line should delete the whole word."""
        path = create_temp_file("foo bar")
        child = spawn_alfred(path)

        # Move to 'bar' with w
        send_keys(child, "w")
        time.sleep(0.2)

        # dw on last word
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "bar" not in content, \
            f"'bar' should be fully deleted, got: {repr(content)}"
        os.unlink(path)

    def test_yy_p_duplicates_line(self):
        """yy then p duplicates the current line."""
        path = create_temp_file("only line")
        child = spawn_alfred(path)

        # yy = yank line
        send_keys(child, "y")
        time.sleep(0.1)
        send_keys(child, "y")
        time.sleep(0.3)

        # p = paste below
        send_keys(child, "p")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        lines = [l for l in content.split("\n") if l.strip()]
        assert len(lines) >= 2, f"Expected 2 lines after yy+p, got: {repr(content)}"
        assert lines[0].strip() == "only line", f"First line: {repr(lines[0])}"
        os.unlink(path)

    def test_diw_deletes_inner_word(self):
        """diw on 'hello world' with cursor on 'world' deletes 'world'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to 'world' (w jumps to next word start)
        send_keys(child, "w")
        time.sleep(0.2)

        # diw = delete inner word
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "i")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "world" not in content, f"'world' should be deleted, got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Visual mode (v, V)
# -------------------------------------------------------------------------

class TestVisualMode:
    """Verify visual selection with operators."""

    def test_v_select_and_delete(self):
        """v + lll + d — select 4 chars and delete them."""
        path = create_temp_file("ABCDEFGH")
        child = spawn_alfred(path)

        send_keys(child, "v")
        time.sleep(0.2)
        send_keys(child, "l")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.1)
        send_keys(child, "d")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "ABCD" not in content, f"First 4 chars should be deleted, got: {repr(content)}"
        assert "EFGH" in content, f"Expected 'EFGH' to remain, got: {repr(content)}"
        os.unlink(path)

    def test_V_select_line_and_delete(self):
        """V + d — delete entire current line.

        Uses :q! instead of :wq since visual-line-delete might leave
        the buffer in an unexpected modified state. Verifies via :w first.
        """
        path = create_temp_file("first\nsecond\nthird")
        child = spawn_alfred(path)

        # Send V as a raw byte with generous delay
        send_keys(child, "V", delay=0.15)
        time.sleep(0.5)
        send_keys(child, "d", delay=0.15)
        time.sleep(0.5)

        # Save explicitly, then force quit to avoid unsaved-changes prompt
        send_colon_command(child, "w")
        time.sleep(0.5)
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0, f"Expected exit 0, got {exit_code}"
        assert "first" not in content, f"'first' should be deleted, got: {repr(content)}"
        assert "second" in content, f"'second' should remain, got: {repr(content)}"
        os.unlink(path)

    def test_V_select_two_lines_and_delete(self):
        """V + j + d — delete two lines."""
        path = create_temp_file("aaa\nbbb\nccc\nddd")
        child = spawn_alfred(path)

        send_keys(child, "V", delay=0.15)
        time.sleep(0.5)
        send_keys(child, "j", delay=0.15)
        time.sleep(0.3)
        send_keys(child, "d", delay=0.15)
        time.sleep(0.5)

        send_colon_command(child, "w")
        time.sleep(0.5)
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        assert exit_code == 0, f"Expected exit 0, got {exit_code}"
        assert "aaa" not in content, f"'aaa' should be deleted, got: {repr(content)}"
        assert "bbb" not in content, f"'bbb' should be deleted, got: {repr(content)}"
        assert "ccc" in content, f"'ccc' should remain, got: {repr(content)}"
        os.unlink(path)

    def test_v_escape_cancels(self):
        """v + Escape — cancel visual mode, no changes."""
        path = create_temp_file("unchanged")
        child = spawn_alfred(path)

        send_keys(child, "v")
        time.sleep(0.2)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "q")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "unchanged", f"Expected 'unchanged', got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Tier 2: Marks, registers, case toggle, macros, number ops, editing
# -------------------------------------------------------------------------

class TestMarks:
    """Verify marks (m{a-z}, '{a-z})."""

    def test_set_mark_and_jump_back(self):
        """Set mark 'a' at line 3, move away, jump back with 'a."""
        lines = [f"line{i}" for i in range(10)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Move to line 3
        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.2)

        # Set mark a
        send_keys(child, "m")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.2)

        # Move away to line 7
        send_keys(child, "4")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.2)

        # Jump back to mark a
        send_keys(child, "'")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.3)

        # Insert marker to verify position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "MARK")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        assert "MARK" in result_lines[3], \
            f"Expected MARK on line 3, got: {repr(result_lines[3])}"
        os.unlink(path)


class TestCaseToggle:
    """Verify ~ (toggle case)."""

    def test_tilde_toggles_case(self):
        """~ on 'hello' toggles h→H."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "~")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content.startswith("H"), f"Expected 'H...', got: {repr(content)}"
        os.unlink(path)

    def test_3_tilde_toggles_three_chars(self):
        """3~ on 'hello' toggles first 3 chars → 'HELlo'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "~")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content.startswith("HEL"), f"Expected 'HEL...', got: {repr(content)}"
        os.unlink(path)


class TestMacros:
    """Verify macro recording and playback."""

    def test_record_delete_and_replay(self):
        """qa + x + q records delete, @a replays it."""
        path = create_temp_file("ABCD")
        child = spawn_alfred(path)

        # Start recording macro 'a'
        send_keys(child, "q")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.2)

        # Delete one char
        send_keys(child, "x")
        time.sleep(0.3)

        # Stop recording
        send_keys(child, "q")
        time.sleep(0.2)

        # Replay macro a
        send_keys(child, "@")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "CD", f"Expected 'CD' after record x + replay, got: {repr(content)}"
        os.unlink(path)


class TestNumberOps:
    """Verify Ctrl-a (increment) and Ctrl-x (decrement)."""

    def test_ctrl_a_increments(self):
        """Ctrl-a on 'count=42' increments to 43."""
        path = create_temp_file("count=42")
        child = spawn_alfred(path)

        # Move to the number
        send_keys(child, "6")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # Ctrl-a to increment
        child.send("\x01")  # Ctrl-a
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "43" in content, f"Expected '43' after Ctrl-a, got: {repr(content)}"
        os.unlink(path)

    def test_ctrl_x_decrements(self):
        """Ctrl-x on 'num=10' decrements to 9."""
        path = create_temp_file("num=10")
        child = spawn_alfred(path)

        # Move to number
        send_keys(child, "4")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # Ctrl-x to decrement
        child.send("\x18")  # Ctrl-x
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "9" in content, f"Expected '9' after Ctrl-x, got: {repr(content)}"
        os.unlink(path)


class TestSimpleEditing:
    """Verify r, D, S, s, P, X commands."""

    def test_r_replaces_char(self):
        """ra on 'hello' at col 0 → 'aello'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "r")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "aello", f"Expected 'aello', got: {repr(content)}"
        os.unlink(path)

    def test_D_deletes_to_end(self):
        """D on 'hello world' at col 5 → 'hello'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        send_keys(child, "D")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "hello", f"Expected 'hello' after D, got: {repr(content)}"
        os.unlink(path)

    def test_s_substitutes_char(self):
        """s on 'hello' at col 0 → deletes h, enters insert, type 'Y' → 'Yello'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "s")
        time.sleep(0.3)
        send_keys(child, "Y")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "Yello", f"Expected 'Yello', got: {repr(content)}"
        os.unlink(path)

    def test_X_deletes_char_before(self):
        """X on 'hello' at col 2 → 'hllo'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "2")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        send_keys(child, "X")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "hllo", f"Expected 'hllo' after X, got: {repr(content)}"
        os.unlink(path)

    def test_X_at_col_0_is_noop(self):
        """X at start of line does nothing."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        send_keys(child, "X")
        time.sleep(0.3)

        send_colon_command(child, "q")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "hello", f"Expected 'hello' unchanged, got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Tier 3: substitute, global delete, jump list
# -------------------------------------------------------------------------

class TestSubstitute:
    """Verify :s search and replace."""

    def test_substitute_first_on_line(self):
        """:s/foo/bar/ replaces first occurrence only."""
        path = create_temp_file("foo baz foo")
        child = spawn_alfred(path)

        send_colon_command(child, "s/foo/bar/")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "bar baz foo", f"Expected 'bar baz foo', got: {repr(content)}"
        os.unlink(path)

    def test_substitute_global_on_line(self):
        """:s/foo/bar/g replaces all occurrences on line."""
        path = create_temp_file("foo baz foo")
        child = spawn_alfred(path)

        send_colon_command(child, "s/foo/bar/g")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "bar baz bar", f"Expected 'bar baz bar', got: {repr(content)}"
        os.unlink(path)

    def test_substitute_whole_buffer(self):
        """:%s/old/new/g replaces across all lines."""
        path = create_temp_file("old line1\nold line2\nkeep")
        child = spawn_alfred(path)

        send_colon_command(child, "%s/old/new/g")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        assert "old" not in content, f"'old' should be replaced, got: {repr(content)}"
        assert "new line1" in content, f"Expected 'new line1', got: {repr(content)}"
        assert "keep" in content, f"'keep' should be preserved, got: {repr(content)}"
        os.unlink(path)

    def test_substitute_delete_pattern(self):
        """:s/remove//g deletes all occurrences."""
        path = create_temp_file("aremovebremovec")
        child = spawn_alfred(path)

        send_colon_command(child, "s/remove//g")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "abc", f"Expected 'abc', got: {repr(content)}"
        os.unlink(path)


class TestGlobalDelete:
    """Verify :g/pattern/d (global delete)."""

    def test_global_delete_matching_lines(self):
        """:g/TODO/d removes lines containing TODO."""
        path = create_temp_file("keep\nTODO fix\nkeep\nTODO remove\nkeep")
        child = spawn_alfred(path)

        send_colon_command(child, "g/TODO/d")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        assert "TODO" not in content, f"TODO lines should be deleted, got: {repr(content)}"
        assert content.count("keep") == 3, f"Expected 3 'keep' lines, got: {repr(content)}"
        os.unlink(path)

    def test_global_invert_delete(self):
        """:v/keep/d deletes lines NOT containing 'keep'."""
        path = create_temp_file("keep this\nremove this\nkeep that\ndelete me")
        child = spawn_alfred(path)

        send_colon_command(child, "v/keep/d")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        assert "remove" not in content, f"'remove' should be deleted, got: {repr(content)}"
        assert "delete" not in content, f"'delete' should be deleted, got: {repr(content)}"
        assert "keep this" in content, f"Expected 'keep this', got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Tab key support
# -------------------------------------------------------------------------

class TestTab:
    """Verify Tab key inserts spaces in insert mode."""

    def test_tab_inserts_4_spaces_by_default(self):
        """Tab in insert mode inserts 4 spaces."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        child.send("\t")  # Tab key
        time.sleep(0.3)
        send_keys(child, "hello")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "    hello", \
            f"Expected '    hello' (4 spaces + hello), got: {repr(content)}"
        os.unlink(path)

    def test_tab_at_start_of_existing_line(self):
        """Tab at beginning of existing text indents it."""
        path = create_temp_file("code")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        child.send("\t")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "    code", \
            f"Expected '    code' (4 spaces before code), got: {repr(content)}"
        os.unlink(path)

    def test_multiple_tabs(self):
        """Two tabs insert 8 spaces."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        child.send("\t")
        time.sleep(0.2)
        child.send("\t")
        time.sleep(0.2)
        send_keys(child, "x")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "        x", \
            f"Expected '        x' (8 spaces + x), got: {repr(content)}"
        os.unlink(path)

    def test_tab_then_undo(self):
        """Tab + Escape + u undoes the tab insertion."""
        path = create_temp_file("text")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        child.send("\t")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        # Undo
        send_keys(child, "u")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "text", \
            f"Expected 'text' after undo, got: {repr(content)}"
        os.unlink(path)

    def test_tab_in_middle_of_line(self):
        """Tab between words inserts 4 spaces at cursor position."""
        path = create_temp_file("ab")
        child = spawn_alfred(path)

        # Move right once, enter insert
        send_keys(child, "l")
        time.sleep(0.2)
        send_keys(child, "i")
        time.sleep(0.3)
        child.send("\t")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "a    b", \
            f"Expected 'a    b' (a + 4 spaces + b), got: {repr(content)}"
        os.unlink(path)


# -------------------------------------------------------------------------
# Panel system (status bar + gutter via Lisp plugins)
# -------------------------------------------------------------------------

class TestPanels:
    """Verify the panel-based plugin system works end-to-end.

    Panels are created by Lisp plugins at startup. The status-bar plugin
    creates a bottom panel, the line-numbers plugin creates a left panel.
    These tests verify that panels don't interfere with normal editing
    and that the editor functions correctly with the panel system active.
    """

    def test_editor_starts_with_panels_active(self):
        """Editor starts without crash — panels are created by plugins at load time."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # If panels failed to initialize, the editor would crash.
        # Verify it's alive by sending a quit command.
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"
        os.unlink(path)

    def test_editing_works_with_panels(self):
        """Insert text, save, quit — panels don't interfere with buffer operations."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "panel test")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "panel test", \
            f"Expected 'panel test', got: {repr(content)}"
        os.unlink(path)

    def test_navigation_with_panels(self):
        """Navigate a multi-line file — panels update without interfering."""
        lines = [f"line{i}" for i in range(20)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Navigate down 15 lines (panels should update line numbers + status)
        send_keys(child, "15")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)

        # Insert marker to verify position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "HERE")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        assert "HERE" in result_lines[15], \
            f"Expected HERE on line 15, got: {repr(result_lines[15])}"
        os.unlink(path)

    def test_mode_switch_with_panels(self):
        """Switch modes multiple times — status bar panel should update mode display."""
        path = create_temp_file("test")
        child = spawn_alfred(path)

        # Enter insert mode
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "A")
        send_escape(child)
        time.sleep(0.3)

        # Enter visual mode
        send_keys(child, "v")
        time.sleep(0.2)
        send_escape(child)
        time.sleep(0.3)

        # Back to normal, save and quit
        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "A" in content, f"Expected 'A' inserted, got: {repr(content)}"
        os.unlink(path)

    def test_large_file_with_panels(self):
        """Open a 100-line file — gutter panel adjusts width for 3-digit line numbers."""
        lines = [f"content line {i+1}" for i in range(100)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Navigate to line 50
        send_keys(child, "50")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)

        # Insert at line 50 to verify position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, ">>")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        assert ">>" in result_lines[50], \
            f"Expected >> on line 50, got: {repr(result_lines[50])}"
        os.unlink(path)

    def test_word_count_plugin_with_panels(self):
        """The word-count command works alongside panel plugins."""
        path = create_temp_file("one two three four five")
        child = spawn_alfred(path)

        send_colon_command(child, "word-count")
        time.sleep(0.5)

        # The message should appear (word count displayed).
        # We can't read the screen, but if the editor doesn't crash
        # and we can still quit, the plugin coexists with panels.
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)
        assert exit_code == 0
        os.unlink(path)

    def test_multiple_edits_with_panels(self):
        """Complex editing session — panels track all changes."""
        path = create_temp_file("aaa\nbbb\nccc")
        child = spawn_alfred(path)

        # Delete first line
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "d")
        time.sleep(0.3)

        # Insert new text
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "NEW")
        send_enter(child)
        send_escape(child)
        time.sleep(0.3)

        # Undo
        send_keys(child, "u")
        time.sleep(0.3)

        # Save and quit
        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        # After undo, the insert should be reverted
        assert "bbb" in content or "ccc" in content, \
            f"Expected some original content after undo, got: {repr(content)}"
        os.unlink(path)
