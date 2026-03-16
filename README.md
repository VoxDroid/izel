# ⬡ IZEL — *Only One*

**Izel** (from Nahuatl: *izel* — "unique", "one of a kind", "only one") is a compiled, multi-paradigm systems programming language built entirely in Rust.

## Vision
To create a systems programming language that gives developers absolute control over hardware while introducing novel constructs like **Effect Systems**, **Witness Types**, and **Memory Zones**.

## Current Status: Foundation Scaffolding
The project is currently in **Phase 0** of its [Implementation Checklist](docs/checklist.md).
- [x] Workspace Initialization
- [x] Core Infrastructure (`izel_span`, `izel_diagnostics`, `izel_session`)
- [x] Minimal Lexer/Parser shells
- [x] Compiler Driver (`izelc`)

## Getting Started

### Prerequisites
- Rust (latest stable)
- LLVM 17 (required for `inkwell`)

### Build
```bash
cargo build --workspace
```

### Run Compiler
```bash
cargo run --bin izelc -- examples/hello.iz
```

## Contributing
Please see the [Project Overview](docs/project_overview.md) for the full language specification and design philosophy.

---
**Creator:** VoxDroid (@VoxDroid)  
**Email:** izeno.contact@gmail.com  
**License:** MIT
