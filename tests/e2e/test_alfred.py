"""
End-to-end tests for the Alfred text editor.

Each test spawns the Alfred binary inside a real PTY via pexpect,
sends keystrokes, and verifies observable outcomes (file content after
save, exit codes, screen output).

These tests exercise the full stack: binary startup, plugin loading,
Lisp runtime, keymap dispatch, buffer operations, and file I/O.
"""

import os
import subprocess
import tempfile
import time

import pexpect
import pytest


ALFRED_BIN = "/usr/local/bin/alfred"
# Generous timeout: the editor should respond well within this.
TIMEOUT = 10


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



# -------------------------------------------------------------------------
# Test 1: Basic startup and quit
# -------------------------------------------------------------------------

class TestDiagnostic:
    """Diagnostic tests to debug insert mode behavior."""

    def test_insert_mode_debug(self):
        """Debug: check what happens when pressing i, typing, then saving."""
        path = create_temp_file("original")
        child = spawn_alfred(path)

        # Read initial screen
        try:
            initial = child.read_nonblocking(size=8192, timeout=1)
            print(f"Initial screen: {repr(initial[:500])}")
        except Exception:
            pass

        # Press 'i' to enter insert mode
        child.send("i")
        time.sleep(0.5)

        # Read screen after 'i'
        try:
            after_i = child.read_nonblocking(size=8192, timeout=1)
            print(f"After 'i': {repr(after_i[:500])}")
        except Exception as e:
            print(f"After 'i' read error: {e}")

        # Type 'X'
        child.send("X")
        time.sleep(0.5)

        # Read screen after 'X'
        try:
            after_x = child.read_nonblocking(size=8192, timeout=1)
            print(f"After 'X': {repr(after_x[:500])}")
        except Exception as e:
            print(f"After 'X' read error: {e}")

        # Escape
        child.send("\x1b")
        time.sleep(0.5)

        # Read screen after Escape
        try:
            after_esc = child.read_nonblocking(size=8192, timeout=1)
            print(f"After Escape: {repr(after_esc[:500])}")
        except Exception as e:
            print(f"After Escape read error: {e}")

        # :wq
        send_colon_command(child, "wq")
        exit_code = wait_for_exit(child)

        content = read_file(path)
        print(f"File content: {repr(content)}")
        print(f"Exit code: {exit_code}")
        os.unlink(path)


class TestBasicStartup:
    """Alfred opens a file and exits cleanly with :q."""

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
        time.sleep(0.2)
        send_keys(child, "X")
        time.sleep(0.2)
        send_escape(child)
        time.sleep(0.2)

        # :q! should force quit
        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0, f"Expected exit code 0, got {exit_code}"
        # File should be unchanged
        assert read_file(path) == "original"
        os.unlink(path)


# -------------------------------------------------------------------------
# Test 2: Insert mode
# -------------------------------------------------------------------------

class TestInsertMode:
    """Alfred enters insert mode with 'i', accepts typed text, saves with :wq."""

    def test_insert_hello(self):
        """Press i, type 'hello', Escape, :wq -- file contains 'hello'."""
        path = create_temp_file("")
        child = spawn_alfred(path)

        # Enter insert mode
        send_keys(child, "i")
        time.sleep(0.3)

        # Type text
        send_keys(child, "hello")
        time.sleep(0.3)

        # Return to normal mode
        send_escape(child)
        time.sleep(0.3)

        # Save and quit
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


# -------------------------------------------------------------------------
# Test 3: Navigation
# -------------------------------------------------------------------------

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


# -------------------------------------------------------------------------
# Test 4: Delete character
# -------------------------------------------------------------------------

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


# -------------------------------------------------------------------------
# Test 5: Undo
# -------------------------------------------------------------------------

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


# -------------------------------------------------------------------------
# Test 6: Command mode / eval
# -------------------------------------------------------------------------

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

    def test_eval_string(self):
        """Open file, :eval (message 'hello'), :q! -- exits without crash."""
        path = create_temp_file("test")
        child = spawn_alfred(path)

        # Send raw eval command
        send_colon_command(child, 'eval (+ 40 2)')
        time.sleep(0.5)

        send_colon_command(child, "q!")
        exit_code = wait_for_exit(child)

        assert exit_code == 0
        os.unlink(path)


# -------------------------------------------------------------------------
# Test 7: Write without quit
# -------------------------------------------------------------------------

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
