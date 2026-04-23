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

## Explore The 001-100 Sample Applications

The repository includes a large practical suite under `sample_applications/`.

Compile-check one application:

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- sample_applications/001_budget_forecast_calculator.iz
```

Compile-check all applications:

```bash
for f in sample_applications/[0-9][0-9][0-9]_*.iz; do
	bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- "$f" || break
done
```

For tutorial sequencing and category guidance, read:
- `sample_applications/README.md`
- `sample_applications/TUTORIAL.md`

Runtime support for control-flow execution is under active expansion. `while` loops are covered by
runtime integration tests, while broader `loop`/`each` lowering continues to expand; the compile-first
workflow above remains the most reliable way to validate the full suite today.

Proceed to ownership next. Almost every advanced Izel feature depends on understanding moves and
borrows first.
