# ⬡ IZEL — *Only One*

**Izel** (from Nahuatl: *izel* — "unique", "one of a kind", "only one") is a compiled, multi-paradigm systems programming language built entirely in Rust.

## Vision
To create a systems programming language that gives developers absolute control over hardware while introducing novel constructs like **Effect Systems**, **Witness Types**, and **Memory Zones**.

## Current Status: Active Multi-Phase Prototype
The repository now contains a working multi-crate compiler/tooling pipeline (lexer, parser,
resolver, AST lowerer, type checker, borrow checker, MIR, LLVM codegen, formatter, LSP, package
manager). See [Implementation Checklist](docs/checklist.md) for roadmap-level tracking and
[Tests Checklist](docs/tests_checklist.md) for validation depth.

## Getting Started

### Prerequisites
- Rust (latest stable)
- LLVM 17 (required for `inkwell`/`llvm-sys`)
- CMake 3.20+

On macOS:

```bash
brew install llvm@17 cmake
```

Verify dependencies:

```bash
bash tools/ci/check_system_deps.sh
```

### Build
```bash
bash tools/ci/with_llvm_env.sh cargo build --workspace
```

### Run Compiler
```bash
bash tools/ci/with_llvm_env.sh cargo run --bin izelc -- examples/hello.iz
```

## Contributing
Please see the [Project Overview](docs/project_overview.md) for the full language specification and design philosophy.

---
**Creator:** VoxDroid (@VoxDroid)  
**Email:** izeno.contact@gmail.com  
**License:** MIT
