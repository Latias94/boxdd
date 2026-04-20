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
- Math interop (features: `mint`/`cgmath`/`nalgebra`/`glam`): any `Into<Vec2>` accepts the corresponding 2D vector/point types, plus arrays/tuples.
- Two error-handling styles: panic-on-misuse by default, plus `try_*` APIs returning `ApiResult<T>` for recoverable errors.
- Explicit threading model: `worker_count` enables Box2D's internal parallelism, while `World` and owned handles remain pinned to one thread/task.
- Hot-path query, debug-draw collection, and state-extraction APIs expose `*_into` variants so games can reuse `Vec` buffers across frames instead of reallocating.
- Character mover helpers cover the full safe workflow: `cast_mover`, `collide_mover`, `solve_planes`, and `clip_vector`.
- World runtime helpers cover counters, per-stage `Profile` timings, speculative-collision toggles, and safe explosion control.
- Standalone collision geometry helpers cover shape proxies, GJK distance, contact manifolds, chain-segment manifolds, shape cast, TOI, and `Aabb::is_valid` / `Aabb::ray_cast` without raw `ffi`.
- Shape creation and editing now use crate-owned geometry values, and chain segments can be inspected through the crate-owned `ChainSegment` type.
- Live shape runtime helpers now cover `aabb`, `test_point`, direct `ray_cast`, computed `mass_data`, and runtime event toggles without raw `ffi`.
- Body runtime helpers now cover `rotation`, sleep/awake/enabled/bullet/name controls, attached `shapes/joints` enumeration, and body-level contact/hit event toggles.
- Joint runtime helpers now cover both common metadata/control and type-specific distance/prismatic/revolute/weld/wheel/motor state across owned/scoped/id-style APIs.
- Shape classification, mass properties, and contact extraction now use crate-owned value types such as `ShapeType`, `MassData`, `ContactData`, and `Manifold`.
- Body motion constraints use the crate-owned `MotionLocks` type instead of raw Box2D flags.
- Crate-owned `MassData` and `MotionLocks` cross the FFI boundary explicitly via `from_raw(...)` / `into_raw()` when raw interop is still needed.
- Debug draw callbacks and collected commands use the crate-owned `HexColor` type instead of raw `ffi` colors.
- Typed world-level friction and restitution mixing callbacks expose `user_material_id` without dropping to raw `ffi`.

## Quickstart
```rust
use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};

let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
let mut world = World::new(def).unwrap();
let body = world.create_body_owned(BodyBuilder::new().position([0.0, 2.0]).build());
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = world.create_polygon_shape_for_owned(body.id(), &ShapeDef::default(), &poly);
world.step(1.0/60.0, 4);
```

## Features (optional)
- `serde`: serialization for core value/config types (`Vec2`, `Rot`, `Transform`, `Aabb`, `QueryFilter`, etc.).
- `serialize`: snapshot helpers (save/apply world config; take/rebuild minimal full-scene snapshot).
- `mint`: lightweight math interop types (`mint::Vector2`, `mint::Point2`, bidirectional `mint::RowMatrix2` / `mint::ColumnMatrix2` for `Rot`, and row/column-major 2D affine matrices for `Transform`).
- `cgmath`, `nalgebra`, `glam`: conversions with their 2D types (e.g. `Vector2/Point2`, `UnitComplex/Isometry2`, `glam::Vec2`).
- `bytemuck`: enable `Pod`/`Zeroable` for core math types (`Vec2`, `Rot`, `Transform`, `Aabb`) for zero-copy interop.
- `unchecked`: exposes extra `unsafe` unchecked APIs for hot paths (skips id validity checks; you must guarantee ids are valid).

## Math Interop
- `Vec2` always accepts `[f32; 2]` and `(f32, f32)` anywhere `Into<Vec2>` is used.
- `mint` now covers `Vec2`, `Aabb`, `Rot`, and `Transform`, including row- and column-major 2D matrix forms and recoverable `TryFrom` validation for `Rot` / `Transform`.
- `cgmath`, `nalgebra`, and `glam` remain first-class interop options for projects that already standardize on those math crates.

## Threading and Async
- `WorldDef::builder().worker_count(n)` lets Box2D use internal worker threads during `world.step(...)`. It does not make `World`, `WorldHandle`, or owned handles `Send`/`Sync`.
- Keep physics ownership on one thread/task. In async runtimes prefer `spawn_local` / `LocalSet`; in multi-threaded engines prefer a dedicated physics thread and communicate with channels.
- `set_custom_filter*`, `set_pre_solve*`, `set_friction_callback`, and `set_restitution_callback` may run on Box2D worker threads, so those closures must stay `Send + Sync` and should be treated as pure callbacks.
- See `examples/physics_thread.rs` for a minimal dedicated-thread pattern.

## Error Handling
- The default safe APIs panic on misuse such as stale ids or calling Box2D while the world is locked in a callback. This keeps the common path terse and avoids Rust-level UB.
- At engine/runtime boundaries, prefer `try_*` APIs and handle `ApiError` explicitly.
- `ApiError` covers stale ids, callback-locked access, invalid typed-joint family use, invalid chain defs, interior NUL strings, typed user-data mismatches, and material-callback slot exhaustion.
- World-level runtime tuning and explosion helpers now also expose `try_*` variants when callback locking should be handled recoverably.

## World Runtime Extras
- `world.counters()` and `world.profile()` expose simulation size counters and last-step timing breakdowns without dropping to raw `ffi`.
- `ExplosionDef` and `world.explode(...)` / `world.try_explode(...)` now expose Box2D's explosion API directly on the main safe surface.
- Runtime tuning controls such as sleeping, continuous collision, warm starting, speculative collision, restitution threshold, hit threshold, contact tuning, and maximum linear speed now have matching `try_*` coverage.
- `BodyBuilder::allow_fast_rotation(...)`, computed body AABB helpers (`Body::aabb()`, `OwnedBody::aabb()`, `World::body_aabb(...)`), and read-only `WorldHandle` runtime getters keep more of the upstream runtime surface on the main safe API.

## Snapshots
- Enable `serialize` and see example `examples/scene_serialize.rs` for a minimal scene round-trip.
- Note: chain shapes are captured when created via this wrapper (`World::create_chain_for_id` / `Body::create_chain`).
- Note: `ShapeDef` flags that have no runtime getters are captured for shapes created via this wrapper.

## Build Modes
- From source: builds vendored Box2D C sources via `cc` and uses pregenerated bindings by default.
  - Example: `cargo build -p boxdd`.
- System library (optional): link an existing `box2d` installed on the system.
  - Via env: set `BOX2D_LIB_DIR=/path/to/lib` and optionally `BOXDD_SYS_LINK_KIND=static|dylib`.
  - Via feature: enable `pkg-config` and provide `box2d` through your system's package manager.
  - Note: crate features that affect C build (e.g. `simd-avx2`, `disable-simd`, `validate`) are ignored in system mode. Set `BOXDD_SYS_STRICT_FEATURES=1` to fail the build if such features are enabled.

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
- `examples/physics_thread.rs` shows the recommended dedicated physics-thread + channel pattern when your game/app is otherwise multi-threaded.

## Hot Path APIs
- Convenience methods like `world.overlap_aabb(...)` and `world.cast_ray_all(...)` still return owned `Vec`s for one-off use.
- For per-frame hot paths, prefer reusable-buffer variants such as `world.overlap_aabb_into(...)`, `world.cast_ray_all_into(...)`, `world.debug_draw_collect_into(...)`, `shape.sensor_overlaps_into(...)`, `body.contact_data_into(...)`, `body.shapes_into(...)`, `body.joints_into(...)`, and `chain.segments_into(...)`.
- `body.contact_data_into(...)` and `shape.contact_data_into(...)` now fill `Vec<ContactData>`; explicit raw escape hatches are available as `contact_data_into_raw(...)` if you truly need the upstream FFI layout.

## Character Mover APIs
- The safe wrapper now covers Box2D's geometric character mover pipeline.
- Use `world.cast_mover(...)` to test motion, `world.collide_mover(...)` or `world.collide_mover_into(...)` to collect planes, `boxdd::solve_planes(...)` to solve them, and `boxdd::clip_vector(...)` to clip velocity against solved planes.
- See `examples/character_mover.rs` for a minimal end-to-end usage example.

## Collision Geometry APIs
- `boxdd::collision` exposes Box2D's standalone low-level geometry algorithms as safe Rust value types.
- Use `ShapeProxy`, `SimplexCache`, `DistanceInput`, `ShapeCastPairInput`, `Sweep`, and `ToiInput` with `shape_distance(...)`, `shape_cast(...)`, and `time_of_impact(...)`.
- Standalone manifold helpers such as `collide_polygons(...)`, `collide_polygon_and_circle(...)`, `collide_segment_and_capsule(...)`, and `collide_chain_segment_and_polygon(...)` now return the safe `Manifold` type.
- `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)` now cover common AABB validation and ray-cast needs without reaching for `boxdd_sys::ffi`.
- These advanced APIs are intentionally not in the prelude, so collision-heavy code can import them explicitly.

## Shape Geometry APIs
- `boxdd::shapes::circle`, `segment`, `capsule`, `box_polygon`, and `polygon_from_points` return safe geometry value types instead of raw Box2D structs.
- `Shape::circle()` / `segment()` / `capsule()` / `polygon()` and the corresponding setters now use the same geometry types as world/body creation APIs.
- `Circle`, `Capsule`, and `Polygon` expose standalone helpers such as `mass_data(...)`, `aabb(...)`, `contains_point(...)`, and `ray_cast(...)` for world-free geometry work.
- Live `Shape` / `OwnedShape` / `World::shape_*` APIs now also cover runtime `aabb`, `test_point`, `ray_cast`, `mass_data`, and event-toggle state.
- Raw geometry conversion is explicit on the crate-owned geometry types: use `from_raw(...)` / `into_raw()` when you intentionally cross the FFI boundary.
- `ShapeDefBuilder::filter(...)` and `ChainDef::builder().filter(...)` now take the safe `Filter` type; explicit raw escape hatches are named `filter_raw(...)`.
- `Filter` also uses explicit raw conversion via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::b2Filter>` conversions.

## Joint Runtime APIs
- `Joint`, `OwnedJoint`, and `World::joint_*` now stay aligned for common runtime metadata and control: joint type, connected body ids, `collide_connected`, constraint tuning, local frames, and wake helpers.
- Type-specific runtime getters/setters for distance, prismatic, revolute, weld, wheel, and motor joints are now aligned across `World`, `OwnedJoint`, and scoped `Joint<'_>` handles.
- `JointType` and `ConstraintTuning` are crate-owned value types; raw access stays explicit through `joint_type_raw` and `JointType::from_raw(...)` / `into_raw()`.
- `try_*` typed joint APIs now return `ApiError::InvalidJointType` when a valid joint is used through the wrong family surface.
- World-space joint builders now preserve previously configured base flags such as `collide_connected` while populating runtime-computed body ids and local frames.

## Material Mixing Callbacks
- `world.set_friction_callback(...)` and `world.set_restitution_callback(...)` expose Box2D's material mixing hooks as safe typed closures.
- Each callback receives two `MaterialMixInput` values containing the incoming coefficient and `user_material_id`.
- These callbacks may run on Box2D worker threads, so they must stay thread-safe and should be treated as pure mixing functions.

## Events
- Three access styles:
  - By value: `world.contact_events()`/`sensor_events()`/`body_events()`/`joint_events()` return owned data for storage or cross-frame use.
  - Zero‑copy views: `with_*_events_view(...)` iterate without allocations (borrows internal buffers).
  - Raw slices: `unsafe { with_*_events(...) }` expose FFI slices (borrows internal buffers).
- Example (zero‑copy views):
```rust
use boxdd::prelude::*;
let mut world = World::new(WorldDef::default()).unwrap();
world.with_contact_events_view(|begin, end, hit| {
    let _ = (begin.count(), end.count(), hit.count());
});
world.with_sensor_events_view(|beg, end| {
    let _ = (beg.count(), end.count());
});
world.with_body_events_view(|moves| {
    for m in moves { let _ = (m.body_id(), m.fell_asleep()); }
});
world.with_joint_events_view(|j| { let _ = j.count(); });
```

## Notes
- Vendored C sources + pregenerated bindings by default (no LLVM needed on CI).
  - To force bindgen: enable the `boxdd-sys/bindgen` feature, set `BOXDD_SYS_FORCE_BINDGEN=1`, and ensure `libclang` is available. On Windows/MSVC, set `LIBCLANG_PATH` if needed.
- On docs.rs, the native C build is skipped.
- Safe handle methods validate ids and panic on invalid ids (prevents UB if an id becomes stale). For recoverable failures (invalid ids / calling during Box2D callbacks), use `try_*` APIs.
- Threading: `World` and owned handles are `!Send`/`!Sync`. `worker_count` only controls Box2D's internal stepping workers. Run physics on one thread; in async runtimes prefer `spawn_local`/`LocalSet`, or create the world inside a dedicated physics thread and communicate via channels.

## Documentation
- Local: `cargo doc --open`
- Online: https://docs.rs/boxdd

## Changelog
- See `CHANGELOG.md`.

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
