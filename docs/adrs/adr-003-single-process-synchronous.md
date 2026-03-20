# ADR-003: Single-Process Synchronous Execution

## Status

Accepted

## Context

Editor architectures range from single-process/single-threaded (Emacs, Kakoune) to multi-process/fully-async (Xi). The choice impacts complexity, reliability, and performance.

The strongest evidence comes from Xi editor's retrospective by Raph Levien: "I now firmly believe that the process separation between front-end and core was not a good idea." Xi's async-everywhere approach made even basic features (word wrapping, scrolling) exponentially harder to implement. JSON protocol communication caused performance issues (Swift's JSON parsing was "shockingly slow"; Rust's serde caused 9.3MB binary bloat).

Additional evidence:
- Emacs: single-threaded for 40+ years, remains the most extensible editor
- Kakoune: design document explicitly states no multithreading
- Neovim: single main thread with libuv for I/O multiplexing
- Zed: multi-threaded async, but has a full-time professional team and years of development

Quality attributes: **reliability** (avoid distributed system failure modes), **maintainability** (simpler mental model), **time-to-market** (less complexity to build).

## Decision

Alfred uses single-process, synchronous execution for the walking skeleton. The event loop, Lisp evaluation, and rendering all run on the main thread. No async runtime, no multi-process communication, no background threads.

## Alternatives Considered

### Alternative 1: Multi-process (Xi-style)
- **What**: Frontend and core in separate processes communicating via JSON-RPC. Plugins in separate processes
- **Expected impact**: Clean separation, crash isolation, language independence for frontend
- **Why rejected**: Xi editor's author explicitly warns against this approach. The complexity overwhelmed development. For a solo developer + AI agents, this is infeasible for a walking skeleton

### Alternative 2: Async-everywhere (tokio-based)
- **What**: Use tokio runtime, async/await throughout, non-blocking I/O
- **Expected impact**: Responsive UI during long operations, familiar to Rust async ecosystem
- **Why rejected**: Adds colored function problem (async infects entire codebase). Debugging async code is harder. The walking skeleton has no operations long enough to justify async (no LSP, no syntax highlighting, no network). Premature complexity

### Alternative 3: Single-process with background thread pool
- **What**: Main thread for event loop + rendering, thread pool for heavy computation
- **Expected impact**: UI stays responsive during background work
- **Why rejected**: Not needed for walking skeleton scope. No operations require background processing. Can be added later when specific needs arise (e.g., syntax highlighting, file search). Adding prematurely introduces synchronization complexity

## Consequences

### Positive
- Simplest possible execution model
- No synchronization bugs (no mutexes, no channels, no async state machines)
- Deterministic behavior -- easy to test, debug, and reason about
- Matches the Emacs model which has proven sufficient for 40+ years
- Lisp evaluation is simpler without async considerations

### Negative
- Long-running Lisp expressions will freeze the UI (acceptable for walking skeleton -- no long operations expected)
- No parallelism for CPU-intensive operations (syntax highlighting -- deferred anyway)
- Will need architectural evolution when async capabilities are added post-skeleton
- Cannot do background file operations (auto-save, incremental search -- deferred)
