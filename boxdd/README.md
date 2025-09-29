<div align="center">

# boxdd - Rust bindings for Box2D v3 (C API)

[![Crates.io](https://img.shields.io/crates/v/boxdd.svg?style=flat-square)](https://crates.io/crates/boxdd)
[![Docs](https://docs.rs/boxdd/badge.svg)](https://docs.rs/boxdd)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](#license)

![boxdd](https://raw.githubusercontent.com/Latias94/boxdd/main/screenshots/boxdd.gif)

</div>

## Crates
- `boxdd-sys`: low-level FFI for the official Box2D v3 C API (vendored)
- `boxdd`: safe layer (world, bodies, shapes, joints, queries, events, debug draw)

## Highlights
- Safe, ergonomic Rust wrapper over the official Box2D v3 C API.
- Mint interop by default: any `Into<Vec2>` accepts `mint::Vector2<f32>`, `mint::Point2<f32>`, arrays/tuples.

## Quickstart
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def).unwrap();
let mut body = world.create_body(BodyBuilder::new().position([0.0, 2.0]).build());
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = body.create_polygon_shape(&ShapeDef::default(), &poly);
world.step(1.0/60.0, 4);
```

## Features (optional)
- `serde`: serialization for `Vec2`, `Rot` (radians), `Transform` ({ pos, angle }) and config types.
- `serialize`: snapshot helpers (save/apply world config; take/rebuild minimal full-scene snapshot).
- `cgmath`, `nalgebra`, `glam`: conversions with their 2D types (e.g. `Vector2/Point2`, `UnitComplex/Isometry2`, `glam::Vec2`).

## Snapshots
- Enable `serialize` and see example `examples/scene_serialize.rs` for a minimal scene round-trip.
- Note: chain shapes are captured when created via ID-style API (`World::create_chain_for_id`).

## Build Modes
- Default (from source): builds vendored Box2D C sources via `cc` and generates bindings with bindgen.
  - Example: `cargo build -p boxdd` (or explicitly `--features build-from-source`).
- Prebuilt: link a precompiled static library instead of building C sources.
  - Example: `cargo build -p boxdd --features prebuilt`.
  - Library directory: set `BOX2D_LIB_DIR=/path/to/lib` (contains `libbox2d.a` or `box2d.lib`). Without it, the system linker search is used.

## Getting Started

```bash
git submodule update --init --recursive
cargo build

# run some examples
cargo r --example world_basics
cargo r --example joints
cargo r --example queries
cargo r --example sensors
cargo r --example testbed_imgui_glow --features imgui-glow-testbed
```

## Examples
- The `examples/` folder covers worlds/bodies/shapes, joints, queries/casts, events/sensors, CCD, and debug draw.

## Notes
- Vendored C sources + bindgen (requires a C toolchain and libclang). On Windows/MSVC, set `LIBCLANG_PATH` if needed.
- Optional: link a prebuilt static library via `BOX2D_LIB_DIR=/path/to/lib`
- On docs.rs, the native build is skipped

## Documentation
- Local: `cargo doc --open`
- Online: https://docs.rs/boxdd

## Acknowledgments
- Thanks to the Rust Box2D bindings project for prior art and inspiration: https://github.com/Bastacyclop/rust_box2d
- Huge thanks to the upstream Box2D project by Erin Catto: https://github.com/erincatto/box2d

## Related Projects

If you're working with graphics applications in Rust, you might also be interested in:

- **[asset-importer](https://github.com/Latias94/asset-importer)** - A comprehensive Rust binding for the latest [Assimp](https://github.com/assimp/assimp) 3D asset import library, providing robust 3D model loading capabilities for graphics applications
- **[dear-imgui](https://github.com/Latias94/dear-imgui)** - Comprehensive Dear ImGui bindings for Rust using C++ bindgen, providing immediate mode GUI capabilities for graphics applications

## License
- `boxdd`: MIT OR Apache-2.0
- `boxdd-sys`: MIT OR Apache-2.0
