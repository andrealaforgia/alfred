# Lean Canvas -- Alfred Editor

**Feature**: alfred-core
**Date**: 2026-03-19
**Phase**: 4 -- Market Viability (Complete)

---

This canvas is adapted for a personal technical project. Commercial fields (revenue streams, channels, cost structure) are replaced with fields relevant to a showcase project.

---

## 1. Problem

### Top 3 Problems
1. **AI-generated code lacks architectural quality**: Most AI coding demos produce working but poorly structured code. There is no compelling public example of AI agents building a modular, extensible system with clean architecture.

2. **Editor extensibility is hard to prove**: Building an editor that is genuinely extensible (not just configurable) requires proving that non-trivial features like modal editing can live entirely in the extension layer. Most "extensible" editors still hardcode core behavior.

3. **nWave framework needs a credible showcase**: The framework needs a project complex enough to demonstrate agent capabilities (multi-crate Rust, FFI, plugin systems) but scoped enough to complete.

### Existing Alternatives
- AI coding demos (typically CRUD apps, too simple to showcase architecture)
- Existing editors (Helix has no plugin system; Neovim inherited Vim's C core)
- Toy Lisp interpreters (prove language design but not system integration)

---

## 2. Solution

### Core Solution
A text editor with a thin Rust kernel and an adopted Lisp as the sole extension mechanism, where Vim-style modal editing works entirely as a Lisp plugin.

### Key Differentiators
- **Plugin-first by design**: Not "editor with plugins added later" but "kernel designed so everything is a plugin"
- **Adopt, don't build**: Use proven components (ropey, crossterm, ratatui, existing Lisp) -- the innovation is in composition, not in reinventing each part
- **AI-agent-built**: Each milestone executed by AI agents, proving the framework
- **7-milestone sequential validation**: Each milestone proves a specific architectural claim before proceeding

---

## 3. Key Metrics

Since this is a showcase project, metrics focus on architectural proof and execution quality, not users or revenue.

### Architecture Proof Metrics
| Metric | Target | How Measured |
|--------|--------|-------------|
| Plugin-to-kernel ratio | >60% of behavior in Lisp plugins | Lines of Lisp vs lines of Rust for user-facing features |
| Milestone completion rate | 7/7 milestones pass | Each milestone's "what it proves" statement verified |
| Plugin independence | Plugins work without modifying kernel | Remove a plugin, kernel still runs. Add a plugin, no Rust changes |
| API surface adequacy | Vim keybindings work with <15 core primitives | Count primitives used by vim-keybindings plugin |

### Execution Quality Metrics
| Metric | Target | How Measured |
|--------|--------|-------------|
| Test coverage on kernel | >80% | Rust test coverage tools |
| Zero kernel panics under normal use | 0 panics | Run standard editing workflows |
| Milestone cycle time | <2 weeks per milestone | Calendar tracking |
| AI agent autonomy | Agents complete milestones with <5 human interventions each | Intervention count per milestone |

---

## 4. Risks

All four risk categories assessed:

### Value Risk: Does anyone care?
| Assessment | Detail |
|-----------|--------|
| Risk level | Low |
| Rationale | Primary audience is the creator and technical peers. Value is in the learning and the showcase, not in adoption. The project succeeds if it demonstrates AI agent capabilities, regardless of external interest |
| Mitigation | Not needed -- intrinsic value |

### Usability Risk: Can someone use it?
| Assessment | Detail |
|-----------|--------|
| Risk level | Medium |
| Rationale | The editor must be usable enough to demo. If Vim keybindings feel broken or laggy, the showcase fails |
| Mitigation | M7 completion criteria include functional modal editing. Performance profiling at M2 (Lisp evaluation latency) |

### Feasibility Risk: Can we build it?
| Assessment | Detail |
|-----------|--------|
| Risk level | Medium |
| Key risks | (1) Lisp-to-Rust FFI performance, (2) Plugin API surface adequacy, (3) Key sequence timeout handling complexity |
| Mitigation | (1) Profile at M2, both Janet and rust_lisp candidates are designed for embedding. (2) Iterative API design across M4-M7 -- each plugin reveals missing primitives. (3) Start simple, add complexity only at M7 |
| Evidence | Every component has production precedent: ropey (Helix), crossterm/ratatui (multiple TUI apps), Janet (many embedded uses), evil-mode (Vim-in-Emacs as a package) |

### Viability Risk: Should we build it?
| Assessment | Detail |
|-----------|--------|
| Risk level | Low |
| Rationale | Personal project. No business model dependency. The cost is time. The return is learning + portfolio + framework showcase |
| Constraint | Must complete within a reasonable timeframe (target: ~14 weeks for M1-M7). If a milestone takes >3 weeks, reassess scope |

---

## 5. Unfair Advantage

| Advantage | Why It Matters |
|-----------|---------------|
| nWave AI agentic framework | The editor is built BY the framework, not just WITH AI. The process is the product |
| Adopt-don't-build philosophy | By using ropey, crossterm, ratatui, and an existing Lisp, effort concentrates on architecture and integration -- the hard parts |
| Research-backed decisions | 48-source research document covering 7 editor architectures. Every major decision has evidence, not opinion |
| Sequential milestone discipline | Each milestone is validated before the next begins. No speculative multi-milestone work. No scope creep |

---

## 6. Key Activities

### Walking Skeleton (M1-M7)

| Phase | Milestone | Key Activity | Duration Target |
|-------|-----------|-------------|----------------|
| Kernel | M1 | Event loop + rope buffer + rendering + cursor movement | 2 weeks |
| Lisp | M2 | Adopt Lisp, expose core primitives, verify FFI | 2 weeks |
| Plugins | M3 | Plugin discovery, loading, lifecycle management | 2 weeks |
| Proof 1 | M4 | Line numbers plugin (first end-to-end plugin) | 1 week |
| Proof 2 | M5 | Status bar plugin (rendering/state composition) | 1 week |
| Proof 3 | M6 | Basic keybinding plugin (input interception, buffer mutation) | 2 weeks |
| Proof 4 | M7 | Vim keybindings plugin (modal editing, full architecture proof) | 2-3 weeks |

**Total estimated duration**: ~12-14 weeks

### Post-Walking-Skeleton (Deferred)
- Dynamic Rust plugin support (.so/.dylib)
- Undo/redo (Transaction system)
- Syntax highlighting (tree-sitter integration)
- LSP integration
- Split windows, search/replace, mouse support

---

## 7. Go/No-Go Decision

### Go Criteria (All Must Be True)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Problem validated: clear purpose and audience | PASS -- showcase nWave framework to technical audience |
| 2 | Architecture validated against production evidence | PASS -- 7 editor case studies, 48 sources |
| 3 | Every technology choice has production precedent | PASS -- ropey (Helix), crossterm/ratatui (ecosystem), Janet/rust_lisp (embedding) |
| 4 | Scope is bounded with clear deferrals | PASS -- M1-M7 defined, everything else explicitly deferred |
| 5 | Risks identified with mitigations | PASS -- all 4 risk categories assessed |
| 6 | Milestone sequence is sequential and validated | PASS -- each milestone has "what it proves" and completion criteria |

### No-Go / Kill Signals
- If M2 (Lisp integration) reveals that neither Janet nor rust_lisp can call Rust functions with <1ms latency per call, the architecture is at risk. Reassess at M2 gate.
- If M4 (first plugin) requires >5 new Rust kernel changes to work, the API design is wrong. Reassess plugin API approach.
- If any milestone takes >3 weeks, reassess scope or approach for remaining milestones.

### Decision

**GO** -- Proceed to implementation. Begin with M1 (Rust kernel).

---

## Gate G4 Assessment

| Criterion | Target | Actual | Status |
|-----------|--------|--------|--------|
| Lean Canvas complete | All sections | 7/7 sections completed (adapted for personal project) | Pass |
| All 4 risks addressed | Value, Usability, Feasibility, Viability | All assessed with evidence and mitigations | Pass |
| Key metrics defined | Measurable targets | 8 metrics across architecture proof and execution quality | Pass |
| Go/No-Go documented | Clear decision with kill signals | GO decision with 3 explicit kill signals | Pass |

**Gate G4: PASS** -- Discovery complete. Ready for handoff.

---

## Phase Summary

| Phase | Gate | Status |
|-------|------|--------|
| 1. Problem Validation | G1 | PASS |
| 2. Opportunity Mapping | G2 | PASS |
| 3. Solution Testing | G3 | PASS |
| 4. Market Viability | G4 | PASS |

**Discovery Status: COMPLETE**

All four phases passed. The Alfred editor project is validated and ready for implementation, starting with M1 (Rust kernel -- event loop, rope buffer, terminal rendering, cursor movement).
