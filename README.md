# boxdd — Rust bindings for Box2D v3 (C API)

![boxdd](screenshots/boxdd.gif)

Crates
- `boxdd-sys`: low-level FFI built from the official Box2D v3 C API (vendored submodule, built with `cc`, or linked to a prebuilt static library via env var).
- `boxdd`: safe, ergonomic layer providing modular APIs for world, bodies, shapes, joints, queries, events, and debug draw.

Getting started
- Initialize submodules
  - `git submodule update --init --recursive`
- Build
  - `cargo build`
- Run examples (optional)
  - `cargo r --example testbed_imgui_glow --features imgui-glow-testbed`
  - `cargo r --example joints`

Build modes
- Default: vendored source + bindgen
  - `boxdd-sys` compiles the C sources under `boxdd-sys/third-party/box2d/src` and generates Rust bindings from headers under `.../include/` using bindgen.
  - Requires a working C toolchain and libclang.
- Docs.rs: bindings only
  - On docs.rs, the build script skips compiling the C library and only runs bindgen so rustdoc can build API docs without linking.
- Prebuilt static library (optional)
  - Set `BOX2D_LIB_DIR=/path/to/lib` to link against an existing `box2d` static library and skip compiling the vendored sources.
  - Ensure the prebuilt library matches your target (ABI, CRT, architecture) and the vendored Box2D version.

Pregenerated bindings (docs.rs / offline)
- We commit pregenerated bindings so docs.rs can build without fetching submodules or needing libclang.
- To refresh pregenerated bindings and update the submodule, run:
  - `python tools/update_submodule_and_bindings.py --profile release`
- This writes `boxdd-sys/src/bindings_pregenerated.rs`. Commit that file with your release.
- Tip: the script sets `BOXDD_SYS_SKIP_CC=1` so only bindgen runs (faster regeneration).

Design notes
- Targets Box2D v3 C API for portability and simplicity.
- Safe crate exposes both RAII-style wrappers and ID-based APIs to suit different needs (IDs reduce lifetime friction in game state).
- `mint` interop: you can pass `mint::Vector2<f32>` / `mint::Point2<f32>` as vectors; arrays and tuples are also supported.

License
- `boxdd`: MIT OR Apache-2.0
- `boxdd-sys`: MIT OR Apache-2.0

Quick recipes
- Basic world and bodies
  ```no_run
  use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
  let def = WorldDef::builder().gravity([0.0, -9.8]).build();
  let mut world = World::new(def).unwrap();
  let a = world.create_body_id(BodyBuilder::new().position([-1.0, 2.0]).build());
  let b = world.create_body_id(BodyBuilder::new().position([ 1.0, 2.0]).build());
  let sdef = ShapeDef::builder().density(1.0).build();
  world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
  world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
  world.step(1.0/60.0, 4);
  ```
- Revolute joint with limits/motor
  ```no_run
  use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, RevoluteJointDef};
  let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build()).unwrap();
  let a = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
  let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());
  let sdef = ShapeDef::builder().density(1.0).build();
  world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
  world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
  let base = world.joint_base_from_world_points(a, b, world.body_position(a), world.body_position(b));
  let rdef = RevoluteJointDef::new(base).limit_deg(-30.0, 30.0).enable_motor(true).motor_speed_deg(90.0).max_motor_torque(10.0);
  let _jid = world.create_revolute_joint_id(&rdef);
  ```
- Prismatic joint along an axis
  ```no_run
  use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, PrismaticJointDef};
  let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build()).unwrap();
  let a = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
  let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());
  let sdef = ShapeDef::builder().density(1.0).build();
  world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
  world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
  let axis = Vec2::new(1.0, 0.0);
  let base = world.joint_base_from_world_with_axis(a, b, world.body_position(a), world.body_position(b), axis);
  let pdef = PrismaticJointDef::new(base).with_limit_and_spring(0.0, 2.0, 4.0, 0.7);
  let _jid = world.create_prismatic_joint_id(&pdef);
  ```
- Ray cast
  ```no_run
  use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, QueryFilter};
  let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build()).unwrap();
  // ... add some shapes ...
  let hit = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
  if hit.hit { /* use hit.point / hit.normal */ }
  ```
