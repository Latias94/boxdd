# WASM Status

`boxdd-sys` can be checked for `wasm32-unknown-unknown` and `wasm32-wasip1`, but the workspace intentionally ships a generated source/example Pages index before a live browser demo.

## Current support target

- Core FFI smoke: `cargo check -p boxdd-sys --target wasm32-unknown-unknown`
- WASI smoke: `cargo check -p boxdd-sys --target wasm32-wasip1`
- Example smoke: `boxdd/examples/wasm_wasi_smoke.rs`

## Deferred browser demo

A full browser demo needs a separate audit for:

- callback behavior under wasm builds
- allocator and panic reporting strategy
- debug draw transport to canvas
- deterministic fixed timestep inside `requestAnimationFrame`
- asset packaging for GitHub Pages

Until that work is complete, `docs/pages` is generated from checked-in examples and links to concrete source files plus run commands instead of pretending those examples are browser-runnable.

## Bevy adapter

`bevy_boxdd` is designed as a Bevy ECS adapter and does not currently claim browser support. It should be checked separately from `boxdd` because Bevy platform support and feature selection dominate compile behavior.
