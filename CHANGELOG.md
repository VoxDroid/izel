# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Expanded book chapters under `docs/book/` with complete introductory coverage.
- Expanded normative and compatibility content under `docs/spec/`.
- Added concrete compile-pass/compile-fail fixtures replacing empty keep-files.
- Added broader `izel_pm` parser and CLI branch coverage tests.
- Added LLVM codegen support for MIR string constants and string primitive lowering.
- Added runtime intrinsics for string printing (`io_print_str`) and integer to string conversion (`i32_to_str`).
- Added runtime string ownership cleanup intrinsic (`str_free`) and std helper wiring (`free_str`).
- Added codegen regression tests for string escape decoding and `str_free` intrinsic coverage.

### Changed
- Replaced transitional dual round-trip test body generation with an empty, valid body.
- Type checker now records inferred expression types in `expr_types`.
- MIR codegen now emits LLVM phi handling instead of no-op fallback behavior.
- Updated runtime `std/io` to expose string-first `println` and integer helper `println_int`.
- Playground runtime/docs now reflect end-to-end support for string literal `println` execution.
- Runtime string literal lowering now decodes escape sequences (for example `\n`, `\t`, `\xNN`, `\u{...}`).
- Expanded `library/std/io` declarations to better match executable runtime std io APIs.

