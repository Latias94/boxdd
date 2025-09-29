# boxdd — Rust bindings for Box2D v3

- Safe, ergonomic layer over the official Box2D v3 C API.
- Mint interop by default: any `Into<Vec2>` accepts `mint::Vector2<f32>`, `mint::Point2<f32>`, arrays/tuples.

Quickstart
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def).unwrap();
let mut body = world.create_body(BodyBuilder::new().position([0.0, 2.0]).build());
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = body.create_polygon_shape(&ShapeDef::default(), &poly);
world.step(1.0/60.0, 4);
```

Features (optional)
- `serde`: serialization for `Vec2`, `Rot` (radians), `Transform` ({ pos, angle }) and config types.
- `serialize`: snapshot helpers (save/apply world config; take/rebuild minimal full-scene snapshot).
- `cgmath`, `nalgebra`, `glam`: conversions with their 2D types (e.g. `Vector2/Point2`, `UnitComplex/Isometry2`, `glam::Vec2`).

Snapshots
- Enable `serialize` and see example `examples/scene_serialize.rs` for a minimal scene round-trip.
- Note: chain shapes are captured when created via ID-style API (`World::create_chain_for_id`).

Build modes
- Default: from source. Builds vendored Box2D via `cc`, generates bindings with bindgen.
  - `cargo build -p boxdd` (or explicitly `--features build-from-source`).
- Prebuilt: link a precompiled static library instead of building C sources.
  - `cargo build -p boxdd --features prebuilt`.
  - Set `BOX2D_LIB_DIR=/path/to/lib` (contains `libbox2d.a` or `box2d.lib`). Without it, the system linker search is used.

Notes
- Requires a C toolchain and libclang for bindgen (LLVM). On Windows/MSVC, set `LIBCLANG_PATH` if needed.
