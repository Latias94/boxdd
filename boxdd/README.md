# boxdd — Safe, ergonomic Rust bindings for Box2D v3

Overview
- Thin, modular safe layer over the official Box2D v3 C API.
- Ergonomics: builder patterns, world-space joint helpers, rich queries/events, debug draw.
- Mint interop: accept `mint::Vector2<f32>` / `mint::Point2<f32>` and arrays/tuples.
- Two usage styles: RAII wrappers or ID-based APIs (bindless).

Quickstart (RAII)
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def).unwrap();

let mut body = world.create_body(BodyBuilder::new().position([0.0, 2.0]).build());
let sdef = ShapeDef::builder().density(1.0).build();
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = body.create_polygon_shape(&sdef, &poly);

world.step(1.0/60.0, 4);
```

Quickstart (ID-style)
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def).unwrap();

let body = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
let sdef = ShapeDef::builder().density(1.0).build();
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = world.create_polygon_shape_for(body, &sdef, &poly);

world.step(1.0/60.0, 4);
```

Mint interop
- Any parameter typed as `Into<Vec2>` accepts `mint::Vector2<f32>`, `mint::Point2<f32>`, `[f32; 2]`, `(f32, f32)`.
- Convert results back with `From`.

Modules
- `world`, `body`, `shapes`, `joints`, `query`, `events`, `debug_draw`, `prelude`.
- Import `boxdd::prelude::*` for a convenient surface of common types.

Events
- `*_events()` methods return owned vectors of snapshot data.
- `with_*_events` variants offer zero-copy access valid only for the closure call and current step.

Debug Draw
- Implement the `DebugDraw` trait or use `RawDebugDraw` for zero-copy.
- Call `World::debug_draw` each step with `DebugDrawOptions`.

License
- MIT OR Apache-2.0.

