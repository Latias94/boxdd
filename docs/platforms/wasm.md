# WASM Status

`boxdd` now has a checked browser-provider runtime path for `wasm32-unknown-unknown`. The Rust module imports Box2D C API symbols from an Emscripten-built provider module, and both modules use the same `WebAssembly.Memory`.

## Supported workflows

| Surface | Target | Status | Command |
| --- | --- | --- | --- |
| `boxdd-sys` compile smoke | `wasm32-unknown-unknown` | compile-only by default | `cargo check -p boxdd-sys --target wasm32-unknown-unknown` |
| Provider runtime | `wasm32-unknown-unknown` + Emscripten provider | Node smoke plus Pages provider assets | `cargo run -p xtask -- provider-smoke` |
| GitHub Pages runtime | browser | generated Bevy + egui example index with one runnable scene route per official sample-style port | `cargo run -p xtask -- build-pages-wasm` |
| WASI smoke | `wasm32-wasip1` | check-only unless a WASI SDK source build is configured | `cargo check -p boxdd-sys --target wasm32-wasip1` |

## Provider runtime

The low-level provider smoke lives in `examples-wasm/provider-smoke`. It validates world stepping, ray casts, shape casts, and distance joints without requiring a browser page. The user-facing browser runtime is the Bevy + egui testbed in `bevy_boxdd/examples/testbed_2d`, published as generated routes under `docs/pages/examples/<scene-id>/`.

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126 --locked

# Requires emcc on PATH or EMSDK pointing at an activated Emscripten SDK.
cargo run -p xtask -- provider-smoke-app
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- build-pages-wasm
cargo run -p xtask -- validate-pages
```

`provider-smoke-app` builds the Rust wasm module with `BOXDD_SYS_WASM_MODE=provider` and records the exact `b2*` imports it needs from `box2d-sys-v0`. `provider-smoke` additionally builds `box2d-sys-v0.js`/`.wasm` with Emscripten and runs the shared-memory smoke under Node. `build-pages-wasm` runs `generate-pages`, compiles the Bevy testbed wasm, patches the wasm-bindgen import glue to use the provider shim, builds the Emscripten provider exports required by that Bevy wasm, and publishes assets under both `docs/pages/wasm/generated` and `docs/pages/bevy-testbed/generated`.

## Build modes

`boxdd-sys` uses `BOXDD_SYS_WASM_MODE` to choose the raw FFI strategy on wasm targets:

- `compile-only`: default for `wasm32-unknown-unknown`; uses pregenerated bindings and skips compiling/linking Box2D C so safe Rust APIs can type-check.
- `provider`: imports Box2D symbols from the `box2d-sys-v0` wasm import module. This is the wasm mode used by `examples-wasm/provider-smoke` and the Bevy Pages runtime.
- `source`: compiles vendored Box2D C for wasm when an explicit C toolchain path is available. `BOXDD_SYS_WASM_CC=1` opts into this mode for `wasm32-unknown-unknown`, and `wasm32-wasip1` uses `WASI_SDK_PATH` when set.

## Known boundaries

The provider runtime intentionally avoids Rust callback transport across the Emscripten provider's function table. World stepping, queries, collision helpers, shape/joint operations, events, and readback paths are covered; debug draw callbacks and user callbacks should still be validated through native targets unless a dedicated callback bridge is added.

## Bevy adapter

`bevy_boxdd` now has a browser-tested example path through `bevy_boxdd/examples/testbed_2d`. Treat that as the supported Pages/demo configuration: Bevy Web + egui + `BOXDD_SYS_WASM_MODE=provider` + the generated provider shim. General Bevy browser support should still be checked per application because renderer features and asset packaging dominate compile behavior.
