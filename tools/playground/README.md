# Izel Playground (WASM Browser REPL)

This directory contains the browser playground scaffold for Phase 7.

## Components

- `wasm/`: Rust-to-WASM bridge module exposed to JavaScript.
- `index.html`, `main.js`, `styles.css`: browser REPL host UI.

## Build

```bash
cd tools/playground/wasm
cargo build --target wasm32-unknown-unknown
```

The JS host expects a wasm-bindgen-compatible package output in `tools/playground/pkg/`.
