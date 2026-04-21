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
- Added stderr-backed runtime intrinsic for string error printing (`io_eprint_str`) used by `eprintln`.
- Added dedicated runtime IO integration snapshots for stdout/stderr stream separation in `izel_driver` tests.
- Added `sample_applications/` with 100 numbered practical Izel app examples (`001`-`100`) and tutorial docs.
- Added runtime `std/tui` and declaration `library/std/tui` surfaces for terminal dashboard-style application output.
- Added runtime `std/io` intrinsics for stdin and files: `io_read_stdin`, `io_read_file`, `io_write_file`.
- Added runtime IO integration coverage for file write/read roundtrip behavior in `izel_driver` tests.
- Added runtime `std/io` file utility intrinsics: `io_append_file`, `io_remove_file`, `io_file_exists`, and `io_list_dir`.
- Added runtime `std/io` numeric stdin parsing intrinsics: `io_read_stdin_int` and `io_read_stdin_float`.
- Added runtime IO integration coverage for append/exists/remove/listing and numeric stdin parsing paths.
- Added runtime `std/io` status intrinsic `io_last_status` and boolean helper intrinsic `io_file_exists_bool`.
- Added explicit runtime IO missing-path and invalid numeric parse integration snapshots.
- Added runtime `std/io` error-kind intrinsic `io_last_error_kind` with normalized status categories.
- Added runtime `std/io` structured listing and binary-safe hex intrinsics: `io_list_dir_structured`, `io_read_file_bytes_hex`, and `io_write_file_bytes_hex`.
- Added runtime IO integration snapshots for try helpers, normalized error kinds, structured listings, bytes-hex IO, stress append workloads, and cross-platform path separators.
- Added compile-pass std IO fixtures for status helpers and try helper surfaces.

### Changed
- Replaced transitional dual round-trip test body generation with an empty, valid body.
- Type checker now records inferred expression types in `expr_types`.
- MIR codegen now emits LLVM phi handling instead of no-op fallback behavior.
- Updated runtime `std/io` to expose string-first `println` and integer helper `println_int`.
- Playground runtime/docs now reflect end-to-end support for string literal `println` execution.
- Runtime string literal lowering now decodes escape sequences (for example `\n`, `\t`, `\xNN`, `\u{...}`).
- Expanded `library/std/io` declarations to better match executable runtime std io APIs.
- Expanded executable/declaration std IO surfaces with `read_stdin()`, `read_file(path)`, and `write_file(path, content)`.
- Expanded executable/declaration std IO surfaces with `append_file(path, content)`, `remove_file(path)`, `file_exists(path)`, `list_dir(path)`, `read_stdin_int()`, and `read_stdin_float()`.
- Expanded executable/declaration std IO and fs surfaces with status-first `try_*` helpers, structured directory listing helpers, and bytes-hex file helpers.
- Runtime IO execution now uses native Rust helper symbols for dynamic stdin/file reads and directory listing (no shell command composition).
- Runtime documentation now includes a `std/io` cookbook covering status-first flows and binary-safe hex roundtrips.
- MIR let-lowering now infers local types from initializer expressions when metadata is missing, preventing unsafe str temporary lowering.
- Mixed int/float binop lowering now promotes integers to `f64` for arithmetic/comparison compatibility.
- Parser condition parsing now preserves full `given`/`while` boolean expressions while correctly respecting following blocks.
- AST and MIR lowering now preserve control-flow nodes (`while`, `loop`, `each`) for ongoing runtime support expansion.

