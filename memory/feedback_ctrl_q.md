---
name: Ctrl-Q quit keybinding conflict
description: User dislikes Ctrl-Q as quit shortcut because it closes their terminal application (which also runs Claude Code)
type: feedback
---

Do not use Ctrl-Q as the quit keybinding for Alfred. It conflicts with the user's terminal emulator shortcut.

**Why:** Ctrl-Q closes the entire terminal application (not just Alfred), which kills the Claude Code session. The user experienced this directly.

**How to apply:** Choose a different quit keybinding for Alfred's hardcoded M1 bindings and future plugins. Consider Ctrl-C, Ctrl-X, or an Emacs-style sequence like Ctrl-X Ctrl-C. Update both the hardcoded M1 keybinding and the acceptance criteria.
