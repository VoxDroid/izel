# Izel Playground (WASM Browser REPL)

This directory contains the browser playground that runs the Izel frontend pipeline in WebAssembly.

## Components

- `wasm/`: Rust-to-WASM bridge module exposed to JavaScript.
- `index.html`, `main.js`, `styles.css`: browser REPL host UI.
- `pkg/`: generated wasm-bindgen browser package output (created by build scripts).

## Build

```bash
cd tools/playground
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
npm run build:wasm
```

## Run Locally

```bash
cd tools/playground
npm run serve
```

Then open http://localhost:4173.

The Run action (or Cmd/Ctrl+Enter) performs two stages:

1. Frontend validation in WASM (tokenize, parse, lower, typecheck).
2. Runtime execution through a local API endpoint (`POST /api/run`) served by `server.js`, which runs `izel_driver --run` on a temporary source file.

The runtime path supports string literals, so `println("hello")` style examples execute end-to-end.
Escape sequences in string literals are normalized during codegen (for example `\n`, `\t`, `\x41`, `\u{1F600}`).

If you want frontend-only behavior, use:

```bash
cd tools/playground
npm run serve:static
```
