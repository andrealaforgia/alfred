# Alfred Codebase Refactoring Analysis
**Analysis Date:** 2026-03-25  
**Codebase Health Grade:** B  
**Paradigm:** Functional Core / Imperative Shell (Rust)  
**Code Smell Issues:** 14 total (2 high, 8 medium, 4 low)

---

## Executive Summary

The Alfred editor codebase is architecturally sound but has grown into three mega-files that violate the functional paradigm and create tight coupling between layers:

| File | Lines | Smells | Risk |
|------|-------|--------|------|
| `crates/alfred-tui/src/app.rs` | 7,987 | Long method, message chain, divergent change | HIGH |
| `crates/alfred-lisp/src/bridge.rs` | 5,198 | Feature envy, insider trading, coupling | HIGH |
| `crates/alfred-core/src/editor_state.rs` | 3,355 | God object, long method, shotgun surgery | HIGH |

These three files account for **65% of Rust code** and represent **100% of architectural debt**.

### Primary Issues

1. **Structural Coupling:** TUI reaches directly into 8 core submodules (buffer, cursor, viewport, etc.) via 355 fully-qualified references instead of using a facade
2. **God Object:** EditorState aggregates 39 public fields representing 10+ distinct subsystems; adding any subsystem requires cascading changes
3. **Monolithic State Machine:** Input event handling (605 lines), colon command dispatch (150 lines), and deferred action execution all live in app.rs
4. **Procedural Registration:** 30+ command handlers registered inline (887 lines) instead of via data-driven table
5. **Primitive Explosion:** 35 Lisp primitives each directly access 3-8 core submodules, creating intense coupling

### Impact on Development

- **Hard to Add Features:** New command = modify app.rs + editor_state.rs + bridge.rs (3 files, >500 lines changed)
- **Hard to Test:** Complex dispatch logic in app.rs untestable without full TUI setup
- **Hard to Extend:** Plugin authors struggle with bridge.rs coupling to internal core APIs
- **Hard to Maintain:** 605-line handle_key_event method has 12+ branching paths; one bug in one path risks all inputs

---

## Refactoring Roadmap

### Phase 1: Abstraction Boundaries (Weeks 1-2)

**Goal:** Establish facade layer to eliminate structural coupling

1. **Extract EditorFacade** (Refactoring #3, Effort: 18h)
   - Create `crates/alfred-core/src/facade.rs` with controlled read/write methods
   - Replace 355 direct `alfred_core::` references in app.rs with facade calls
   - Enable future mock implementations for testing

2. **Refactor Bridge to Use EditorFacade** (Refactoring #7, Effort: 16h)
   - Replace 8 direct submodule imports in bridge.rs with single `EditorFacade` import
   - Update all 35 Lisp primitives to use facade methods
   - Eliminates coupling explosion at extension boundary

**Risk Level:** Medium  
**Expected Outcome:** Dependency graph becomes `app.rs -> EditorFacade -> submodules` (layered) instead of `app.rs, bridge.rs -> [all submodules]` (flat)

---

### Phase 2: TUI Decomposition (Weeks 3-5)

**Goal:** Extract monolithic app.rs into specialized modules

3. **Extract InputStateMachine** (Refactoring #1, Effort: 24h)
   - Move 605-line `handle_key_event` to `crates/alfred-tui/src/input_machine.rs`
   - Pure function: `process_key(InputState, KeyEvent) -> (InputState, DeferredAction)`
   - Enables property-based testing of state transitions
   - Makes terminal I/O boundary explicit

4. **Extract ColonCommandDispatcher** (Refactoring #2, Effort: 16h)
   - Move 10+ colon command handlers to `crates/alfred-tui/src/colon_commands.rs`
   - Replace monolithic match with command registry
   - Open/Closed Principle: add commands without modifying dispatcher

5. **Extract DeferredActionExecutor** (Refactoring #4, Effort: 8h)
   - Move `execute_deferred_action` logic to `crates/alfred-core/src/command_dispatcher.rs`
   - Pure function: `execute(EditorState, DeferredAction) -> Result<(), String>`
   - Decouples command execution from app.rs event loop

6. **Extract RendererFacade** (Refactoring #8, Effort: 10h)
   - Create minimal facade over renderer module
   - Simplify app.rs event loop: read key -> process key -> render frame -> repeat

**Risk Level:** Medium  
**Expected Outcome:** app.rs shrinks from 7,987 to ~1,200 lines; pure domain logic extracted to testable modules

---

### Phase 3: Core Refactoring (Weeks 6-9)

**Goal:** Decompose EditorState and data-drive command registration

7. **Decompose EditorState into Subsystem Types** (Refactoring #5, Effort: 32h)
   - Extract: `BufferView`, `CursorView`, `RegisterSet`, `UndoState`, `SearchState`, `ModeState`
   - EditorState becomes composition of subsystems
   - Each subsystem has focused contract (SRP)
   - Enables independent testing and future extension

8. **Data-Drive Command Registration** (Refactoring #6, Effort: 20h)
   - Create `crates/alfred-core/src/command_table.rs` with declarative BUILTIN_COMMANDS table
   - Extract 30+ handlers to `crates/alfred-core/src/handlers/` module
   - `register_builtin_commands` shrinks from 887 to ~50 lines
   - New command = table row + handler function (no register logic change)

**Risk Level:** High  
**Expected Outcome:** EditorState becomes compositional; register_builtin_commands becomes transparent data table

---

### Phase 4: Code Cleanliness (Week 10)

9. **Remove Dead Code** (Refactoring #9, Effort: 1h)
10. **Replace Boolean Blindness** (Refactoring #10, Effort: 2h)
11. **Extract Duplicate Logic** (Refactoring #11, Effort: 3h)

**Risk Level:** Low  
**Expected Outcome:** Cleaner codebase, fewer confusion bugs

---

## Expected Outcomes

### Before Refactoring
```
Grade: B
Critical Issues: 2 (high-risk bloaters in app.rs and editor_state.rs)
Total Issues: 14
Largest Files:
  - app.rs: 7,987 lines
  - bridge.rs: 5,198 lines
  - editor_state.rs: 3,355 lines
Total Megafile Code: 16,540 lines (65% of codebase)
```

### After Refactoring
```
Expected Grade: A-
Critical Issues: 0
Residual Medium Issues: 1-2 (plugin system design debt for future)
Largest Files:
  - app.rs: ~1,200 lines (event loop + facade)
  - bridge.rs: ~2,000 lines (primitives, cleaner coupling)
  - editor_state.rs: ~1,000 lines (subsystem composition)
  + 6 new focused modules (input_machine, colon_commands, handlers, etc.)
Total Code Distribution: More balanced (no file >2,000 lines)
```

### Architectural Improvements

| Principle | Current | After Refactoring |
|-----------|---------|-------------------|
| **SRP** | 4 violations (app.rs, editor_state.rs, bridge.rs, command.rs) | 0 violations |
| **OCP** | Closed for new commands, inputs, modes | Open: data-driven tables |
| **DIP** | TUI depends on concrete modules (8 imports) | TUI depends on EditorFacade (1 import) |
| **Coupling** | 355 direct references in app.rs | <50 (all via facade) |
| **Testability** | Integration tests only; unit testing app.rs infeasible | Pure functions testable in isolation |

---

## Risk Management

### Critical Success Factors
1. **Phase 1 (EditorFacade) must complete before Phase 2** - Later phases depend on abstraction boundary
2. **Feature freeze during Phases 1-2** - Avoid merge conflicts during heavy refactoring
3. **Incremental commits per extracted component** - Enables rollback and bisect debugging
4. **Property-based testing for InputStateMachine** - Ensures complex state machine correctness

### Rollback Plan
Each phase produces working commits that can be independently reverted:
- Phase 1: Revert EditorFacade/bridge changes; restore direct module access
- Phase 2: Revert extracted modules; restore monolithic app.rs
- Phase 3: Revert subsystem types; restore flat EditorState; restore procedural command registration

---

## Resource Allocation

| Phase | Duration | Effort (hours) | Risk | Dependencies |
|-------|----------|--------------|------|--------------|
| 1: Abstraction Boundaries | 2 weeks | 34 | Medium | None |
| 2: TUI Decomposition | 3 weeks | 58 | Medium | Phase 1 |
| 3: Core Refactoring | 4 weeks | 52 | High | Phase 1 + 2 |
| 4: Code Cleanliness | 1 week | 6 | Low | None |
| **Total** | **10 weeks** | **150 hours** | — | — |

**Recommendation:** Allocate 15 hours/week to refactoring over 10 weeks while maintaining incremental feature development on Lisp plugins (non-conflicting).

---

## Detailed Refactoring Recommendations

See JSON files for comprehensive details:
- **`refactoring-expert-data.json`** - Full recommendations with code examples, steps, risk assessment
- **`ownership-analyzer-data.json`** - Git history analysis, subsystem ownership, change patterns

---

## Functional Paradigm Alignment

Current architecture violates functional paradigm in three ways:

1. **Mutable EditorState aggregation** - 39 public fields mutated directly by multiple sites
2. **Imperative command dispatch** - Conditional branching instead of data-driven composition
3. **Tight coupling in I/O shell** - App.rs entangles terminal I/O with domain logic

**Post-Refactoring Alignment:**

```rust
// Functional core: pure transformations
pub fn process_key(state: InputState, key: KeyEvent) -> (InputState, DeferredAction)
pub fn execute_command(editor: &mut EditorState, cmd: Command) -> Result<(), Error>

// Imperative shell: I/O boundaries
fn main_event_loop(editor: &mut EditorState, runtime: &LispRuntime) {
    loop {
        let key = read_terminal_event()?;
        let (next_state, action) = input_machine::process_key(state, key);
        command_dispatcher::execute(&mut editor, action, runtime)?;
        renderer::render_frame(&editor, &runtime)?;
    }
}
```

This aligns with CLAUDE.md paradigm:
- **Functional core:** Pure command handlers, input state machine, command dispatcher
- **I/O shell:** App event loop, terminal rendering, plugin discovery
- **Types-first:** InputState enum, DeferredAction enum, EditorFacade trait
- **Composition:** EditorState as composition of subsystems instead of aggregation

---

## Questions for Stakeholder Review

1. **EditorFacade API:** Should it be trait-based (dynamic dispatch) or struct-based (static dispatch)?
2. **Subsystem Stability:** After decomposition, should subsystems be independent crates?
3. **Plugin API:** Should EditorFacade methods be exposed to Lisp plugins for custom extension?
4. **Command Table:** Should command table support feature flags for conditional command sets?
5. **Testing Infrastructure:** Invest in property-based testing framework (proptest/quickcheck)?

---

## Appendix: File Structure After Refactoring

```
crates/alfred-core/src/
├── lib.rs
├── facade.rs (NEW: 250 lines)
├── command_dispatcher.rs (NEW: 200 lines)
├── buffer_view.rs (NEW: 150 lines, extracted from editor_state)
├── cursor_view.rs (NEW: 150 lines)
├── register_set.rs (NEW: 100 lines)
├── undo_state.rs (NEW: 100 lines)
├── search_state.rs (NEW: 80 lines)
├── mode_state.rs (NEW: 80 lines)
├── command_table.rs (NEW: 50 lines, data-driven)
├── handlers/
│   ├── mod.rs (NEW: 900 lines, all 30 command handlers)
│   ├── cursor.rs
│   ├── editing.rs
│   ├── register.rs
│   ├── search.rs
│   └── ...
├── editor_state.rs (REFACTORED: 3,355 -> 1,000 lines)
├── buffer.rs (unchanged: 1,668 lines)
├── cursor.rs (unchanged: 1,128 lines)
└── ...

crates/alfred-tui/src/
├── lib.rs
├── app.rs (REFACTORED: 7,987 -> 1,200 lines)
├── input_machine.rs (NEW: 800 lines)
├── colon_commands.rs (NEW: 600 lines)
├── renderer_facade.rs (NEW: 150 lines)
├── renderer.rs (unchanged: 1,307 lines)
└── ...

crates/alfred-lisp/src/
├── lib.rs
├── bridge.rs (REFACTORED: 5,198 -> 2,000 lines, uses EditorFacade only)
├── runtime.rs (unchanged: 552 lines)
└── ...
```

---

## Conclusion

This refactoring roadmap transforms Alfred from a code-smell-grade-B to grade-A codebase by:

1. Establishing clear architectural boundaries (EditorFacade)
2. Extracting monolithic modules into focused, testable components
3. Replacing imperative dispatch with data-driven composition
4. Decomposing God Objects into focused subsystem types

The 10-week effort (150 hours) addresses **70% of critical smells** and positions the codebase for sustainable long-term extension aligned with the declared functional paradigm.

**Risk is manageable with disciplined Phase-1-first approach, incremental commits, and peer code review.**

