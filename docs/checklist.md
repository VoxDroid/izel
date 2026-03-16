# ⬡ IZEL — DETAILED IMPLEMENTATION CHECKLIST

This document provides a granular, step-by-step roadmap for building the Izel compiler and toolchain. Every action is aligned with the vision of a "unique, only one" systems language.

---

## Phase 0: Bootstrap & Infrastructure (Months 1–2)
*Goal: Establish the workspace and the "Hello World" path.*

### 0.1 Workspace Setup
- [ ] **Root Initialization**:
    - [ ] Create `Cargo.toml` with `[workspace]` and initial members.
    - [ ] Add `rust-toolchain.toml` (targeting stable/nightly Rust).
    - [ ] Configure `.gitignore` (ignore `target/`, `Izel.lock`, `.izel/`).
- [ ] **Core Utility Crates**:
    - [ ] `izel_span`: 
        - [ ] Define `BytePos`, `SourceId`, and `Span` structs.
        - [ ] Implement `SourceMap` to manage file buffers.
    - [ ] `izel_diagnostics`:
        - [ ] Integrate `codespan-reporting`.
        - [ ] Define `Diagnostic` and `Label` wrappers.
        - [ ] Implement `emit` for rich terminal output.
    - [ ] `izel_session`:
        - [ ] Define `Session` global state.
        - [ ] Implement `ParseOptions` and `Config` (via `clap`).

### 0.2 `izel_lexer` (DFA Tokenizer)
- [ ] **Token Definitions**:
    - [ ] Map all keywords (`forge`, `shape`, `weave`, etc.).
    - [ ] Define sigils (`~`, `!`, `@`, `|>`, `::`, `->`, `=>`, `..`, `..=`).
- [ ] **Scanner Logic**:
    - [ ] Implement `Cursor` for UTF-8 character streaming.
    - [ ] Handle comments: `//` (single) and `/~ ... ~/` (nested/multi).
    - [ ] Implement `StringReader`: esc codes, Unicode escapes `\u{...}`.
    - [ ] Implement `NumberReader`: Support `_` separators, hex/oct/bin prefixes.
- [ ] **Verification**:
    - [ ] Implement `izelc --emit tokens` for debugging.
    - [ ] Set up `cargo-fuzz` target for the lexer.

### 0.3 `izel_parser` (CST & AST)
- [ ] **CST Infrastructure**:
    - [ ] Define `GreenNode` or equivalent for lossless representation.
    - [ ] Ensure all whitespace and comments are preserved (trivia).
- [ ] **Expression Parser (Pratt)**:
    - [ ] Implement precedence table (14 levels).
    - [ ] Support pipeline `|>` (level 1) to method calls/path (level 14).
- [ ] **Declaration Parser**:
    - [ ] `forge` (functions) with param lists and return types.
    - [ ] `let` / `~` bindings.
    - [ ] Simple blocks `{ ... }`.

### 0.4 `izel_codegen` (Minimal Path)
- [ ] **LLVM Integration**:
    - [ ] Initialize `Context`, `Module`, and `Builder` via `inkwell`.
    - [ ] Implement `Codegen` for primitives (`i32`, `f64`, `bool`).
    - [ ] Map `forge main()` to C-style `main`.
- [ ] **Minimal Runtime**:
    - [ ] Implement `builtin_println` in Rust/C and link it.
- [ ] **Verification**: 
    - [ ] Run `izelc hello.iz` and produce an executable.

---

## Phase 1: Core Language Elaboration (Months 3–5)
*Goal: Complete the syntax and module resolution.*

### 1.1 Complete Syntax Support
- [ ] **Composite Types**:
    - [ ] `shape` (structs) with field visibility (`open`, `hidden`).
    - [ ] `scroll` (enums) with data-carrying variants.
- [ ] **Control Flow**:
    - [ ] `given` / `else` (with expression support).
    - [ ] `branch` (exhaustive pattern matching).
    - [ ] `loop`, `while`, `each .. in`.
- [ ] **Abstractions**:
    - [ ] `weave` (interfaces).
    - [ ] `shape impl` / `weave impl` blocks.
- [ ] **Functional Blocks**:
    - [ ] `bind` (closures) and `move` semantics.

### 1.2 `izel_resolve` (Name & Module Resolution)
- [ ] **Scope Tree**:
    - [ ] Implement lexical scoping and shadowing logic.
    - [ ] Handle `self` and `super` keywords.
- [ ] **Module Graph**:
    - [ ] Build dependency graph from `draw` requests.
    - [ ] Detect cyclic imports and report as errors.
    - [ ] Implement `ward` hierarchy (nested modules).
- [ ] **Symbol Table**:
    - [ ] Map idents to unique `DefId`s.
    - [ ] Handle re-exports and wildcard `*` imports.

### 1.3 `izel_ast_lower` (Desugaring)
- [ ] **Sugar Expansion**:
    - [ ] Expand `` `...` `` interpolated strings to `std::fmt` calls.
    - [ ] Expand `x!` (cascade propagation) to match-based return.
    - [ ] Expand `?T` to `Option<T>`.
    - [ ] Expand `??` (null-coalesce) and `?.` (opt-chain).

---

## Phase 2: Static Analysis & Correctness (Months 6–8)
*Goal: Implement the type system and borrow checker.*

### 2.1 `izel_typeck` (Type Inference)
- [ ] **Inference Engine**:
    - [ ] Implement Hindley-Milner with constraint gathering.
    - [ ] Implement Row-based unification for effects.
- [ ] **Traits & Poly**:
    - [ ] Resolve `weave` bounds on generics.
    - [ ] Handle associated types (`type Item`).
    - [ ] Implement orphan rule check (coherence).
- [ ] **Effect System**:
    - [ ] Transitive effect discovery (e.g., `f` calls `g !io` -> `f` is `!io`).
    - [ ] Check `forge f() !effect` annotations at call sites.

### 2.2 `izel_borrow` (Ownership System)
- [ ] **Ownership Tracking**:
    - [ ] Map movements of bindings (consume vs borrow).
- [ ] **Region Inference (NLL)**:
    - [ ] Build Control Flow Graph (CFG).
    - [ ] Calculate live ranges for every binding.
    - [ ] Enforce "One Mutable XOR Many Immutable" rule.
- [ ] **Lifetime Annotations**:
    - [ ] Allow explicit `'a` elision and verification.

---

## Phase 3: Unique Feature Implementation (Months 9–12)
*Goal: The distinguishing features of Izel.*

### 3.1 Witness Types & Proofs
- [ ] **System Design**:
    - [ ] Implement `Witness<P>` as a lang-item.
    - [ ] Restrict construction to `@proof` tagged functions.
- [ ] **Built-ins**:
    - [ ] Implement `NonZero<T>`, `InBounds<T>`, `Sorted<T>`.
- [ ] **Verification**:
    - [ ] Ensure `raw` is the only way to bypass proofs.

### 3.2 Temporal Constraints (`@requires` / `@ensures`)
- [ ] **Compile-time Engine**:
    - [ ] Create symbolic evaluator for static constant expressions.
- [ ] **Runtime Instrumentation**:
    - [ ] For dynamic inputs, inject assertions into functions.
    - [ ] Add `izelc --check-contracts` flag.
- [ ] **Invariants**: 
    - [ ] Implement `#[invariant]` checking for `shape` state.

### 3.3 Memory Zones
- [ ] **Allocators**:
    - [ ] Implement `ZoneAllocator` (Arena style).
- [ ] **Escape Analysis**:
    - [ ] Verify zone-allocated data never outlives the `zone` block.
- [ ] **Codegen**:
    - [ ] Emit bulk deallocation at the end of `zone` blocks.

### 3.4 Cascade Error System
- [ ] **Error Context**:
    - [ ] Extend `Result<T, E>` to `Result<T, Cascade<E>>`.
    - [ ] Implement the `or "message"` context override for `!`.
- [ ] **Trace Construction**:
    - [ ] Auto-capture `here!()` (file/line) on propagation.

### 3.5 Duality Types
- [ ] **Elaboration**:
    - [ ] Derive `decode` from `encode` logic in `dual shape`.
- [ ] **Verification**:
    - [ ] Auto-generate `#[test]` for round-trip law if effectful.
    - [ ] Statically prove round-trip if `pure`.

---

## Phase 4: Standard Library & Runtime (Months 13–15)
- [ ] **`std::prim`**: Integral math, floating point, bool logic.
- [ ] **`std::iter`**: Full pipeline suite (`map`, `filter`, `fold`, `zip`, etc.).
- [ ] **`std::collections`**:
    - [ ] `Vec<T>`, `HashMap<K, V>`, `BTreeMap<K, V>`.
- [ ] **Async Native**:
    - [ ] Implement `flow` tasks and `tide` executors.
    - [ ] Channel implementation (`std::chan`).

---

## Phase 5: Developer Tooling (Months 16–18)
- [ ] **`izel-fmt`**: Lossless CST-based deterministic formatter.
- [ ] **`izel-lsp`**:
    - [ ] Implement `tower-lsp` server.
    - [ ] Hook into `izel_query` (`salsa`) for incremental analysis.
- [ ] **`izel-pm`**:
    - [ ] Implement `Izel.toml` parser with `winnow`.
    - [ ] Build dependency resolver (SemVer).

---

## Phase 6: MIR Optimization (Months 19–22)
- [ ] **IR Transformation**:
    - [ ] AST -> HIR -> MIR (SSA form).
- [ ] **Passes**:
    - [ ] **Pipeline Fusion**: Fuse `map(f).filter(g)` into a single loop.
    - [ ] **TCO**: Converge tail recursion to jumps.
    - [ ] **LICM**: Loop invariant code motion.
    - [ ] **DCE**: Dead code elimination.

---

## Phase 7: Self-Hosting (Months 23+)
- [ ] **Porting**:
    - [ ] Lexer -> Izel.
    - [ ] Parser -> Izel.
- [ ] **The Big Leap**:
    - [ ] Compile Izel-Lexer with Izel-Lexer.
    - [ ] Verify bootstrap checksums.
