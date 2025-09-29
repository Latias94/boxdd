//! Minimal smoke test for WASM (WASI) build.
//!
//! Build:
//! - WASI (no emsdk):
//!   - rustup target add wasm32-wasi
//!   - Set WASI_SDK_PATH to your wasi-sdk root (e.g. C:\\wasi-sdk-22.0)
//!   - cargo build -p boxdd --example wasm_wasi_smoke --target wasm32-wasi
//!
//! Run (requires a WASI runtime, e.g. wasmtime/wasmer):
//!   wasmtime target/wasm32-wasi/debug/examples/wasm_wasi_smoke.wasm

use boxdd::{World, WorldDef};

fn main() {
    // Create a world with gravity and run a few steps.
    let def = WorldDef::builder().gravity([0.0_f32, -9.8]).build();
    let mut world = World::new(def).expect("create world");

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    let c = world.counters();
    println!(
        "WASM/WASI smoke: bodies={}, shapes={}, contacts={}, joints={}, islands={}",
        c.body_count, c.shape_count, c.contact_count, c.joint_count, c.island_count
    );
}
