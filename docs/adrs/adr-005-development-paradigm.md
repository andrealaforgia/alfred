# ADR-005: Hybrid Development Paradigm (Functional Core, Imperative Shell)

## Status

Accepted

## Context

Rust is multi-paradigm. The editor domain has both pure-functional aspects (buffer transformations, key resolution, command lookup) and inherently stateful aspects (event loop, terminal I/O, mutable editor state, Lisp interpreter state). The paradigm choice affects code structure, testability, and how AI agents approach implementation.

Three paradigms were considered for their fit with the domain:
- **Functional approach**: Pure core, effect boundaries, composition pipelines
- **OOP approach**: Traits as interfaces, dependency injection, command pattern
- **Hybrid**: Functional core for data transformations, imperative shell for I/O and state

## Decision

Use a **hybrid (functional core, imperative shell)** paradigm.

- **Functional core** (`alfred-core`): Buffer operations, keymap resolution, and command lookup are pure functions that take inputs and return outputs without side effects. Data types prefer immutability where Rust's ownership model allows. Testing is straightforward -- pass inputs, assert outputs, no mocking needed
- **Imperative shell** (`alfred-tui`, `alfred-bin`): The event loop, terminal rendering, file I/O, and Lisp interpreter management use mutable state and side effects. This is where `&mut self` lives
- **Traits as ports**: Traits define the boundary between core and infrastructure (e.g., rendering, file I/O). This is dependency inversion, not classical OOP

## Alternatives Considered

### Alternative 1: Pure Functional (composition pipelines, algebraic types)
- **What**: Model all state changes as immutable transformations. Use effect systems or monadic patterns for I/O
- **Expected impact**: Maximum testability, formal reasoning about state transitions
- **Why rejected**: Rust's ownership model fights pure FP patterns. Persistent data structures (required for pure FP) have higher overhead than Rust's move semantics. The borrow checker already provides safety guarantees that FP's immutability typically provides. Fighting the borrow checker for pure FP adds complexity without proportional benefit

### Alternative 2: OOP (traits-as-interfaces, deep abstraction)
- **What**: Model editor components as objects with interfaces. Heavy use of `dyn Trait`, `Box<dyn T>`, dependency injection throughout
- **Expected impact**: Familiar patterns, well-understood interfaces
- **Why rejected**: Deep trait hierarchies fight Rust's ownership model. Dynamic dispatch (`dyn Trait`) has runtime cost and prevents monomorphization. The editor's data flow (read event -> resolve key -> execute command -> render) is naturally a pipeline, not an object graph. Over-abstraction adds indirection without benefit for a single-implementation system

## Consequences

### Positive
- `alfred-core` is highly testable (pure functions, no mocking)
- Effect boundaries are explicit and visible in the crate structure
- Aligns with Helix's functional primitives approach (proven pattern)
- Rust's ownership model naturally enforces the boundary between pure and effectful code
- AI agents can implement core logic as pure functions and shell as straightforward imperative code

### Negative
- Requires discipline to keep the core pure (no `println!`, no file I/O, no terminal access in `alfred-core`)
- Some operations that feel like they should be methods (e.g., `buffer.insert()`) may be free functions for purity
- Developers familiar with pure OOP or pure FP may find the hybrid unfamiliar
