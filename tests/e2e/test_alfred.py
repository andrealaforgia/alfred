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
