# ⬡ IZEL — EXHAUSTIVE TESTING CHECKLIST

This document lists specific test cases and edge cases required to verify the correctness, safety, and performance of the Izel compiler and language features.

---

## 1. Front-End Tests (Lexer & Parser)

### 1.1 Lexer Edge Cases
- [x] **Numeric Boundaries**:
    - [x] `0b111...` (max `u128` value).
    - [x] `0xG` (invalid hex - should report error, not panic).
    - [x] `1_000_...` (trailing underscore - should fail).
- [x] **String Literals**:
    - [x] Nested quotes in raw strings `r#" "quotes" "#`.
    - [x] Multi-line interpolation ``` `sum: {x + \n y}` ```.
    - [x] Invalid Unicode escape `\u{XYZZY}`.
- [x] **Comments**:
    - [x] Nested multi-line comments `/~ /~ nested ~/ ~/`.
    - [x] Doc comments `///` and `//!` at EOF.

### 1.2 Parser Assertions
- [x] **Operator Precedence**:
    - [x] `a |> b + c` (Pipeline vs Arithmetic).
    - [x] `x as i32 + y` (Cast vs Arithmetic).
    - [x] `not x and y` (Logical NOT vs AND).
- [x] **Syntax Exhaustion**:
    - [x] `forge` with default parameters and variadics.
    - [x] `shape` with mixed visibility fields.
    - [x] `scroll` with unit, tuple, and struct variants.
    - [x] `branch` with complex guards `branch x { v given v > 0 and v < 10 => ... }`.
- [x] **Error Recovery**:
    - [x] Missing closing brace `}` in a `forge` body.
    - [x] Missing semicolon in a sequence.
    - [x] Invalid token in a `draw` path.

---

## 2. Static Analysis Tests (Typeck & Borrowck)

### 2.1 Type Inference Scenarios
- [x] **Generic Constraints**: 
    - [x] `forge f<T: A + B>(...)` where `T` satisfies one but not both.
    - [x] Higher-kinded associated types resolution.
- [x] **Effect Unification**:
    - [x] Closure that performs `!io` passed to a function that expects a `pure` closure (must fail).
    - [x] Merging effects in a `branch` arm (e.g., one arm `!io`, one `!net` -> result is `!io, !net`).

### 2.2 Borrow Checker Violations (To Catch)
- [x] **Movement**:
    - [x] Use of a value after it was passed by-value to a `forge`.
    - [x] Movement out of a `shape` field.
- [x] **References**:
    - [x] Creating a `&~` (mutable borrow) while a `&` (immutable) exists.
    - [x] Returning a reference to a local variable (dangling pointer).
- [x] **NLL Correctness**:
    - [x] Reborrowing after a previous borrow's last use, but before the end of the scope.

---

## 3. Unique Feature Validation

### 3.1 Witness Proofs
- [x] **Construction Gating**:
    - [x] Attempt to manually construct `Witness::new()` outside a `@proof` function.
- [x] **Safety Bypass**:
    - [x] Use `raw` to create an invalid `NonZero<i32>(0)` and ensure subsequent `divide` panics correctly or is caught by sanitizers.

### 3.2 Temporal Constraints
- [x] **Pre-condition Violations**:
    - [x] Call `factorial(-1)` with a constant (compile error).
    - [x] Call `factorial(n)` where `n < 0` at runtime (runtime panic).
- [x] **Post-condition Verification**:
    - [x] A function that returns a value outside its `@ensures` range should fail verification.

### 3.3 Memory Zone Isolation
- [x] **Leak Test**: Allocate 1,000,000 small objects in a `zone` and monitor RSS after the block ends.
- [x] **Escape Test**:
    - [x] Assign a reference to a zone-allocated `String` to a variable declared outside the `zone`.
    - [x] Pass a zone-allocated slice to a `tide` background flow (must fail if flow outlives zone).

### 3.4 Cascade Error Chains
- [x] **Propagation Depth**: Verify a 10-level deep `!` propagation produces a 10-node context chain.
- [x] **Message Overrides**: Ensure `result! or "custom"` correctly replaces the default context at that level.

### 3.5 Duality Check
- [x] **Round-trip Accuracy**:
    - [x] `JsonValue` -> `Shape` -> `JsonValue` comparison.
    - [x] Large/Nested `dual shape` structures (e.g., AST nodes).

---

## 4. Performance & Backend Tests

### 4.1 Optimization Passes
- [x] **Pipeline Fusion Check**:
    - [x] Inspect LLVM IR for `iter |> map |> filter |> collect` to ensure no intermediate heap allocations occur.
- [x] **TCO Success**:
    - [x] Recursively call a `pure forge` 10,000,000 times to verify no stack overflow.

### 4.2 Code Generation Correctness
- [x] **FFI Interop**:
    - [x] Pass an Izel `shape` to C `memcpy` and back.
    - [x] Handle null pointers returned from C `malloc` within a `raw` block.
- [x] **Floating Point**:
    - [x] Verify IEEE 754 compliance for `f32` and `f64` arithmetic across different targets.

---

## 5. Toolchain Verification

### 5.1 `izel-pm` (Package Manager)
- [x] **Dependency Resolution**:
    - [x] Construct a tree with diamond dependencies and verify version unification.
    - [x] Test offline build cache behavior.

### 5.2 `izel-fmt`
- [x] **Idempotency**: `izel fmt` run twice on the same file should change 0 bytes.
- [x] **Correctness**: Ensure formatter never changes code semantics (verified by compiling before and after).

### 5.3 `izel-lsp`
- [x] **Go-to-Definition**: Cross-module jumps for types and methods.
- [x] **Real-time Diagnostics**: Ensure error markers appear within 100ms of a syntax error being typed.
