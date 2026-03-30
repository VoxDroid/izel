# ⬡ IZEL — EXHAUSTIVE TESTING CHECKLIST

This document lists specific test cases and edge cases required to verify the correctness, safety, and performance of the Izel compiler and language features.

## Verification Snapshot (2026-03-30)

- [x] `cargo check --workspace --all-targets`
- [x] `cargo test --workspace`
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] `bash tools/ci/check_system_deps.sh`
- [x] `bash tools/ci/check_coverage.sh --report-only` (workspace line coverage: `69.36%`)

Status legend:
- `[x]` means directly validated by executed automated checks and/or explicit regression tests.
- `[ ]` means not yet directly validated by an explicit automated test in the current repository.

Validation note:
- All crate-level `test_placeholder` integration stubs were replaced with concrete behavior tests in this validation pass.
- Full workspace line coverage is currently `69.36%`; the remaining unchecked scenarios in this checklist represent the principal path to 100%.

---

## 1. Front-End Tests (Lexer & Parser)

### 1.1 Lexer Edge Cases
- [ ] **Numeric Boundaries**:
    - [ ] `0b111...` (max `u128` value).
    - [x] `0xG` (invalid hex - should report error, not panic).
    - [ ] `1_000_...` (trailing underscore - should fail).
- [ ] **String Literals**:
    - [ ] Nested quotes in raw strings `r#" "quotes" "#`.
    - [ ] Multi-line interpolation ``` `sum: {x + \n y}` ```.
    - [ ] Invalid Unicode escape `\u{XYZZY}`.
- [ ] **Comments**:
    - [x] Nested multi-line comments `/~ /~ nested ~/ ~/`.
    - [ ] Doc comments `///` and `//!` at EOF.

### 1.2 Parser Assertions
- [ ] **Operator Precedence**:
    - [ ] `a |> b + c` (Pipeline vs Arithmetic).
    - [ ] `x as i32 + y` (Cast vs Arithmetic).
    - [ ] `not x and y` (Logical NOT vs AND).
- [ ] **Syntax Exhaustion**:
    - [ ] `forge` with default parameters and variadics.
    - [ ] `shape` with mixed visibility fields.
    - [ ] `scroll` with unit, tuple, and struct variants.
    - [ ] `branch` with complex guards `branch x { v given v > 0 and v < 10 => ... }`.
- [ ] **Error Recovery**:
    - [ ] Missing closing brace `}` in a `forge` body.
    - [x] Missing semicolon in a sequence.
    - [ ] Invalid token in a `draw` path.

---

## 2. Static Analysis Tests (Typeck & Borrowck)

### 2.1 Type Inference Scenarios
- [ ] **Generic Constraints**: 
    - [ ] `forge f<T: A + B>(...)` where `T` satisfies one but not both.
    - [ ] Higher-kinded associated types resolution.
- [ ] **Effect Unification**:
    - [ ] Closure that performs `!io` passed to a function that expects a `pure` closure (must fail).
    - [ ] Merging effects in a `branch` arm (e.g., one arm `!io`, one `!net` -> result is `!io, !net`).

### 2.2 Borrow Checker Violations (To Catch)
- [ ] **Movement**:
    - [x] Use of a value after it was passed by-value to a `forge`.
    - [ ] Movement out of a `shape` field.
- [ ] **References**:
    - [x] Creating a `&~` (mutable borrow) while a `&` (immutable) exists.
    - [ ] Returning a reference to a local variable (dangling pointer).
- [ ] **NLL Correctness**:
    - [ ] Reborrowing after a previous borrow's last use, but before the end of the scope.

---

## 3. Unique Feature Validation

### 3.1 Witness Proofs
- [x] **Construction Gating**:
    - [x] Attempt to manually construct `Witness::new()` outside a `@proof` function.
- [ ] **Safety Bypass**:
    - [ ] Use `raw` to create an invalid `NonZero<i32>(0)` and ensure subsequent `divide` panics correctly or is caught by sanitizers.

### 3.2 Temporal Constraints
- [x] **Pre-condition Violations**:
    - [x] Call `factorial(-1)` with a constant (compile error).
    - [x] Call `factorial(n)` where `n < 0` at runtime (runtime panic).
- [x] **Post-condition Verification**:
    - [x] A function that returns a value outside its `@ensures` range should fail verification.

### 3.3 Memory Zone Isolation
- [ ] **Leak Test**: Allocate 1,000,000 small objects in a `zone` and monitor RSS after the block ends.
- [ ] **Escape Test**:
    - [x] Assign a reference to a zone-allocated `String` to a variable declared outside the `zone`.
    - [ ] Pass a zone-allocated slice to a `tide` background flow (must fail if flow outlives zone).

### 3.4 Cascade Error Chains
- [ ] **Propagation Depth**: Verify a 10-level deep `!` propagation produces a 10-node context chain.
- [ ] **Message Overrides**: Ensure `result! or "custom"` correctly replaces the default context at that level.

### 3.5 Duality Check
- [ ] **Round-trip Accuracy**:
    - [ ] `JsonValue` -> `Shape` -> `JsonValue` comparison.
    - [ ] Large/Nested `dual shape` structures (e.g., AST nodes).

---

## 4. Performance & Backend Tests

### 4.1 Optimization Passes
- [ ] **Pipeline Fusion Check**:
    - [ ] Inspect LLVM IR for `iter |> map |> filter |> collect` to ensure no intermediate heap allocations occur.
- [ ] **TCO Success**:
    - [ ] Recursively call a `pure forge` 10,000,000 times to verify no stack overflow.

### 4.2 Code Generation Correctness
- [ ] **FFI Interop**:
    - [ ] Pass an Izel `shape` to C `memcpy` and back.
    - [ ] Handle null pointers returned from C `malloc` within a `raw` block.
- [ ] **Floating Point**:
    - [ ] Verify IEEE 754 compliance for `f32` and `f64` arithmetic across different targets.

---

## 5. Toolchain Verification

### 5.1 `izel-pm` (Package Manager)
- [ ] **Dependency Resolution**:
    - [ ] Construct a tree with diamond dependencies and verify version unification.
    - [ ] Test offline build cache behavior.

### 5.2 `izel-fmt`
- [x] **Idempotency**: `izel fmt` run twice on the same file should change 0 bytes.
- [ ] **Correctness**: Ensure formatter never changes code semantics (verified by compiling before and after).

### 5.3 `izel-lsp`
- [ ] **Go-to-Definition**: Cross-module jumps for types and methods.
- [ ] **Real-time Diagnostics**: Ensure error markers appear within 100ms of a syntax error being typed.

---

## 6. Verified Regression Coverage (Current)

- [x] Witness construction gating and built-in witness typing checks (`izel_typeck`).
- [x] Effect inference/boundary behavior and effect compatibility checks (`izel_typeck`).
- [x] Contract assertion and postcondition emission checks (`izel_mir`).
- [x] Zone allocator scope checks and zone escape detection (`izel_typeck`, `izel_borrow`).
- [x] Standard library API surface coverage checks (`izel_std`).
- [x] Phase 7 asset/surface guards for bootstrap, registry, tree-sitter, and playground (`izel_driver`).
- [x] All previously placeholder crate-level integration tests now execute concrete assertions.
