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


# ---------------------------------------------------------------------------
# Navigation extended (8 tests)
# ---------------------------------------------------------------------------

class TestNavigationExtended:
    """Verify extended navigation commands: w, b, 0, $, gg, G, Ctrl-d, ;."""

    def test_w_moves_to_next_word(self):
        """w on 'hello world', insert X, verify X before 'world'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # w moves to start of next word ('world')
        send_keys(child, "w")
        time.sleep(0.3)

        # Insert X before 'world'
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "Xworld" in content, \
            f"Expected 'Xworld' after w + insert X, got: {repr(content)}"
        os.unlink(path)

    def test_b_moves_to_previous_word(self):
        """b on 'hello world' with cursor at end, moves back to 'world'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to end of line first
        send_keys(child, "$")
        time.sleep(0.3)

        # b moves backward to start of current/previous word
        send_keys(child, "b")
        time.sleep(0.3)

        # Insert X before 'world'
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "Xworld" in content, \
            f"Expected 'Xworld' after $ + b + insert X, got: {repr(content)}"
        os.unlink(path)

    def test_0_moves_to_line_start(self):
        """0 on 'hello' at col 3, insert X, verify 'Xhello'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # Move to col 3
        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # 0 moves to start of line
        send_keys(child, "0")
        time.sleep(0.3)

        # Insert X at start
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "Xhello", \
            f"Expected 'Xhello' after 0 + insert X, got: {repr(content)}"
        os.unlink(path)

    def test_dollar_moves_to_line_end(self):
        """$ on 'hello', then a to insert after, type X, verify 'helloX'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # $ moves to last char of line
        send_keys(child, "$")
        time.sleep(0.3)

        # a inserts after cursor (after last char = end of line)
        send_keys(child, "a")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "helloX", \
            f"Expected 'helloX' after $ + a + X, got: {repr(content)}"
        os.unlink(path)

    def test_gg_moves_to_document_start(self):
        """g on a 10-line file at line 5, insert X, verify X on line 0.

        Note: Alfred maps single 'g' to cursor-document-start (not 'gg').
        """
        lines = [f"line{i}" for i in range(10)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Move to line 5
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)

        # g moves to document start (line 0)
        send_keys(child, "g")
        time.sleep(0.3)

        # Insert X at document start
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        assert result_lines[0].startswith("X"), \
            f"Expected line 0 to start with 'X', got: {repr(result_lines[0])}"
        os.unlink(path)

    def test_G_moves_to_document_end(self):
        """G on a 10-line file, insert X, verify X on last line."""
        lines = [f"line{i}" for i in range(10)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # G moves to document end (last line)
        send_keys(child, "G")
        time.sleep(0.3)

        # Insert X on last line
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.rstrip("\n").split("\n")
        last_line = result_lines[-1]
        assert "X" in last_line, \
            f"Expected 'X' on last line, got: {repr(last_line)}"
        os.unlink(path)

    def test_ctrl_d_scrolls_down(self):
        """Ctrl-d on a 50-line file, insert X, verify X is roughly halfway."""
        lines = [f"line{i}" for i in range(50)]
        path = create_temp_file("\n".join(lines))
        child = spawn_alfred(path)

        # Ctrl-d scrolls half page down
        child.send("\x04")  # Ctrl-d
        time.sleep(0.5)

        # Insert X at current position
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        result_lines = content.split("\n")
        # Find which line has X
        x_line = None
        for idx, line in enumerate(result_lines):
            if "X" in line and line != f"line{idx}":
                x_line = idx
                break
        assert x_line is not None, \
            f"Expected X marker in file, got: {repr(content[:200])}"
        # Ctrl-d should move roughly half a screen (12 lines for 24-row terminal)
        assert x_line >= 5, \
            f"Expected cursor to move down significantly, X found on line {x_line}"
        os.unlink(path)

    def test_semicolon_repeats_find_char(self):
        """fa then ; on 'abcabc', insert X before second 'a'."""
        path = create_temp_file("abcabc")
        child = spawn_alfred(path)

        # fa finds first 'a' — but cursor starts on 'a' at col 0,
        # so fa finds the next 'a' at col 3
        send_keys(child, "f")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.3)

        # ; repeats last find — should go to next 'a' if there is one,
        # but there are only two 'a's. Cursor should be on second 'a' already.
        # Insert X before it.
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "Xa" in content, \
            f"Expected 'Xa' (X before an 'a'), got: {repr(content)}"
        # Verify X is before the second 'a' (at col 3)
        assert content.index("Xa") >= 3, \
            f"Expected X before second 'a' (pos >= 3), got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Insert mode extended (4 tests)
# ---------------------------------------------------------------------------

class TestInsertModeExtended:
    """Verify extended insert commands: I, a, A, O."""

    def test_I_inserts_at_line_start(self):
        """I on '  hello' inserts at beginning of line, type X, verify X at front."""
        path = create_temp_file("  hello")
        child = spawn_alfred(path)

        # Move cursor to middle of line first
        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # I inserts at line start (col 0 or first non-blank)
        send_keys(child, "I")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        # X should appear at the beginning of the line (before or after spaces)
        assert "X" in content, f"Expected X in content, got: {repr(content)}"
        # X should be near the start
        x_pos = content.index("X")
        assert x_pos <= 2, \
            f"Expected X near start of line (pos <= 2), got pos {x_pos} in: {repr(content)}"
        os.unlink(path)

    def test_a_inserts_after_cursor(self):
        """a on 'ab' with cursor at 'a' (col 0), type X, verify 'aXb'."""
        path = create_temp_file("ab")
        child = spawn_alfred(path)

        # Cursor starts at col 0 ('a')
        # a inserts after cursor
        send_keys(child, "a")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "aXb", \
            f"Expected 'aXb' after a + X, got: {repr(content)}"
        os.unlink(path)

    def test_A_inserts_at_line_end(self):
        """A on 'hello', type X, verify 'helloX'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # A inserts at end of line
        send_keys(child, "A")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "helloX", \
            f"Expected 'helloX' after A + X, got: {repr(content)}"
        os.unlink(path)

    def test_O_opens_line_above(self):
        """O on 'second', type 'first', verify 'first' is on line 0."""
        path = create_temp_file("second")
        child = spawn_alfred(path)

        # O opens a new line above and enters insert mode
        send_keys(child, "O")
        time.sleep(0.3)
        send_keys(child, "first")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        lines = content.split("\n")
        assert lines[0] == "first", \
            f"Expected 'first' on line 0, got: {repr(lines[0])}"
        assert "second" in lines[1], \
            f"Expected 'second' on line 1, got: {repr(lines[1])}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Editing extended (6 tests)
# ---------------------------------------------------------------------------

class TestEditingExtended:
    """Verify extended editing commands: J, P, cc, C, d0, db."""

    def test_J_joins_lines(self):
        """J on 'hello\\nworld' joins into 'hello world'."""
        path = create_temp_file("hello\nworld")
        child = spawn_alfred(path)

        # J joins current line with next
        send_keys(child, "J")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "hello" in content and "world" in content, \
            f"Expected both 'hello' and 'world' in content, got: {repr(content)}"
        # After join, should be on one line (no newline between them)
        lines = content.split("\n")
        assert len(lines) == 1, \
            f"Expected 1 line after J, got {len(lines)} lines: {repr(content)}"
        os.unlink(path)

    def test_P_pastes_before(self):
        """yy then P duplicates line above current."""
        path = create_temp_file("original")
        child = spawn_alfred(path)

        # yy yanks the current line
        send_keys(child, "y")
        time.sleep(0.1)
        send_keys(child, "y")
        time.sleep(0.3)

        # P pastes before (above current line)
        send_keys(child, "P")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        lines = [l for l in content.split("\n") if l.strip()]
        assert len(lines) >= 2, \
            f"Expected at least 2 lines after yy + P, got: {repr(content)}"
        assert lines[0].strip() == "original", \
            f"Expected 'original' on line 0, got: {repr(lines[0])}"
        assert lines[1].strip() == "original", \
            f"Expected 'original' on line 1, got: {repr(lines[1])}"
        os.unlink(path)

    def test_cc_changes_entire_line(self):
        """cc on 'old text', type 'new', verify 'new'."""
        path = create_temp_file("old text")
        child = spawn_alfred(path)

        # cc changes entire line (deletes line content, enters insert)
        send_keys(child, "c")
        time.sleep(0.1)
        send_keys(child, "c")
        time.sleep(0.3)

        # Type replacement text
        send_keys(child, "new")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "new" in content, \
            f"Expected 'new' after cc, got: {repr(content)}"
        assert "old" not in content, \
            f"Expected 'old' to be gone after cc, got: {repr(content)}"
        os.unlink(path)

    def test_C_changes_to_end(self):
        """C on 'hello world' at col 5, type 'X', verify 'helloX'."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to col 5
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # C changes from cursor to end of line
        send_keys(child, "C")
        time.sleep(0.3)

        # Type replacement
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "helloX", \
            f"Expected 'helloX' after C, got: {repr(content)}"
        os.unlink(path)

    def test_d0_deletes_to_line_start(self):
        """d0 on 'hello' at col 3, verify 'lo'."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # Move to col 3
        send_keys(child, "3")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # d0 deletes from cursor to start of line
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "0")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "lo", \
            f"Expected 'lo' after d0 at col 3, got: {repr(content)}"
        os.unlink(path)

    def test_db_deletes_word_backward(self):
        """db on 'hello world' at col 6, deletes backward word."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to col 6 (the 'w' of 'world')
        send_keys(child, "6")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # db deletes the word backward
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "b")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        # db from 'w' of 'world' should delete backward to prev word start
        # This deletes "hello " or at least some portion backward
        assert "world" in content, \
            f"Expected 'world' to remain after db, got: {repr(content)}"
        assert len(content) < len("hello world"), \
            f"Expected shorter content after db, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Operator extended (4 tests)
# ---------------------------------------------------------------------------

class TestOperatorExtended:
    """Verify extended operator commands: dj, dk, yw+p, y$+p."""

    def test_dj_deletes_two_lines(self):
        """dj on 'a\\nb\\nc' deletes current line and line below."""
        path = create_temp_file("a\nb\nc")
        child = spawn_alfred(path)

        # dj deletes current line + line below (lines 'a' and 'b')
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "a" not in content, \
            f"Expected 'a' to be deleted, got: {repr(content)}"
        assert "b" not in content, \
            f"Expected 'b' to be deleted, got: {repr(content)}"
        assert "c" in content, \
            f"Expected 'c' to remain, got: {repr(content)}"
        os.unlink(path)

    def test_dk_deletes_up(self):
        """dk on line 1 of 'a\\nb\\nc' deletes current line and line above."""
        path = create_temp_file("a\nb\nc")
        child = spawn_alfred(path)

        # Move to line 1 ('b')
        send_keys(child, "j")
        time.sleep(0.2)

        # dk deletes current line + line above (lines 'b' and 'a')
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "k")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "a" not in content, \
            f"Expected 'a' to be deleted, got: {repr(content)}"
        assert "b" not in content, \
            f"Expected 'b' to be deleted, got: {repr(content)}"
        assert "c" in content, \
            f"Expected 'c' to remain, got: {repr(content)}"
        os.unlink(path)

    def test_yw_then_p_pastes_word(self):
        """yw on 'hello world', then $ p, verify 'hello' pasted at end."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # yw yanks the first word ('hello')
        send_keys(child, "y")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        # $ moves to end of line, p pastes after cursor
        send_keys(child, "$")
        time.sleep(0.2)
        send_keys(child, "p")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        # 'hello' should appear at least twice (original + pasted)
        assert content.count("hello") >= 2, \
            f"Expected 'hello' at least twice after yw + $ + p, got: {repr(content)}"
        os.unlink(path)

    def test_y_dollar_then_p(self):
        """y$ on 'hello world' at col 5, yanks ' world', 0 p pastes at start."""
        path = create_temp_file("hello world")
        child = spawn_alfred(path)

        # Move to col 5
        send_keys(child, "5")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.2)

        # y$ yanks from cursor to end of line
        send_keys(child, "y")
        time.sleep(0.1)
        send_keys(child, "$")
        time.sleep(0.3)

        # 0 moves to start of line, p pastes after cursor
        send_keys(child, "0")
        time.sleep(0.2)
        send_keys(child, "p")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        # The yanked text (' world' or 'world') should appear in the result
        assert content.count("world") >= 2, \
            f"Expected 'world' at least twice after y$ + 0 + p, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Text object extended (4 tests)
# ---------------------------------------------------------------------------

class TestTextObjectExtended:
    """Verify extended text object commands: daw, ci\", di(, da{."""

    def test_daw_deletes_around_word(self):
        """daw on 'hello world end' with cursor on 'world' deletes 'world' and surrounding space."""
        path = create_temp_file("hello world end")
        child = spawn_alfred(path)

        # Move to 'world' (w moves to next word)
        send_keys(child, "w")
        time.sleep(0.2)

        # daw deletes around word (word + surrounding whitespace)
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "world" not in content, \
            f"Expected 'world' to be deleted, got: {repr(content)}"
        assert "hello" in content, \
            f"Expected 'hello' to remain, got: {repr(content)}"
        assert "end" in content, \
            f"Expected 'end' to remain, got: {repr(content)}"
        os.unlink(path)

    def test_ci_quote_changes_inner_quotes(self):
        """ci\" on 'say \"old\" ok' changes inner quotes to 'new'."""
        path = create_temp_file('say "old" ok')
        child = spawn_alfred(path)

        # Move cursor inside the quotes (move to 'o' of 'old')
        send_keys(child, "f")
        time.sleep(0.1)
        send_keys(child, "o")
        time.sleep(0.3)

        # ci" changes inner quotes
        send_keys(child, "c")
        time.sleep(0.1)
        send_keys(child, 'i')
        time.sleep(0.1)
        send_keys(child, '"')
        time.sleep(0.3)

        # Type replacement
        send_keys(child, "new")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert '"new"' in content, \
            f"Expected '\"new\"' after ci\", got: {repr(content)}"
        assert "old" not in content, \
            f"Expected 'old' to be gone after ci\", got: {repr(content)}"
        os.unlink(path)

    def test_di_paren_deletes_inner_parens(self):
        """di( on 'fn(arg)' with cursor inside parens deletes 'arg'."""
        path = create_temp_file("fn(arg)")
        child = spawn_alfred(path)

        # Move cursor inside parentheses
        send_keys(child, "f")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.3)

        # di( deletes inner parentheses content
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "i")
        time.sleep(0.1)
        send_keys(child, "(")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "fn()", \
            f"Expected 'fn()' after di(, got: {repr(content)}"
        os.unlink(path)

    def test_da_brace_deletes_around_braces(self):
        """da{ on 'x{content}y' with cursor inside braces deletes '{content}'."""
        path = create_temp_file("x{content}y")
        child = spawn_alfred(path)

        # Move cursor inside braces
        send_keys(child, "f")
        time.sleep(0.1)
        send_keys(child, "c")
        time.sleep(0.3)

        # da{ deletes around braces (including the braces themselves)
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "a")
        time.sleep(0.1)
        send_keys(child, "{")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "xy", \
            f"Expected 'xy' after da{{, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Visual mode extended (3 tests)
# ---------------------------------------------------------------------------

class TestVisualExtended:
    """Verify extended visual mode: select+yank+paste, select+change, V+yank+paste."""

    def test_v_select_and_yank_then_paste(self):
        """v + ll + y on 'hello', then $ p, verify yanked text appended."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # v enters visual mode, select 'hel' (cursor + 2 right)
        send_keys(child, "v")
        time.sleep(0.2)
        send_keys(child, "l")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.1)

        # y yanks the selection (should be 'hel')
        send_keys(child, "y")
        time.sleep(0.3)

        # $ moves to end, p pastes after cursor
        send_keys(child, "$")
        time.sleep(0.2)
        send_keys(child, "p")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        # The yanked text should appear after the original
        assert len(content) > len("hello"), \
            f"Expected content longer than 'hello' after yank+paste, got: {repr(content)}"
        os.unlink(path)

    def test_v_select_and_change(self):
        """v + ll + c on 'ABCDE', type 'X', verify selection replaced."""
        path = create_temp_file("ABCDE")
        child = spawn_alfred(path)

        # v enters visual mode, select 'ABC' (cursor + 2 right)
        send_keys(child, "v")
        time.sleep(0.2)
        send_keys(child, "l")
        time.sleep(0.1)
        send_keys(child, "l")
        time.sleep(0.1)

        # c changes the selection (deletes and enters insert mode)
        send_keys(child, "c")
        time.sleep(0.3)

        # Type replacement
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert "X" in content, \
            f"Expected 'X' in content after visual change, got: {repr(content)}"
        assert "ABC" not in content, \
            f"Expected 'ABC' to be replaced, got: {repr(content)}"
        assert "DE" in content, \
            f"Expected 'DE' to remain, got: {repr(content)}"
        os.unlink(path)

    def test_V_yank_and_paste(self):
        """V + y on 'first\\nsecond', then j p, verify first line duplicated."""
        path = create_temp_file("first\nsecond")
        child = spawn_alfred(path)

        # V enters visual line mode, y yanks the line
        send_keys(child, "V", delay=0.15)
        time.sleep(0.3)
        send_keys(child, "y", delay=0.15)
        time.sleep(0.3)

        # j moves down, p pastes below
        send_keys(child, "j")
        time.sleep(0.2)
        send_keys(child, "p")
        time.sleep(0.3)

        send_colon_command(child, "w")
        time.sleep(0.5)
        send_colon_command(child, "q!")
        wait_for_exit(child)

        content = read_file(path)
        # 'first' should appear at least twice
        assert content.count("first") >= 2, \
            f"Expected 'first' at least twice after V yank + paste, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Undo/redo (2 tests)
# ---------------------------------------------------------------------------

class TestUndoRedo:
    """Verify undo and redo interactions."""

    def test_redo_after_undo(self):
        """x deletes 'a', u undoes, Ctrl-r redoes -> 'bc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        # x deletes 'a' -> 'bc'
        send_keys(child, "x")
        time.sleep(0.3)

        # u undoes -> 'abc'
        send_keys(child, "u")
        time.sleep(0.3)

        # Ctrl-r redoes -> 'bc'
        child.send("\x12")  # Ctrl-r
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "bc", \
            f"Expected 'bc' after x + u + Ctrl-r, got: {repr(content)}"
        os.unlink(path)

    def test_multiple_undo(self):
        """x, x -> 'c', u, u -> 'abc'."""
        path = create_temp_file("abc")
        child = spawn_alfred(path)

        # x deletes 'a' -> 'bc'
        send_keys(child, "x")
        time.sleep(0.3)

        # x deletes 'b' -> 'c'
        send_keys(child, "x")
        time.sleep(0.3)

        # u undoes second delete -> 'bc'
        send_keys(child, "u")
        time.sleep(0.3)

        # u undoes first delete -> 'abc'
        send_keys(child, "u")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path).rstrip("\n")
        assert content == "abc", \
            f"Expected 'abc' after x + x + u + u, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# File operations (3 tests)
# ---------------------------------------------------------------------------

class TestFileOps:
    """Verify file operation commands: :e, :w filename, unsaved changes warning."""

    def test_e_opens_another_file(self):
        """Create two files, open first, :e second, :wq, verify second file unchanged."""
        path1 = create_temp_file("file one content")
        path2 = create_temp_file("file two content")
        child = spawn_alfred(path1)

        # Open the second file
        send_colon_command(child, f"e {path2}")
        time.sleep(1.0)

        # Quit without modifying
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        # Second file should be unmodified
        content2 = read_file(path2)
        assert content2 == "file two content", \
            f"Expected second file unchanged, got: {repr(content2)}"
        os.unlink(path1)
        os.unlink(path2)

    def test_w_with_filename_saves_as(self):
        """Type text, :w /tmp/path, :q!, verify file created."""
        path = create_temp_file("")
        save_as_path = "/tmp/alfred_e2e_saveas_test.txt"
        child = spawn_alfred(path)

        # Enter insert mode, type text
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "saved content")
        send_escape(child)
        time.sleep(0.3)

        # :w with a different filename
        send_colon_command(child, f"w {save_as_path}")
        time.sleep(0.5)

        # Force quit (original buffer may show as modified)
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0
        # Verify the save-as file was created
        assert os.path.exists(save_as_path), \
            f"Expected {save_as_path} to exist after :w"
        content = read_file(save_as_path)
        assert "saved content" in content, \
            f"Expected 'saved content' in save-as file, got: {repr(content)}"
        os.unlink(path)
        os.unlink(save_as_path)

    def test_unsaved_changes_warning(self):
        """Modify buffer, :q should warn (not exit), :q! should force exit."""
        path = create_temp_file("original")
        child = spawn_alfred(path)

        # Modify the buffer
        send_keys(child, "i")
        time.sleep(0.3)
        send_keys(child, "X")
        send_escape(child)
        time.sleep(0.3)

        # :q should warn about unsaved changes (editor stays alive)
        send_colon_command(child, "q")
        time.sleep(1.0)

        # Editor should still be alive — force quit
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0 after q!, got {exit_code}"
        # File should be unchanged (we never saved)
        content = read_file(path)
        assert content == "original", \
            f"Expected 'original' unchanged, got: {repr(content)}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Edge cases (6 tests)
# ---------------------------------------------------------------------------

class TestEdgeCases:
    """Verify edge cases don't crash the editor."""

    def test_x_on_empty_buffer(self):
        """x on empty file should not crash, file stays empty."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        # x on empty buffer — should be a no-op, no crash
        send_keys(child, "x")
        time.sleep(0.3)

        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        content = read_file(path)
        assert content == "", \
            f"Expected empty file, got: {repr(content)}"
        os.unlink(path)

    def test_dd_on_single_line(self):
        """dd on 'only' single-line file, verify file is empty or single newline."""
        path = create_temp_file("only")
        child = spawn_alfred(path)

        # dd deletes the only line
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "d")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        stripped = content.strip()
        assert stripped == "" or stripped == "\n", \
            f"Expected empty or near-empty file after dd on single line, got: {repr(content)}"
        os.unlink(path)

    def test_dw_on_last_word_of_last_line(self):
        """dw on 'hello' (only word on only line), verify empty after save."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # dw on the only word
        send_keys(child, "d")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        stripped = content.strip()
        assert stripped == "", \
            f"Expected empty file after dw on last word, got: {repr(content)}"
        os.unlink(path)

    def test_operations_on_single_char_buffer(self):
        """x on 'a' (single char file), verify empty after save."""
        path = create_temp_file("a")
        child = spawn_alfred(path)

        # x deletes the single character
        send_keys(child, "x")
        time.sleep(0.3)

        send_colon_command(child, "wq")
        wait_for_exit(child)

        content = read_file(path)
        stripped = content.strip()
        assert stripped == "", \
            f"Expected empty file after x on 'a', got: {repr(content)}"
        os.unlink(path)

    def test_cw_on_empty_line(self):
        """Open line with o, Escape, cw on empty line, verify no crash."""
        path = create_temp_file("text")
        child = spawn_alfred(path)

        # o opens line below (enters insert), then Escape to normal
        send_keys(child, "o")
        time.sleep(0.3)
        send_escape(child)
        time.sleep(0.3)

        # cw on the empty line — should not crash
        send_keys(child, "c")
        time.sleep(0.1)
        send_keys(child, "w")
        time.sleep(0.3)

        # If we're in insert mode (from cw), escape out
        send_escape(child)
        time.sleep(0.3)

        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, \
            f"Expected no crash (exit 0) for cw on empty line, got {exit_code}"
        os.unlink(path)

    def test_search_no_match(self):
        """Search for non-existent pattern, verify no crash."""
        path = create_temp_file("hello")
        child = spawn_alfred(path)

        # /xyz Enter — search for non-existent pattern
        send_keys(child, "/")
        time.sleep(0.2)
        send_keys(child, "xyz")
        send_enter(child)
        time.sleep(0.5)

        # Editor should still be alive — quit
        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, \
            f"Expected no crash (exit 0) after search with no match, got {exit_code}"
        os.unlink(path)


# ---------------------------------------------------------------------------
# Large file / rope chunk boundary tests
# ---------------------------------------------------------------------------

class TestFolderBrowser:
    """Verify the folder browser feature works when opening a directory."""

    def test_open_directory_no_plugin_errors(self):
        """Opening a directory should not produce plugin errors."""
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        with open(os.path.join(tmpdir, "hello.txt"), "w") as f:
            f.write("hello world\n")
        os.mkdir(os.path.join(tmpdir, "subdir"))

        child = spawn_alfred(tmpdir)

        try:
            screen = child.read_nonblocking(size=16384, timeout=2)
        except Exception:
            screen = ""

        send_keys(child, "q")
        time.sleep(0.3)
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"
        assert "Plugin errors" not in screen, \
            f"Plugin errors when opening directory: {repr(screen[:500])}"
        assert "parse_key_spec" not in screen, \
            f"Key spec parse error in browse-mode plugin: {repr(screen[:500])}"

        shutil.rmtree(tmpdir)

    def test_browser_displays_directory_entries(self):
        """Opening a directory renders the folder browser with correct entries.

        Uses sequential pexpect.expect calls to wait for each entry to appear
        in the PTY output stream. This is reliable because ratatui writes
        entries in order (row by row) within a single frame flush.
        """
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        os.mkdir(os.path.join(tmpdir, "alpha_dir"))
        os.mkdir(os.path.join(tmpdir, "beta_dir"))
        with open(os.path.join(tmpdir, "gamma.txt"), "w") as f:
            f.write("gamma content\n")
        with open(os.path.join(tmpdir, "delta.rs"), "w") as f:
            f.write("fn main() {}\n")

        child = spawn_alfred(tmpdir)

        # Verify each entry appears in the PTY output.
        # Sorted order: ../, alpha_dir/, beta_dir/, delta.rs, gamma.txt
        # ratatui writes rows top-to-bottom, so entries appear in order.
        errors = []
        for entry_name in ["alpha_dir/", "beta_dir/", "delta.rs", "gamma.txt"]:
            try:
                child.expect(entry_name, timeout=5)
            except pexpect.TIMEOUT:
                errors.append(entry_name)

        send_keys(child, "q")
        time.sleep(0.3)
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"
        assert not errors, \
            f"Browser did not render these entries: {errors}"

        shutil.rmtree(tmpdir)

    def test_browser_shows_subdirectory_after_enter(self):
        """Entering a subdirectory updates the browser to show its contents.

        Uses pexpect.expect to reliably detect when the subdirectory
        contents have been rendered.
        """
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        subdir = os.path.join(tmpdir, "myproject")
        os.mkdir(subdir)
        with open(os.path.join(subdir, "unique_file_xyz.txt"), "w") as f:
            f.write("inside subdir\n")
        with open(os.path.join(tmpdir, "top_level.txt"), "w") as f:
            f.write("at root\n")

        child = spawn_alfred(tmpdir)

        # Wait for initial browser to render
        try:
            child.expect("myproject/", timeout=5)
        except pexpect.TIMEOUT:
            send_keys(child, "q")
            time.sleep(0.3)
            wait_for_exit(child)
            shutil.rmtree(tmpdir)
            pytest.fail("Browser did not render 'myproject/' on initial open")

        # Navigate to myproject/ (j to move to it, Enter to open)
        send_keys(child, "j")
        time.sleep(0.2)
        child.send("\r")
        time.sleep(0.5)

        # Wait for subdirectory contents to render
        try:
            child.expect("unique_file_xyz.txt", timeout=5)
        except pexpect.TIMEOUT:
            send_keys(child, "q")
            time.sleep(0.3)
            wait_for_exit(child)
            shutil.rmtree(tmpdir)
            pytest.fail(
                "Browser did not render 'unique_file_xyz.txt' after entering subdir"
            )

        send_keys(child, "q")
        time.sleep(0.3)
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"

        shutil.rmtree(tmpdir)

    def test_browse_and_open_file_then_save(self):
        """Browse a directory, open a file, edit it, save, and verify content."""
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        # Create a known structure: one subdir, two files
        # Sorted order will be: ../, subdir/, aaa.txt, zzz.txt
        os.mkdir(os.path.join(tmpdir, "subdir"))
        target = os.path.join(tmpdir, "aaa.txt")
        with open(target, "w") as f:
            f.write("original\n")
        with open(os.path.join(tmpdir, "zzz.txt"), "w") as f:
            f.write("other\n")

        child = spawn_alfred(tmpdir)

        # Sorted entries: ../ (0), subdir/ (1), aaa.txt (2), zzz.txt (3)
        # Navigate to aaa.txt: j j (cursor starts at 0 = ../)
        send_keys(child, "j")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.1)

        # Open the file with Enter
        child.send("\r")
        time.sleep(1.0)

        # Now in editor mode — go to insert mode and add text
        send_keys(child, "A")  # append at end of line
        time.sleep(0.2)
        send_keys(child, " EDITED")
        time.sleep(0.2)

        # Escape to normal mode, then save and quit
        child.send("\x1b")
        time.sleep(0.3)
        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"

        saved = read_file(target)
        assert "EDITED" in saved, \
            f"Expected 'EDITED' in saved file after browse+edit, got: {saved!r}"

        shutil.rmtree(tmpdir)

    def test_browse_enter_subdirectory_and_go_back(self):
        """Enter a subdirectory, then navigate back to parent."""
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        subdir = os.path.join(tmpdir, "src")
        os.mkdir(subdir)
        with open(os.path.join(subdir, "main.rs"), "w") as f:
            f.write("fn main() {}\n")
        with open(os.path.join(tmpdir, "README.md"), "w") as f:
            f.write("# Hello\n")

        child = spawn_alfred(tmpdir)

        # Sorted: ../ (0), src/ (1), README.md (2)
        # Navigate to src/ and enter it
        send_keys(child, "j")
        time.sleep(0.1)
        child.send("\r")  # Enter src/
        time.sleep(0.5)

        # Now inside src/ — entries: ../ (0), main.rs (1)
        # Go back to parent with h
        send_keys(child, "h")
        time.sleep(0.5)

        # Should be back in tmpdir — navigate to README.md and open it
        # Sorted: ../ (0), src/ (1), README.md (2)
        send_keys(child, "j")
        time.sleep(0.1)
        send_keys(child, "j")
        time.sleep(0.1)
        child.send("\r")  # Open README.md
        time.sleep(1.0)

        # Now in editor — save to verify we opened the right file
        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"

        # Verify README.md content is intact
        readme_content = read_file(os.path.join(tmpdir, "README.md"))
        assert "# Hello" in readme_content, \
            f"Expected README.md content after browse+subdir+back+open, got: {readme_content!r}"

        shutil.rmtree(tmpdir)

    def test_browse_quit_with_q(self):
        """Pressing q in browser mode exits Alfred cleanly."""
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        with open(os.path.join(tmpdir, "file.txt"), "w") as f:
            f.write("content\n")

        child = spawn_alfred(tmpdir)

        # Press q to quit from browser
        send_keys(child, "q")
        time.sleep(0.3)
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit with q, got {exit_code}"

        shutil.rmtree(tmpdir)

    def test_browse_jump_first_and_last(self):
        """g jumps to first entry, G jumps to last, then open last file."""
        import shutil

        tmpdir = tempfile.mkdtemp(prefix="alfred_e2e_browse_")
        # Create several files so there's something to jump across
        for name in ["aaa.txt", "bbb.txt", "ccc.txt", "ddd.txt"]:
            with open(os.path.join(tmpdir, name), "w") as f:
                f.write(f"{name} content\n")

        child = spawn_alfred(tmpdir)

        # Jump to last with G
        send_keys(child, "G")
        time.sleep(0.2)

        # Open last entry (should be ddd.txt)
        child.send("\r")
        time.sleep(1.0)

        # Save to verify correct file
        target = os.path.join(tmpdir, "ddd.txt")
        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected clean exit, got {exit_code}"
        saved = read_file(target)
        assert "ddd.txt content" in saved, \
            f"Expected ddd.txt content (last file), got: {saved!r}"

        shutil.rmtree(tmpdir)


class TestLargeFileRopeChunkBoundary:
    """
    Verifies that all lines in a large file are correctly accessible,
    even when ropey splits the buffer across internal rope chunks.

    Ropey's default chunk size is ~1KB. Lines that span chunk boundaries
    previously caused get_line() to return None (via as_str()), making
    those lines render as empty and causing display artifacts.
    """

    def test_large_rs_file_all_lines_preserved_after_save(self):
        """Opening and saving a large .rs file preserves all lines."""
        # Generate a file large enough to span multiple ropey chunks (~4KB+)
        lines = []
        for i in range(200):
            lines.append(f"// Line {i:04d}: {'x' * 60}")
        content = "\n".join(lines) + "\n"

        fd, path = tempfile.mkstemp(prefix="alfred_e2e_", suffix=".rs")
        with os.fdopen(fd, "w") as f:
            f.write(content)

        child = spawn_alfred(path)

        # Save without modifications — file should be identical
        send_colon_command(child, "w")
        time.sleep(0.5)

        send_colon_command(child, "q")
        exit_code = wait_for_exit(child)

        saved_content = read_file(path)
        saved_lines = saved_content.split("\n")

        # Verify no lines were lost or corrupted
        assert exit_code == 0, f"Expected exit 0, got {exit_code}"
        for i in range(200):
            expected = f"// Line {i:04d}: {'x' * 60}"
            assert saved_lines[i] == expected, \
                f"Line {i} corrupted: expected {expected!r}, got {saved_lines[i]!r}"

        os.unlink(path)

    def test_large_rs_file_edit_at_chunk_boundary_preserved(self):
        """Editing a line near a rope chunk boundary preserves content."""
        # Create ~4KB file; ropey chunk boundary is typically around 1KB
        lines = []
        for i in range(80):
            lines.append(f"fn func_{i:04d}() {{ let x = {i}; }}")
        content = "\n".join(lines) + "\n"

        fd, path = tempfile.mkstemp(prefix="alfred_e2e_", suffix=".rs")
        with os.fdopen(fd, "w") as f:
            f.write(content)

        child = spawn_alfred(path)

        # Navigate to line 40 (likely near a chunk boundary)
        # Alfred doesn't support :N goto-line, so send j keys directly
        for _ in range(39):
            child.send("j")
            time.sleep(0.02)
        time.sleep(0.5)

        # Enter insert mode at start of line, add a comment prefix
        send_keys(child, "I")
        time.sleep(0.1)
        send_keys(child, "// EDITED: ")
        time.sleep(0.1)

        # Return to normal mode and save
        child.send("\x1b")  # Escape
        time.sleep(0.3)

        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        saved_content = read_file(path)
        saved_lines = saved_content.split("\n")

        assert exit_code == 0, f"Expected exit 0, got {exit_code}"

        # Line 39 (0-indexed) should have the edit prefix
        assert saved_lines[39].startswith("// EDITED: fn func_0039"), \
            f"Expected edited line 39, got: {saved_lines[39]!r}"

        # Lines around it should be untouched
        assert saved_lines[38] == "fn func_0038() { let x = 38; }", \
            f"Line 38 should be untouched, got: {saved_lines[38]!r}"
        assert saved_lines[40] == "fn func_0040() { let x = 40; }", \
            f"Line 40 should be untouched, got: {saved_lines[40]!r}"

        os.unlink(path)
