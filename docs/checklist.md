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
- [x] **Token Definitions**:
    - [x] Map all keywords (`forge`, `shape`, `weave`, etc.).
    - [x] Define sigils (`~`, `!`, `@`, `|>`, `::`, `->`, `=>`, `..`, `..=`).
- [x] **Scanner Logic**:
    - [x] Implement `Cursor` for UTF-8 character streaming.
    - [x] Handle comments: `//` (single) and `/~ ... ~/` (nested/multi).
    - [ ] Implement `StringReader`: esc codes, Unicode escapes `\u{...}`.
    - [x] Implement `NumberReader`: Support `_` separators, hex/oct/bin prefixes.
- [x] **Verification**:
    - [x] Implement `izelc --emit tokens` for debugging.
    - [ ] Set up `cargo-fuzz` target for the lexer.

### 0.3 `izel_parser` (CST & AST)
- [x] **CST Infrastructure**:
    - [x] Define `GreenNode` or equivalent for lossless representation.
    - [x] Ensure all whitespace and comments are preserved (trivia).
- [x] **Expression Parser (Pratt)**:
    - [x] Implement precedence table (14 levels).
    - [x] Support pipeline `|>` (level 1) to method calls/path (level 14).
- [x] **Declaration Parser**:
    - [x] Implement `forge` declaration parsing with CST nodes.
    - [x] Implement `let`/`~` binding and block parsing.
    - [x] Simple blocks `{ ... }`.

### 0.4 `izel_codegen` (Minimal Path)
- [x] **LLVM Integration**:
    - [x] Set up `inkwell` context and module.
    - [x] Implement `Codegen` struct to walk the CST/AST.
- [x] **Forge Generation**:
    - [x] Direct mapping of `forge` to LLVM `Function`.
    - [x] Basic `i32` arithmetic and `return` generation.
- [x] **Verification**:
    - [x] Output human-readable `.ll` (LLVM Assembly).
    - [x] JIT execution of `main` for smoke tests.

### 0.5 `izel_mir` (Mid-level IR)
- [x] **MIR Infrastructure**:
    - [x] Define `MirBody`, `BasicBlock`, `Instruction`, `Terminator`.
    - [x] Use `petgraph` for Control Flow Graph representation.
- [ ] **Lowering (Initial)**:
    - [ ] Lower CST `forge` declarations to `MirBody`.
    - [ ] Handle `LetStmt` and simple `i32` expressions in MIR.
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
- [x] **Scope Tree**:
    - [x] Implement lexical scoping and basic symbol definition.
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
