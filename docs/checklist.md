# ⬡ IZEL — DETAILED IMPLEMENTATION CHECKLIST

This document provides a granular, step-by-step roadmap for building the Izel compiler and toolchain. Every action is aligned with the vision of a "unique, only one" systems language.

---

## Phase 0: Bootstrap & Infrastructure (Months 1–2)
*Goal: Establish the workspace and the "Hello World" path.*

### 0.1 Workspace Setup
- [x] **Root Initialization**:
    - [x] Create `Cargo.toml` with `[workspace]` and initial members.
    - [x] Add `rust-toolchain.toml` (targeting stable/nightly Rust).
    - [x] Configure `.gitignore` (ignore `target/`, `Izel.lock`, `.izel/`).
- [x] **Core Utility Crates**:
    - [x] `izel_span`: 
        - [x] Define `BytePos`, `SourceId`, and `Span` structs.
        - [x] Implement `SourceMap` to manage file buffers.
    - [x] `izel_diagnostics`:
        - [x] Integrate `codespan-reporting`.
        - [x] Define `Diagnostic` and `Label` wrappers.
        - [x] Implement `emit` for rich terminal output.
    - [x] `izel_session`:
        - [x] Define `Session` global state.
        - [x] Implement `ParseOptions` and `Config` (via `clap`).


### 0.2 `izel_lexer` (DFA Tokenizer)
- [x] **Token Definitions**:
    - [x] Map all keywords (`forge`, `shape`, `weave`, etc.).
    - [x] Define sigils (`~`, `!`, `@`, `|>`, `::`, `->`, `=>`, `..`, `..=`).
- [x] **Scanner Logic**:
    - [x] Implement `Cursor` for UTF-8 character streaming.
    - [x] Handle comments: `//` (single) and `/~ ... ~/` (nested/multi).
    - [x] Implement `StringReader`: esc codes, Unicode escapes `\u{...}`.
    - [x] Implement `NumberReader`: Support `_` separators, hex/oct/bin prefixes.
- [x] **Verification**:
    - [x] Implement `izelc --emit tokens` for debugging.
    - [x] Set up `cargo-fuzz` target for the lexer.


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
    - [x] Basic `i32` arithmetic and `return` generation (Supports basic JIT execution).
- [x] **Verification**:
    - [x] Output human-readable `.ll` (LLVM Assembly).
    - [x] JIT execution of `main` for smoke tests.

### 0.5 `izel_mir` (Mid-level IR)
- [x] **MIR Infrastructure**:
    - [x] Define `MirBody`, `BasicBlock`, `Instruction`, `Terminator`.
    - [x] Use `petgraph` for Control Flow Graph representation.
- [x] **Lowering (Initial)**:
    - [x] Lower CST `forge` declarations to `MirBody`.
    - [x] Handle `LetStmt` and simple `i32` expressions in MIR.
- [x] **Verification**: 
    - [x] Run `izelc hello.iz` and produce an executable.

---

## Phase 1: Core Language Elaboration (Months 3–5)
*Goal: Complete the syntax and module resolution.*

### 1.1 Complete Syntax Support
- [x] **Composite Types**:
    - [x] `shape` (structs) with field visibility (`open`, `hidden`).
    - [x] `scroll` (enums) with data-carrying variants.
    - [x] `dual` (duality types) initial skeleton.
- [x] **Control Flow**:
    - [x] `given` / `else` (with expression support).
    - [x] `branch` (exhaustive pattern matching).
    - [x] `loop`, `while`, `each .. in`.
- [x] **Abstractions**:
    - [x] `weave` (interfaces).
    - [x] `shape impl` / `weave impl` blocks.
- [x] **Functional Blocks**:
    - [x] `bind` (closures) and `move` semantics.

### 1.2 `izel_resolve` (Name & Module Resolution)
- [x] **Scope Tree**:
    - [x] Implement lexical scoping and basic symbol definition.
- [x] **Module Graph**:
    - [x] Build dependency graph from `draw` requests.
    - [x] Detect cyclic imports and report as errors.
    - [x] Implement `ward` hierarchy (nested modules).
- [x] **Symbol Table**:
    - [x] Map idents to unique `DefId`s (Local variables and globals).
    - [x] Handle re-exports and wildcard `*` imports.

### 1.3 `izel_ast_lower` (Desugaring)
- [x] **Sugar Expansion**:
    - [x] Expand `` `...` `` interpolated strings (Tokenizer and Parser support).
    - [x] Expand `x!` (cascade propagation) to match-based return.
    - [x] Expand `?T` to `Option<T>` (AST target defined).
    - [x] Expand `??` (null-coalesce) and `?.` (opt-chain) (Optional chaining done).

---

## Phase 2: Static Analysis & Correctness (Months 6–8)
*Goal: Implement the type system and borrow checker.*

### 2.1 `izel_typeck` (Type Inference)
- [x] **Inference Engine**:
    - [x] Define comprehensive `Type` enum and `TypeChecker` structure.
    - [x] Implement basic unification (Algorithm W style).
    - [x] Implement Hindley-Milner with constraint gathering.
    - [x] Implement Row-based unification for effects.
- [x] **Traits & Poly**:
    - [x] Resolve `weave` bounds on generics.
    - [x] Handle associated types (`type Item`).
    - [x] Implement orphan rule check (coherence).
- [x] **Effect System**:
    - [x] Transitive effect discovery (e.g., `f` calls `g !io` -> `f` is `!io`).
    - [x] Check `forge f() !effect` annotations at call sites.

### 2.2 `izel_borrow` (Ownership System)
- [x] **Ownership Tracking**:
    - [x] Map movements of bindings (consume vs borrow).
- [x] **Region Inference (NLL)**:
    - [x] Build Control Flow Graph (CFG).
    - [x] Calculate live ranges for every binding.
    - [x] Enforce "One Mutable XOR Many Immutable" rule.
- [x] **Lifetime Annotations**:
    - [x] Allow explicit `'a` elision and verification.

---

## Phase 3: Unique Feature Implementation (Months 9–12)
*Goal: The distinguishing features of Izel.*

### 3.1 Witness Types & Proofs
- [x] **System Design**:
    - [x] Implement `Witness<P>` as a lang-item.
    - [x] Restrict construction to `@proof` tagged functions.
- [x] **Built-ins**:
    - [x] Implement `NonZero<T>`, `InBounds<T>`, `Sorted<T>`.
- [x] **Verification**:
    - [x] Ensure `raw` is the only way to bypass proofs.

### 3.2 Temporal Constraints (`@requires` / `@ensures`)
- [x] **Compile-time Engine**:
    - [x] Create symbolic evaluator for static constant expressions.
- [x] **Runtime Instrumentation**:
    - [x] For dynamic inputs, inject assertions into functions.
    - [x] Add `izelc --check-contracts` flag.
- [x] **Invariants**: 
    - [x] Implement `#[invariant]` checking for `shape` state.

### 3.3 Memory Zones
- [x] **Allocators**:
    - [x] Implement `ZoneAllocator` (Arena style).
- [x] **Escape Analysis**:
    - [x] Verify zone-allocated data never outlives the `zone` block.
- [x] **Codegen**:
    - [x] Emit bulk deallocation at the end of `zone` blocks.

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
