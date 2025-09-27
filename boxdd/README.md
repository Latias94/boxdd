boxdd — Safe, ergonomic Rust bindings for Box2D (v3 C API)

Overview
- Thin, modular safe layer over the official Box2D v3 C API.
- Ergonomics: builder patterns, world-space joint helpers, rich query/events, debug draw.
- Mint interop: accept mint::Vector2<f32>/Point2<f32> and arrays/tuples where vectors are needed.
- Two usage styles: RAII wrappers or ID-based APIs.

Bindless (ID 风格)
- ID 风格通过 `BodyId/ShapeId/JointId` 直接操作世界对象，避免 Rust 借用/析构顺序带来的约束，适合在 ECS、脚本或长期存储句柄的场景。
- 本仓库示例优先展示 ID 风格，外层接口不暴露任何 `ffi` 类型；需要 FFI 的场景请使用 `boxdd-sys`。

Quickstart (RAII)
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def)?;

let mut body = world.create_body(BodyBuilder::new().position([0.0, 2.0]).build());
let sdef = ShapeDef::builder().density(1.0).build();
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = body.create_polygon_shape(&sdef, &poly);

world.step(1.0/60.0, 4);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Quickstart (ID-style)
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def)?;

let body = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
let sdef = ShapeDef::builder().density(1.0).build();
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = world.create_polygon_shape_for(body, &sdef, &poly);

world.step(1.0/60.0, 4);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Mint interop
- Any parameter typed as `Into<b2Vec2>` can take `mint::Vector2<f32>`, `mint::Point2<f32>`, `[f32; 2]`, `(f32, f32)`.
- Convert results back with `mint` via `From`.

Modules
- `world`, `body`, `shapes`, `joints`, `query`, `events`, `debug_draw`, `prelude`.
- Import `boxdd::prelude::*` for a convenient surface of common types.

Events
- `*_events()` methods return owned vectors of snapshot data.
- `with_*_events` variants offer zero-copy access; the slices are valid only for the closure and current step.

FFI hygiene
- 对外（boxdd）所有公共 API 均为安全封装，不直接暴露 `boxdd-sys::ffi` 类型。
- 仅在 `boxdd-sys` 层提供底层绑定与生成的头文件类型。

License
- MIT or Apache-2.0.
