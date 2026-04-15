# IZEL

Izel is a compiled systems programming language implemented in Rust, with first-class effect tracking, witness types, and zone-based memory semantics.

This repository contains the compiler, toolchain, standard library wards, language server, formatter, package manager, and playground.

## Repository Layout

- `crates/`: compiler and toolchain crates.
- `examples/`: runnable Izel source examples.
- `library/std/` and `std/`: standard library ward sources.
- `tests/`: integration, compile-pass/fail, and feature-specific fixtures.
- `docs/`: book chapters, reference/spec content, and project overviews.
- `tools/`: grammar, playground, CI helpers, bootstrap utilities.

## Prerequisites

- Rust toolchain (workspace default in `rust-toolchain.toml`).
- LLVM 17 (`llvm-config`, `clang`, `lld`) for LLVM-backed codegen crates.
- `cmake` for native dependency builds.

macOS setup:

```bash
brew install llvm@17 cmake
```

Validate system dependencies:

```bash
bash tools/ci/check_system_deps.sh
```

## Build

```bash
bash tools/ci/with_llvm_env.sh cargo build --workspace
```

## Compile and Run Izel Code

Compile through the driver frontend/codegen path:

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- examples/hello.iz
```

Format an Izel source file:

```bash
cargo run -p izel_driver -- fmt examples/hello.iz
```

Run the package manager entrypoint:

```bash
cargo run -p izel_pm -- new demo --bin
cd demo
../target/debug/izel build
```

If `izel` is installed on your PATH, use `izel build` instead of the relative binary path.

## Validation Commands

Use these before release or pull requests:

```bash
pre-commit run --all-files
bash tools/ci/with_llvm_env.sh cargo check --workspace --all-targets
bash tools/ci/with_llvm_env.sh cargo test --workspace
cargo fmt --all -- --check
bash tools/ci/with_llvm_env.sh cargo clippy --workspace --all-targets -- -D warnings
```

## Example Programs

Feature-focused examples live under `examples/`, including:

- `hello.iz`
- `effects_valid.iz`
- `contracts_valid.iz`
- `witness_valid.iz`
- `zones_valid.iz`
- `temporal_constraints.iz`

## Browser Playground (WASM)

Build and run the playground:

```bash
cd tools/playground
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
npm install
npm run build:wasm
npm run serve
```

Then open `http://localhost:4173`.

The playground Run action now performs frontend validation in WASM and then executes through a local runtime endpoint (`/api/run`) backed by `izel_driver --run`.
This runtime path supports string literals, so programs like `println("Hello from Izel")` execute in the playground.

Runtime note: `to_str(int)` returns an owned runtime string buffer.
Use `free_str(...)` after use (the std `println_int(...)` helper already handles this cleanup).

For frontend-only static serving (no runtime execution), use:

```bash
cd tools/playground
npm run serve:static
```

## Syntax Highlighting (VS Code)

An installable VS Code package is provided at `tools/grammar/vscode-izel`.

Build a VSIX locally:

```bash
cd tools/grammar/vscode-izel
npm install
npm run package
```

Install in VS Code:

```bash
code --install-extension <generated-file>.vsix
```

## Documentation

- Book: `docs/book/`
- Reference/spec: `docs/reference/`, `docs/spec/`
- Project overview: `docs/project_overview.md`
- Verification checklists: `docs/po_checklist.md`, `docs/tests_checklist.md`

## License

MIT.
