---
name: UI design decisions
description: User's preferences for Alfred UI rendering — alternate screen, deferred mouse, Lisp-driven themes
type: feedback
---

UI rendering decisions locked in:

1. **Full screen**: Extend existing RawModeGuard (Option A) to handle EnterAlternateScreen/LeaveAlternateScreen. Keep RAII cleanup centralized.

2. **Mouse capture**: Deferred to later. Do not implement now.

3. **Color themes**: Everything must be driven by Alfred Lisp. No hardcoded theme structs with fixed slots — the user wants full Lisp control over theming. Go straight to Lisp primitives for defining and applying themes.

**Why:** The user wants maximum customizability via Alfred Lisp. The Lisp-first philosophy applies to UI theming, not just keybindings and plugins.

**How to apply:** When implementing the theme system, design it so Lisp code defines all color mappings. The Rust side provides primitives and a flexible key-value color store, not a rigid Theme struct with predetermined slots.
