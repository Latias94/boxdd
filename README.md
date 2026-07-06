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
- `bevy_boxdd`: Bevy ECS integration for `boxdd` with fixed-step systems, ECS-authored joints, entity-mapped ray/AABB queries, debug draw collection, and physics messages

## Engineering Status
- API coverage matrix: `docs/api-coverage.md` tracks every vendored Box2D `B2_API` symbol; the current safe layer accounts for 424 of 430 symbols, with 4 raw-only and 2 omitted by rationale.
- Official sample parity: `docs/upstream-parity/box2d-sample-matrix.md` maps non-benchmark upstream samples to Rust examples, tests, or testbed scenes. Benchmark rows may remain indexed references when that is the useful artifact.
- GitHub Pages source: `docs/pages/index.html` is generated from checked-in Cargo examples and testbed scenes, so the site works as an example index instead of a hand-written marketing page.
- Bevy integration: `bevy_boxdd` exposes `RigidBody`, `Collider`, `PhysicsMaterial`, distance/revolute `JointDescriptor`, transform sync, entity-mapped ray and AABB overlap helpers through `BoxddPhysicsContext`, debug draw command collection, recoverable error messages, and body/contact/sensor messages.

## 0.4.0 Highlights
- `0.4.0` realigns `boxdd-sys` with the official upstream Box2D submodule again, so repository checkouts and CI no longer depend on a local-only Box2D patch commit.
- Workspace metadata is centralized for `boxdd`, `boxdd-sys`, `bevy_boxdd`, and `xtask`.
- `xtask` now validates API coverage, strict official sample parity, and the generated Pages example index.
- `boxdd::dynamic_tree` wraps the standalone Box2D broad-phase tree as an owned safe Rust type.
- `bevy_boxdd` provides a Bevy 0.19 integration crate without adding Bevy dependencies to the core binding, with compiling examples for contacts, sensors, ray/AABB queries, kinematic transform sync, ECS joints, child colliders, collision filters, and debug draw collection.
- Hot-path APIs are first-class: keep the simple `Vec`-returning calls for one-off use, or move per-frame code to `*_into` and `visit_*`.
- `boxdd::collision` now exposes standalone distance, shape-cast, TOI, manifold, and `Aabb::ray_cast` helpers without dropping to raw `ffi`.
- Shape geometry helpers now expose shape-specific `shape_cast` / `try_shape_cast` entrypoints for circle, capsule, segment, and polygon values.
- Character-mover support is complete on the safe surface: `cast_mover`, `collide_mover`, `solve_planes`, and `clip_vector`.
- `WorldHandle` is now a practical read-only follow-up surface for body, shape, joint, query, and owned-event inspection.
- Callback, query, dynamic-tree, material-mix, event-view, public non-send, typed user-data, and world recycle lifecycle tests now run by default instead of relying on ignored safety tests.
- Raw FFI boundaries are explicit and easier to reason about: use crate-owned ids/value types plus named `from_raw(...)`, `into_raw()`, and `*_raw` escape hatches.
- Open-chain runtime material access still uses visible live-segment indexing on the safe API, but that normalization now lives in Rust instead of a custom Box2D patch.
- The examples and testbed were reorganized around the intended current workflows instead of a loose grab bag of demos.

## Detailed Highlights
- Safe, ergonomic Rust wrapper over the official Box2D v3 C API.
- Math interop (features: `mint`/`cgmath`/`nalgebra`/`glam`): any `Into<Vec2>` accepts the corresponding 2D vector/point types, plus arrays/tuples.
- Two error-handling styles: panic-on-misuse by default, plus `try_*` APIs returning `ApiResult<T>` for recoverable errors.
- Explicit threading model: `worker_count` remains part of `WorldDef`, while `World` and owned handles stay pinned to one thread/task; actual Box2D worker-thread stepping still requires an explicit raw task-system configuration path.
- Hot-path query, debug-draw collection, and state-extraction APIs expose `*_into` buffer-reuse variants, and overlap queries also expose `visit_*` forms for zero result-container allocation.
- Character mover helpers cover the full safe workflow: `cast_mover`, `collide_mover`, `solve_planes`, and `clip_vector`.
- World runtime helpers cover counters, per-stage `Profile` timings, speculative-collision toggles, and safe explosion control.
- Core math types (`Vec2`, `Rot`, `Transform`) now cross the Box2D raw boundary explicitly through `from_raw(...)` / `into_raw()` instead of implicit conversions.
- Global foundation helpers now cover allocated-byte inspection, timing ticks/millisecond helpers, thread yielding, and deterministic hashing without dropping to raw `ffi`.
- Standalone collision geometry helpers cover shape proxies, GJK distance, segment distance, contact manifolds, chain-segment manifolds, shape cast, TOI, and `Aabb::is_valid` / `Aabb::ray_cast` without raw `ffi`, with matching recoverable `try_*` entrypoints for malformed input.
- Shape creation and editing now use crate-owned geometry values, and chain segments can be inspected through the crate-owned `ChainSegment` type.
- Chain runtime material access now uses visible segment indexing on open chains instead of leaking Box2D's ghost-placeholder storage layout.
- Safe shape/joint mutators now front-load obvious Box2D assert preconditions such as non-negative material scalars and ordered joint limits.
- Safe world/body/joint creation now validates obvious Box2D definition preconditions before entering native code, and definition value objects expose `validate()` helpers for preflight checks.
- Pointer-bearing config wrappers now keep their raw re-entry explicit: `BodyDef::from_raw(...)` and `WorldDef::from_raw(...)` are `unsafe` because raw names/task callbacks can otherwise punch through later safe creation paths.
- Live shape runtime helpers now cover `aabb`, `test_point`, direct `ray_cast`, computed `mass_data`, and runtime event toggles without raw `ffi`.
- Body runtime helpers now cover `rotation`, sleep/awake/enabled/bullet/name controls, attached `shapes/joints` enumeration, and body-level contact/hit event toggles.
- Joint runtime helpers now cover both common metadata/control and type-specific distance/prismatic/revolute/weld/wheel/motor state across owned/scoped/id-style APIs.
- Opaque ids (`BodyId`, `ShapeId`, `JointId`, `ChainId`, `ContactId`) are now crate-owned value types; raw interop is explicit through `from_raw(...)` / `into_raw()`.
- `ContactId` now exposes direct `is_valid` / `data` / `data_raw` and `try_*` helpers as inherent methods; no extension-trait import is required to inspect contact ids from events or snapshots.
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
let mut body = world.create_body_owned(BodyBuilder::new().position([0.0, 2.0]).build());
let poly = shapes::box_polygon(0.5, 0.5);
let _shape = body.create_polygon_shape(&ShapeDef::default(), &poly);
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
- `WorldDef::builder().worker_count(n)` stores the desired upstream worker count, but Box2D only uses worker threads when task callbacks are also installed through `unsafe WorldBuilder::task_system_raw(...)`, `WorldDef::set_task_system_raw(...)`, or a fully raw `WorldDef` path. It does not make `World`, `WorldHandle`, or owned handles `Send`/`Sync`.
- Keep physics ownership on one thread/task. In async runtimes prefer `spawn_local` / `LocalSet`; in multi-threaded engines prefer a dedicated physics thread and communicate with channels.
- `set_custom_filter*`, `set_pre_solve*`, `set_friction_callback`, and `set_restitution_callback` may run on Box2D worker threads, so those closures must stay `Send + Sync` and should be treated as pure callbacks.
- See `examples/physics_thread.rs` for a minimal dedicated-thread pattern.

## Error Handling
- The default safe APIs panic on misuse such as stale ids or calling Box2D while the world is locked in a callback. This keeps the common path terse and avoids Rust-level UB.
- At engine/runtime boundaries, prefer `try_*` APIs and handle `ApiError` explicitly.
- `ApiError` covers stale ids, callback-locked access, invalid arguments, out-of-range runtime indices, invalid typed-joint family use, invalid chain defs, interior NUL strings, typed user-data mismatches, and material-callback slot exhaustion.
- `WorldDef`, `BodyDef`, `ShapeDef`, `SurfaceMaterial`, `JointBase`, and concrete `*JointDef` values expose `validate()` so tooling and editor flows can reject invalid config before calling `create_*`.
- World-level runtime tuning and explosion helpers now also expose `try_*` variants when callback locking should be handled recoverably.

## World Runtime Extras
- `world.counters()` and `world.profile()` expose simulation size counters and last-step timing breakdowns without dropping to raw `ffi`.
- `ExplosionDef` and `world.explode(...)` / `world.try_explode(...)` now expose Box2D's explosion API directly on the main safe surface.
- Runtime tuning controls such as sleeping, continuous collision, warm starting, speculative collision, restitution threshold, hit threshold, contact tuning, and maximum linear speed now have matching `try_*` coverage.
- `BodyBuilder::allow_fast_rotation(...)`, computed body AABB helpers (`Body::aabb()`, `OwnedBody::aabb()`, `World::body_aabb(...)`), and read-only `WorldHandle` runtime getters for world diagnostics plus body-by-id, shape-by-id, and joint-by-id queries keep more of the upstream runtime surface on the main safe API.

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

# validate upstream accounting assets
cargo run -p xtask -- api-coverage --check
cargo run -p xtask -- sample-parity --check
cargo run -p xtask -- generate-pages
cargo run -p xtask -- validate-pages

# run a few representative examples
cargo r --example world_basics
cargo r --example queries
cargo r --example character_mover
cargo r --example dynamic_tree
cargo r --example mint_interop --features mint
cargo r --example physics_thread
cargo r --example scene_serialize --features serialize
cargo r --example testbed_imgui_glow --features imgui-glow-testbed

# check the Bevy adapter without pulling it into the core crate
cargo nextest run -p bevy_boxdd --test plugin
cargo check -p bevy_boxdd --examples
cargo run -p bevy_boxdd --example overlap_query_2d
cargo run -p bevy_boxdd --example debug_draw_gizmos_2d
```

## Examples
- The example catalog is now grouped by topic in [`boxdd/examples/README.md`](boxdd/examples/README.md), so users can start from the workflows they care about instead of scanning file names.
- Recommended starting points:
  - `world_basics`: minimal world/body/shape setup
  - `buffer_reuse`, `queries`, `query_casts`, `character_mover`: the main `0.4.0` hot-path, overlap, cast, and mover workflows
  - `dynamic_tree`: standalone broad-phase tree ownership and query/cast workflows
  - `mint_interop`: optional `mint` vector/point/matrix interop sample behind the `mint` feature
  - `collision_basics`: standalone collision geometry helpers without constructing a `World`
  - `events_summary`, `events_view`: owned-with-reuse vs borrowed zero-copy event access
  - `world_handle_reads`: stored read-only `WorldHandle` follow-up queries after id-producing world queries, including reusable-buffer overlap reads
  - `scene_serialize`: snapshot/restore flows behind the `serialize` feature
  - `physics_thread`: the recommended dedicated physics-thread ownership model
  - `testbed_imgui_glow`: optional interactive testbed on the current `dear-imgui-*` stack
  - `bevy_boxdd/examples/falling_box_2d.rs`: Bevy adapter smoke example
  - `bevy_boxdd/examples/contact_events_2d.rs`, `sensor_events_2d.rs`, `ray_query_2d.rs`, `overlap_query_2d.rs`, `kinematic_platform_2d.rs`, `joint_bridge_2d.rs`, `child_colliders_2d.rs`, `collision_filter_2d.rs`, `debug_draw_collect_2d.rs`, `debug_draw_gizmos_2d.rs`: Bevy messages, entity query access, app-driven kinematic sync, ECS joints, child colliders, collision filters, and debug draw command rendering

## Hot Path APIs
- Convenience methods like `world.overlap_aabb(...)` and `world.cast_ray_all(...)` still return owned `Vec`s for one-off use.
- For per-frame hot paths, prefer reusable-buffer variants such as `world.overlap_aabb_into(...)`, `world.cast_ray_all_into(...)`, `world.debug_draw_collect_into(...)`, `shape.sensor_overlaps_into(...)`, `body.contact_data_into(...)`, `body.shapes_into(...)`, `body.joints_into(...)`, and `chain.segments_into(...)`.
- For overlap-heavy paths that only need streaming inspection or early exit, prefer `world.visit_overlap_aabb(...)`, `visit_overlap_polygon_points(...)`, and `visit_overlap_polygon_points_with_offset(...)` so no result container needs to be built at all.
- `body.contact_data_into(...)` and `shape.contact_data_into(...)` now fill `Vec<ContactData>`; explicit raw escape hatches are available as `contact_data_raw_into(...)` if you truly need the upstream FFI layout.

## Character Mover APIs
- The safe wrapper now covers Box2D's geometric character mover pipeline.
- Use `world.cast_mover(...)` to test motion, `world.collide_mover(...)` or `world.collide_mover_into(...)` to collect planes, `boxdd::solve_planes(...)` / `boxdd::try_solve_planes(...)` to solve them, and `boxdd::clip_vector(...)` / `boxdd::try_clip_vector(...)` to clip velocity against solved planes.
- See `examples/character_mover.rs` for a minimal end-to-end usage example.

## Collision Geometry APIs
- `boxdd::collision` exposes Box2D's standalone low-level geometry algorithms as safe Rust value types.
- Use `ShapeProxy`, `SimplexCache`, `DistanceInput`, `ShapeCastPairInput`, `Sweep`, and `ToiInput` with `segment_distance(...)`, `shape_distance(...)`, `shape_cast(...)`, and `time_of_impact(...)`, or the matching recoverable `try_*` variants when malformed input should return `ApiError::InvalidArgument`.
- Standalone manifold helpers such as `collide_polygons(...)`, `collide_polygon_and_circle(...)`, `collide_segment_and_capsule(...)`, and `collide_chain_segment_and_polygon(...)` return the safe `Manifold` type and now also expose matching recoverable `try_collide_*` variants.
- `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)` now cover common AABB validation and ray-cast needs without reaching for `boxdd_sys::ffi`.
- These advanced APIs are intentionally not in the prelude, so collision-heavy code can import them explicitly.

## Shape Geometry APIs
- `boxdd::shapes::circle`, `segment`, `capsule`, `box_polygon`, `rounded_box_polygon`, and `polygon_from_points` return safe geometry value types instead of raw Box2D structs.
- `Shape::circle()` / `segment()` / `capsule()` / `polygon()` and the corresponding setters now use the same geometry types as world/body creation APIs.
- `Circle`, `Segment`, `Capsule`, and `Polygon` expose standalone helpers such as `mass_data(...)`, `aabb(...)`, `contains_point(...)`, `ray_cast(...)`, and `transformed(...)` for world-free geometry work.
- `Circle`, `Segment`, `Capsule`, `ChainSegment`, and `Polygon` now expose `is_valid()` / `validate()` so engines can preflight geometry before crossing the FFI boundary.
- Those world-free helper methods also expose recoverable `try_*` variants (`try_mass_data`, `try_aabb`, `try_contains_point`, `try_ray_cast`, `try_transformed`) so malformed helper inputs can return `ApiError::InvalidArgument` instead of panicking.
- Polygon construction helpers also expose recoverable `try_*` variants (`try_square_polygon`, `try_box_polygon`, `try_rounded_box_polygon`, `try_offset_*`, `try_polygon_from_points`) so finite extents, transforms, and hull construction can be validated without collapsing everything to `Option`.
- Live `Shape` / `OwnedShape` / `World::shape_*` APIs now also cover runtime `aabb`, `test_point`, `ray_cast`, `mass_data`, and event-toggle state.
- Raw geometry conversion is explicit on the crate-owned geometry types: use `from_raw(...)` / `into_raw()` when you intentionally cross the FFI boundary.
- `ShapeDefBuilder::filter(...)` and `ChainDef::builder().filter(...)` now take the safe `Filter` type; explicit raw escape hatches are named `filter_raw(...)`.
- `Filter` also uses explicit raw conversion via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::b2Filter>` conversions.
- `SurfaceMaterial` now behaves like a normal crate-owned value type: builder-style mutation uses `with_*` methods, read access uses getters such as `friction()`, `restitution()`, and `custom_color()`, `custom_color` uses crate-owned `HexColor`, and raw interop stays explicit through `from_raw(...)` / `into_raw()`.

## Joint Runtime APIs
- `Joint`, `OwnedJoint`, `World::joint_*`, and read-only `WorldHandle::joint_*` now stay aligned for common runtime metadata and control: joint type, connected body ids, `collide_connected`, constraint tuning, local frames, thresholds, and wake helpers.
- Type-specific runtime getters/setters for distance, prismatic, revolute, weld, wheel, and motor joints are aligned across `World`, `OwnedJoint`, and scoped `Joint<'_>` handles, while `WorldHandle` mirrors the read-only getter half so stored `JointId` values can stay on the handle path.
- `JointType` and `ConstraintTuning` are crate-owned value types; raw access stays explicit through `joint_type_raw` and `JointType::from_raw(...)` / `into_raw()`.
- `try_*` typed joint APIs now return `ApiError::InvalidJointType` when a valid joint is used through the wrong family surface.
- World-space joint builders now preserve previously configured base flags such as `collide_connected` while populating runtime-computed body ids and local frames.

## Material Mixing Callbacks
- `world.set_friction_callback(...)` and `world.set_restitution_callback(...)` expose Box2D's material mixing hooks as safe typed closures.
- Each callback receives two `MaterialMixInput` values containing the incoming coefficient and `user_material_id`.
- These callbacks may run on Box2D worker threads, so they must stay thread-safe and should be treated as pure mixing functions.

## Events
- Four access styles:
  - By value: `world.contact_events()`/`sensor_events()`/`body_events()`/`joint_events()` return owned data for storage or cross-frame use.
  - Reusable buffers: `*_events_into(...)` reuse caller-owned owned-event storage across frames.
  - Zero‑copy views: `with_*_events_view(...)` iterate without allocations (borrows internal buffers).
  - Raw slices: `unsafe { with_*_events_raw(...) }` expose FFI slices (borrows internal buffers).
- Callback-sensitive event entrypoints also have matching `try_*` variants so callback-lock failures can return `ApiError::InCallback` instead of forcing panic-only control flow.
- Owned event snapshots (`*_events`, `*_events_into`, `try_*`) are available on both `World` and `WorldHandle`.
- Borrowed zero-copy event views and raw event-buffer access intentionally stay on `World`, because they are tied to the completed step's world-local event buffers plus deferred-destroy flushing.
- Example (reusable buffers + zero-copy views):
```rust
use boxdd::{ContactEvents, World, WorldDef};
let mut world = World::new(WorldDef::default()).unwrap();
let mut contact_events = ContactEvents::default();
world.contact_events_into(&mut contact_events);
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
