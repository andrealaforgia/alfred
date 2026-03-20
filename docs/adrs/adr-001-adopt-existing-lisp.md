# ADR-001: Adopt Existing Lisp Interpreter Rather Than Build From Scratch

## Status

Accepted

## Context

Alfred requires a Lisp interpreter as its extension language. The walking skeleton's goal is to prove that a plugin-first architecture works end-to-end, with Vim-style modal editing implemented entirely as a Lisp plugin. Two approaches were considered: building a custom Lisp interpreter following the MAL (Make A Lisp) incremental approach, or adopting an existing embeddable Lisp.

Building a Lisp interpreter is a project-sized effort (MAL has 11 steps, from tokenizer through self-hosting). For a project whose primary goal is showcasing an AI framework through a well-architected editor, the interpreter is a means to an end, not the end itself.

Quality attributes: **time-to-market** (reach the plugin architecture proof faster), **reliability** (avoid interpreter bugs that mask plugin architecture issues), **maintainability** (smaller surface area of custom code).

## Decision

Adopt an existing embeddable Lisp interpreter (Janet or rust_lisp -- see ADR-004 for selection). Do not build a custom interpreter for the walking skeleton.

## Alternatives Considered

### Alternative 1: Build Custom Lisp (MAL approach)
- **What**: Implement a Lisp interpreter in Rust following the MAL process (11 incremental steps)
- **Expected impact**: Full control over language design, tight Rust integration, Clojure-inspired syntax from day one
- **Why rejected**: Building a Lisp is a substantial effort (MAL steps 0-A). Interpreter bugs would obscure plugin architecture issues. The goal is to prove the architecture, not to build a language. Risk of scope creep is high -- language design is an absorbing problem

### Alternative 2: Use Lua (like Neovim)
- **What**: Embed LuaJIT or Lua 5.4 as the extension language
- **Expected impact**: Proven performance (LuaJIT), well-documented embedding, strong community
- **Why rejected**: Alfred's identity is Emacs-inspired with Lisp as the extension language. Lua does not provide homoiconicity or macros, which are core to the Lisp-based editor experience. Neovim chose Lua because Vimscript was already the primary language; Alfred starts fresh and can choose Lisp

## Consequences

### Positive
- Eliminates interpreter development effort (estimated 3-4 weeks saved)
- Inherited reliability from a proven interpreter
- Faster path to M2 (Lisp integration) -- can focus on the FFI bridge, not the language
- Smaller codebase to maintain

### Negative
- Less control over language syntax and semantics
- Must work within the adopted interpreter's constraints (e.g., syntax, built-in types)
- Clojure-inspired syntax may require a preprocessing layer on top of the adopted interpreter (deferred)
- Dependency on external project's maintenance trajectory
