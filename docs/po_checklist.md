# Izel Project Overview Checklist

## 1. Identity & Vision
- [ ] The Name
- [ ] The Mission
- [ ] Core Promises

## 2. What Makes Izel Truly Unique
- [ ] The Seven Pillars of Uniqueness

## 3. Design Philosophy
- [ ] The Twelve Principles of Izel

## 4. Lexical Structure & Notation
- [x] 4.1 File Extension (`.iz`, `.izm`)
- [x] 4.2 Character Set (UTF-8, UAX #31)
- [x] 4.3 Comments (`//`, `///`, `//!`, `/* */`)
- [x] 4.4 Sigils (`~`, `!`, `@`, `|>` etc.)
- [x] 4.5 Keywords
- [x] 4.6 Reserved Future Keywords
- [x] 4.7 Literals (Int, Float, Str, Interpolated, Char, Bool, Nil)

## 5. Variables & Bindings
- [x] 5.1 Immutable Bindings (`let`)
- [x] 5.2 Mutable Bindings (`~`)
- [x] 5.3 Shadowing
- [x] 5.4 Destructuring
- [x] 5.5 Constants and Statics

## 6. Primitive & Compound Types
- [x] 6.1 Numeric Types
- [x] 6.2 Other Primitives (`bool`, `char`, `str`, `()`, `!`)
- [x] 6.3 Compound Types (Tuples, Arrays, Slices, Optionals)
- [x] 6.4 The `?T` Optional Type

## 7. Functions — `forge`
- [x] 7.1 Basic Syntax
- [x] 7.2 `give` — The Return Statement
- [x] 7.3 Named Parameters & Defaults
- [x] 7.4 Variadic Functions
- [x] 7.5 `pure` Functions
- [x] 7.6 Method Syntax (on shapes)
- [x] 7.7 Function Overloading

## 8. Structures — `shape`
- [x] 8.1 Basic Shape
- [x] 8.2 Instantiation & Mutation
- [x] 8.3 Tuple Shapes & Unit Shapes
- [x] 8.4 Packed & Aligned Shapes
- [x] 8.5 Visibility

## 9. Enumerations — `scroll`
- [x] 9.1 Basic & Data-Carrying Variants
- [x] 9.2 Recursive Scrolls
- [x] 9.3 Scroll Methods

## 10. Traits & Interfaces — `weave`
- [x] 10.1 Defining & Implementing a Weave
- [x] 10.2 Weave as Type Bounds
- [x] 10.3 Associated Types & Inheritance
- [x] 10.4 Operator Overloading via Weaves

## 11. Pattern Matching — `branch`
- [x] 11.1 Basic Matching
- [x] 11.2 Destructuring, Guards, Multi-Pattern

## 12. Control Flow
- [x] 12.1 `given` / `else` (Conditionals)
- [x] 12.2 Loops (`loop`, `while`, `each`)
- [x] 12.3 `break`, `next` (continue)

## 13. Closures & Lambdas
- [x] Complete Implementation

## 14. Iterators & Pipelines
- [x] 14.1 The `|>` Operator
- [x] 14.2 Full Combinator List (std::iter)
- [x] 14.3 Custom Iterators

## 15. Error Handling — Cascade Errors
- [x] 15.1 Result<T, E> and Error Propagation
- [x] 15.2 The `!` Cascade Propagator
- [x] 15.3 `seek` / `catch` — Inline Handling
- [x] 15.4 Custom Error Types

## 16. Generics & Parametric Polymorphism
- [x] Generic Types and Implementation

## 17. The Effect System
- [x] 17.1 Built-in Effects
- [x] 17.2 Declaring Effects
- [x] 17.3 Effect Propagation
- [x] 17.4 Effect Boundaries
- [x] 17.5 Effect-Based Testing

## 18. Witness Types
- [x] 18.1 Built-in Witnesses
- [ ] 18.2 Using Witnesses to Eliminate Runtime Checks
- [ ] 18.3 Custom Witnesses

## 19. Temporal Constraints
- [ ] Verification and Enforcement of Constraints

## 20. Memory Zones
- [ ] Zones syntax and lifetimes

## 21. Duality Types
- [ ] Dual shapes parsing and elaboration

## 22. Compile-Time Evaluation — `echo`
- [ ] Implementation of `echo` nodes

## 23. Modules & Visibility — `ward`
- [x] 23.1 Defining & Nesting Wards
- [x] 23.2 `draw` — Import Statement
- [x] 23.3 File-Based Wards

## 24. Macros & Meta-Programming
- [ ] 24.1 Declarative Macros
- [ ] 24.2 `#[derive(...)]` — Built-in Derivable Weaves
- [ ] 24.3 Attribute Macros

## 25. Concurrency & Async — `flow` / `tide`
- [ ] 25.1 Threads & Channels
- [x] 25.2 `flow` / `tide` — Async Functions & Await
- [ ] 25.3 Atomic Types

## 26. Raw Blocks & FFI — `raw` / `bridge`
- [ ] 26.1 `raw` Blocks
- [ ] 26.2 `bridge` — C/C++ FFI
- [ ] 26.3 Inline Assembly

## 27. Architecture Overview
- [x] Aligned

## 28. Compiler Pipeline
- [x] 28.1 Lexer (`izel_lexer`)
- [x] 28.2 Parser (`izel_parser`)
- [x] 28.3 AST Lowering
- [x] 28.4 Name Resolution
- [x] 28.5 Type Checker (`izel_typeck`)
- [x] 28.6 Borrow Checker (`izel_borrow`)
- [x] 28.7 HIR (`izel_hir`)
- [x] 28.8 MIR (`izel_mir`)
- [x] 28.9 Optimizer (`izel_opt`)
- [x] 28.10 Code Generation (`izel_codegen`)

## 29. Type System (Formal)
- [x] 29.1 Type Kinds
- [x] 29.2 Subtyping Rules
- [x] 29.3 Inference
- [x] 29.4 Coherence (Orphan Rule)

## 30. Memory Model
- [x] 30.1 Ownership Rules
- [x] 30.2 Borrowing Rules
- [x] 30.3 RAII & Drop
- [x] 30.4 Allocator Parameterization
- [x] 30.5 Memory Regions Summary

## 31. Standard Library
- [ ] Core (no `!alloc` needed)
- [x] Allocation & Collections
- [ ] I/O & OS
- [ ] Concurrency
- [ ] Math, Hash, Codec
- [ ] Testing

## 32. Toolchain
- [x] `izelc` — Compiler Binary
- [x] `izel` — Package Manager & Build System
- [x] `Izel.toml` — Project Manifest
- [x] `izel-lsp` — Language Server (LSP 3.17)
- [x] `izel-fmt` — Formatter
- [x] `izel-lint` — Linter

## 33. Project Directory Structure
- [x] Scaffolding Complete

## 34. Milestones & Roadmap
- [ ] Phase 0 — Bootstrap (Months 1–2)
- [ ] Phase 1 — Core Language (Months 3–5)
- [ ] Phase 2 — Type System & Safety (Months 6–8)
- [ ] Phase 3 — Unique Features (Months 9–12)
- [x] Phase 4 — Standard Library v0.1 (Months 13–15)
- [x] Phase 5 — Toolchain (Months 16–18)
- [ ] Phase 6 — Optimization & Hardening (Months 19–22)
- [ ] Phase 7 — Self-Hosting (Months 23+)

## 35. Dependencies
- [x] Rust Crates
- [ ] System Dependencies

## 36. Contributing Guidelines
- [x] Getting Started
- [ ] Commit Convention (Conventional Commits)
- [x] Pull Request Requirements
- [x] Testing Philosophy

## 37. Language Specification Index
- [x] Indexed
