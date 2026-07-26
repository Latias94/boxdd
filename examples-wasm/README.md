# WASM Examples

This directory contains low-level WASM provider smoke code that is built through the Box2D provider runtime instead of the native example runner. User-facing browser examples live in `bevy_boxdd/examples/testbed_2d` and are published through `docs/pages`.

## Browser Provider Runtime

Browser builds use `BOXDD_SYS_PROVIDER=wasm-provider`. In that mode the Rust wasm
module imports Box2D C API symbols from an Emscripten-built provider module named
`box2d-sys-v1-single` (or `box2d-sys-v1-double` for the double-precision route), and
both modules share one `WebAssembly.Memory`.

```bash
rustup target add wasm32-unknown-unknown
cargo run -p xtask -- provider-smoke-app

# Requires the pinned Emscripten 6.0.3 SDK identity in `boxdd-sys/emscripten-sdk.toml`.
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- verify-wasm --runtime
cargo run -p xtask -- build-pages-wasm
npm run test:pages-browser
```

The smoke runtime verifies world stepping, closest ray casts, standalone collision helpers, distance joints, and fail-closed rejection of raw task/material callbacks without relying on cross-module callback tables. Rust callback registration, callback-backed world and recording-session queries, dynamic-tree traversal, and debug draw are compile-time unavailable on `wasm32` until that table ABI has its own runtime proof. `verify-wasm --compile-only` includes negative probes for those boundaries. `build-pages-wasm` reuses the provider path, then builds the Bevy + egui testbed and writes browser assets under `docs/pages/wasm/generated` and `docs/pages/bevy-testbed/generated`; `test:pages-browser` runs that exact generated testbed in Chromium, including a shared-memory growth and post-growth physics-step proof.
