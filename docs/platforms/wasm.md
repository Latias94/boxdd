# WASM Status

`boxdd` now has a checked browser-provider runtime path for `wasm32-unknown-unknown`. The Rust module imports Box2D C API symbols from an Emscripten-built provider module, and both modules use the same `WebAssembly.Memory`.

## Supported workflows

| Surface | Target | Status | Command |
| --- | --- | --- | --- |
| `boxdd-sys` compile smoke | `wasm32-unknown-unknown` | compile-only by default | `cargo check -p boxdd-sys --target wasm32-unknown-unknown` |
| Provider runtime | `wasm32-unknown-unknown` + Emscripten provider | Node and browser smoke | `cargo run -p xtask -- provider-smoke` |
| GitHub Pages runtime | browser | generated live canvas plus source/example index | `cargo run -p xtask -- build-pages-wasm` |
| WASI smoke | `wasm32-wasip1` | check-only unless a WASI SDK source build is configured | `cargo check -p boxdd-sys --target wasm32-wasip1` |

## Provider runtime

The browser runtime lives in `examples-wasm/provider-smoke`. It validates world stepping, ray casts, shape casts, distance joints, and a small live scene that `docs/pages/wasm/index.html` draws on canvas.

```powershell
rustup target add wasm32-unknown-unknown

# Requires emcc on PATH or EMSDK pointing at an activated Emscripten SDK.
cargo run -p xtask -- provider-smoke-app
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- build-pages-wasm
cargo run -p xtask -- validate-pages
```

`provider-smoke-app` builds the Rust wasm module with `BOXDD_SYS_WASM_MODE=provider` and records the exact `b2*` imports it needs from `box2d-sys-v0`. `provider-smoke` additionally builds `box2d-sys-v0.js`/`.wasm` with Emscripten and runs the shared-memory smoke under Node. `build-pages-wasm` publishes the same provider module, Rust wasm module, and generated loader under `docs/pages/wasm/generated`.

## Build modes

`boxdd-sys` uses `BOXDD_SYS_WASM_MODE` to choose the raw FFI strategy on wasm targets:

- `compile-only`: default for `wasm32-unknown-unknown`; uses pregenerated bindings and skips compiling/linking Box2D C so safe Rust APIs can type-check.
- `provider`: imports Box2D symbols from the `box2d-sys-v0` wasm import module. This is the browser runtime mode used by `examples-wasm/provider-smoke` and Pages.
- `source`: compiles vendored Box2D C for wasm when an explicit C toolchain path is available. `BOXDD_SYS_WASM_CC=1` opts into this mode for `wasm32-unknown-unknown`, and `wasm32-wasip1` uses `WASI_SDK_PATH` when set.

## Known boundaries

The provider runtime intentionally avoids Rust callback transport across the Emscripten provider's function table. World stepping, queries, collision helpers, shape/joint operations, and readback paths are covered; debug draw callbacks and user callbacks should still be validated through native targets unless a dedicated callback bridge is added.

## Bevy adapter

`bevy_boxdd` is designed as a Bevy ECS adapter and does not currently claim browser support. It should be checked separately from `boxdd` because Bevy platform support, renderer features, and wasm-bindgen packaging dominate compile behavior.
