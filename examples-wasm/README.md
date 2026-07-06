# WASM Examples

This directory contains browser-oriented WASM examples that are built through the
Box2D provider runtime instead of the native example runner.

## Browser Provider Runtime

Browser builds use `BOXDD_SYS_WASM_MODE=provider`. In that mode the Rust wasm
module imports Box2D C API symbols from an Emscripten-built provider module named
`box2d-sys-v0`, and both modules share one `WebAssembly.Memory`.

```bash
rustup target add wasm32-unknown-unknown
cargo run -p xtask -- provider-smoke-app

# Requires Emscripten SDK (`emcc`) on PATH or EMSDK set.
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- build-pages-wasm
```

The smoke runtime verifies world stepping, closest ray casts, standalone
collision helpers, and distance joints without relying on cross-module callback
tables.
