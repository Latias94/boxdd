# WASM Status

`boxdd-sys` can be checked for `wasm32-unknown-unknown` and `wasm32-wasip1`, but the workspace intentionally ships a static Pages hub before a live browser demo.

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

Until that work is complete, `docs/pages` stays static and links to source examples plus coverage matrices.

## Bevy adapter

`bevy_boxdd` is designed as a Bevy ECS adapter and does not currently claim browser support. It should be checked separately from `boxdd` because Bevy platform support and feature selection dominate compile behavior.
