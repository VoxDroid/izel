# 1. Getting Started

This chapter takes you from a fresh clone to running an Izel source file.

## Prerequisites

- Rust toolchain (workspace default toolchain).
- LLVM 17 toolchain components for codegen-related crates.
- Standard build tools (`clang`, `lld`, `cmake`, `zlib`).

macOS quick setup:

```bash
brew install llvm@17 cmake
```

You can validate environment health with:

```bash
bash tools/ci/check_system_deps.sh
```

## Build The Workspace

```bash
bash tools/ci/with_llvm_env.sh cargo check --workspace --all-targets
bash tools/ci/with_llvm_env.sh cargo test --workspace
```

## Try The Compiler Entry Point

Compile-mode path (front-end pipeline + LLVM IR emission):

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- examples/hello.iz
```

Format mode:

```bash
cargo run -p izel_driver -- fmt examples/hello.iz
```

## Try The Package Manager Entry Point

```bash
cargo run -p izel_pm -- new demo --bin
```

This creates:
- `demo/Izel.toml`
- `demo/src/main.iz`

Additional accepted command surfaces:
- `build`, `run`, `test`, `bench`, `check`, `fmt`, `lint`, `doc`, `add`,
  `remove`, `update`, `publish`, `clean`, `tree`, `audit`

Some commands are intentionally scaffold-level while implementation wiring evolves.

## First Program

`src/main.iz`:

```izel
forge main() -> i32 {
	42
}
```

## Development Loop

```bash
bash tools/ci/with_llvm_env.sh cargo test --workspace
pre-commit run --all-files
```

Proceed to ownership next. Almost every advanced Izel feature depends on understanding moves and
borrows first.
