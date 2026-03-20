# Opportunity Tree -- Alfred Editor

**Feature**: alfred-core
**Date**: 2026-03-19
**Phase**: 2 -- Opportunity Mapping (Complete)

---

## Desired Outcome

**Prove that AI agents can build architecturally sound, extensible software** by delivering a text editor where Vim-style modal editing works entirely as a Lisp plugin on top of a thin Rust kernel.

---

## Opportunity Tree

```
OUTCOME: Prove plugin-first editor architecture works end-to-end
|
+-- OPP-1: Reliable Rust kernel (foundation)                    [Score: 14]
|   |
|   +-- OPP-1.1: Fast, correct text storage (rope buffer)      [Score: 12]
|   +-- OPP-1.2: Responsive event loop                         [Score: 11]
|   +-- OPP-1.3: Efficient terminal rendering                  [Score: 10]
|
+-- OPP-2: Working Lisp extension layer                         [Score: 14]
|   |
|   +-- OPP-2.1: Adopt embeddable Lisp (Janet or rust_lisp)    [Score: 13]
|   +-- OPP-2.2: Expose core primitives to Lisp (FFI)          [Score: 14]
|   +-- OPP-2.3: Lisp can define commands and bind keys         [Score: 13]
|
+-- OPP-3: Plugin system that enables real features             [Score: 15]
|   |
|   +-- OPP-3.1: Plugin discovery and loading                  [Score: 12]
|   +-- OPP-3.2: Plugin lifecycle (init/cleanup)                [Score: 12]
|   +-- OPP-3.3: Plugins can compose with each other            [Score: 10]
|
+-- OPP-4: Vim keybindings as proof of architecture             [Score: 15]
|   |
|   +-- OPP-4.1: Modal editing (Normal/Insert) via plugin       [Score: 15]
|   +-- OPP-4.2: Key interception and buffer mutation via Lisp  [Score: 14]
|   +-- OPP-4.3: Mode line / status display via plugin          [Score: 11]
|
+-- OPP-5: Architectural showcase quality                       [Score: 12]
    |
    +-- OPP-5.1: Clean crate/module boundaries                 [Score: 11]
    +-- OPP-5.2: Sequential milestone validation                [Score: 10]
    +-- OPP-5.3: AI agents execute each milestone               [Score: 12]
```

---

## Opportunity Scoring

Scoring uses a simplified Opportunity Algorithm adapted for a personal project. Instead of market satisfaction/importance, the dimensions are:

- **Architectural Importance** (1-10): How critical is this to proving the architecture?
- **Current Gap** (1-10): How far is this from "exists and works"? (Higher = bigger gap = more opportunity)
- **Score** = Importance + Gap (max 20)

| Opportunity | Importance | Gap | Score | Priority |
|-------------|-----------|-----|-------|----------|
| OPP-4.1: Modal editing via plugin | 10 | 5 | 15 | TOP |
| OPP-3: Plugin system | 10 | 5 | 15 | TOP |
| OPP-2.2: Core primitives exposed to Lisp | 9 | 5 | 14 | TOP |
| OPP-1: Reliable Rust kernel | 9 | 5 | 14 | HIGH |
| OPP-2.1: Adopt embeddable Lisp | 8 | 5 | 13 | HIGH |
| OPP-2.3: Lisp can define commands/keys | 8 | 5 | 13 | HIGH |
| OPP-5.3: AI agents execute milestones | 7 | 5 | 12 | MEDIUM |
| OPP-1.1: Rope buffer | 7 | 5 | 12 | MEDIUM |
| OPP-3.1: Plugin discovery/loading | 7 | 5 | 12 | MEDIUM |
| OPP-3.2: Plugin lifecycle | 7 | 5 | 12 | MEDIUM |
| OPP-5.1: Clean crate boundaries | 6 | 5 | 11 | MEDIUM |
| OPP-1.2: Event loop | 6 | 5 | 11 | MEDIUM |
| OPP-4.3: Status display via plugin | 6 | 5 | 11 | MEDIUM |
| OPP-1.3: Terminal rendering | 5 | 5 | 10 | LOWER |
| OPP-3.3: Plugin composition | 5 | 5 | 10 | LOWER |
| OPP-5.2: Sequential validation | 5 | 5 | 10 | LOWER |

Note: All gaps are 5 (nothing exists yet -- greenfield project). Differentiation is entirely on architectural importance.

---

## Top 3 Prioritized Opportunities

### 1. OPP-4.1 + OPP-4.2: Vim modal editing as a Lisp plugin (Score: 15)
**Why top**: This is the single deliverable that proves the entire architecture. If Vim keybindings work as a Lisp plugin -- intercepting input, managing mode state, mutating buffers -- then the kernel, Lisp layer, plugin system, and API surface all work. It is the end-to-end integration test.

**Milestone mapping**: M6 (basic keybinding plugin) and M7 (Vim keybindings plugin)

### 2. OPP-3: Plugin system that enables real features (Score: 15)
**Why second**: The plugin system is the mechanism that makes OPP-4 possible. Without plugin discovery, loading, and lifecycle management, there is no plugin architecture to prove.

**Milestone mapping**: M3 (plugin loading and discovery), M4 (line numbers plugin), M5 (status bar plugin)

### 3. OPP-2.2: Core primitives exposed to Lisp (Score: 14)
**Why third**: The FFI bridge between Rust and Lisp is the critical seam. If Lisp code cannot call `buffer-insert`, `cursor-move`, `define-key`, the plugin system is inert. This is the enabler for everything above.

**Milestone mapping**: M2 (expose core primitives after Lisp adoption)

---

## Opportunity-to-Milestone Mapping

| Milestone | Opportunities Addressed | What It Proves |
|-----------|------------------------|----------------|
| M1 | OPP-1.1, OPP-1.2, OPP-1.3 | Kernel works: file display, cursor movement |
| M2 | OPP-2.1, OPP-2.2 | Lisp integrated: expressions evaluate, call Rust primitives |
| M3 | OPP-3.1, OPP-3.2 | Plugins load: discovery, init, cleanup lifecycle |
| M4 | OPP-3 (partial), OPP-5.1 | First plugin works: line numbers rendered by Lisp |
| M5 | OPP-3 (full), OPP-4.3 | Complex plugin works: status bar with state |
| M6 | OPP-2.3, OPP-4.2 | Plugins intercept input: keybinding, text editing via Lisp |
| M7 | OPP-4.1 | Architecture proven: full modal editing as a plugin |

---

## Deferred Opportunities (Post-Walking-Skeleton)

These are real opportunities but explicitly out of scope for the walking skeleton:

| Opportunity | Why Deferred |
|-------------|-------------|
| Syntax highlighting (tree-sitter) | Requires tree-sitter integration; not needed to prove plugin architecture |
| LSP integration | Large surface area; proves language tooling, not plugin architecture |
| Dynamic Rust plugins (.so/.dylib) | Platform-specific complexity; Lisp-only is sufficient for walking skeleton |
| Undo/redo | Requires Transaction system; proves editing completeness, not architecture |
| Split windows | UI complexity; proves layout, not plugin system |
| Search/replace | Feature, not architecture |
| Mouse support | Input variant, not architecture |
| Config file loading | Convenience, not architecture |

---

## Gate G2 Assessment

| Criterion | Target | Actual | Status |
|-----------|--------|--------|--------|
| Opportunities identified | 5+ | 16 sub-opportunities across 5 categories | Pass |
| Top opportunity score | >8 | 15 | Pass |
| Job step coverage | 80%+ | 100% -- every milestone maps to opportunities, every top opportunity maps to milestones | Pass |
| Team alignment | Confirmed | Solo project -- creator confirmed all priorities through interactive rounds | Pass |

**Gate G2: PASS** -- Opportunities mapped and prioritized. Proceed to Phase 3.
