# Changelog

This project contains two crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

## [boxdd 0.3.0] - 2026-04-20

### Added
- Reusable-buffer query APIs: `*_into` / `try_*_into` for AABB overlap, ray-all, polygon overlap, shape cast, and offset query variants.
- Reusable-buffer data extraction APIs for `contact_data`, `sensor_overlaps`, `shape_sensor_overlaps`, and `segments`.
- Reusable-buffer debug draw command collection via `World::debug_draw_collect_into`.
- In-place valid-filter variants for sensor overlap hot paths: `sensor_overlaps_valid_into` and `shape_sensor_overlaps_valid_into`.
- Workstream documentation under `docs/workstreams/query-buffer-reuse/` to track the 0.3 allocation-hotpath refactor plan, milestones, and cleanup backlog.
- Safe character mover APIs covering collision-plane collection and solver helpers: `collide_mover`, `collide_mover_into`, `solve_planes`, and `clip_vector`.
- A broader 0.3 umbrella workstream under `docs/workstreams/boxdd-0.3-fearless-refactor/` to track the rest of the fearless refactor plan.
- Typed world-level material mixing callbacks for friction and restitution using `MaterialMixInput` and `user_material_id`.
- A standalone `collision` module with safe `ShapeProxy`, `SimplexCache`, `DistanceInput`, `ShapeCastPairInput`, `Sweep`, `ToiInput`, `ToiState`, and `*_distance` / `shape_cast` / `time_of_impact` helpers.
- Safe standalone manifold collision helpers for the crate-owned circle/capsule/segment/polygon geometry types.
- Safe standalone chain-segment manifold collision helpers and a crate-owned `ChainSegment` geometry type.
- `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)` for low-level geometry checks without raw FFI.
- Crate-owned `Circle`, `Segment`, `Capsule`, and `Polygon` geometry value types, including standalone mass/AABB/point/ray helpers for world-free shape geometry work.
- Crate-owned `ShapeType`, `MassData`, `ContactData`, `Manifold`, and `ManifoldPoint` value types for the main safe API surface.
- Crate-owned `MotionLocks` for body translation/rotation constraints.
- Crate-owned `HexColor` for debug-draw callbacks and collected debug-draw commands.
- `mint` interop for `Rot -> mint::RowMatrix2` / `mint::ColumnMatrix2`, plus `Transform <-> mint::ColumnMatrix3x2` / `mint::ColumnMatrix2x3`.
- A dedicated `examples/physics_thread.rs` example showing the recommended dedicated-thread + channel integration pattern.
- Live shape runtime wrappers for `aabb`, `test_point`, direct `ray_cast`, computed `mass_data`, and runtime event toggles across `Shape`, `OwnedShape`, and `World::shape_*`, plus symmetric `try_sensor_overlaps_valid` helpers.
- Body runtime wrappers for rotation, sleep/awake/enabled/bullet/name controls, attached `shapes/joints` enumeration with reusable-buffer `*_into` variants, and body-level contact/hit event toggles across `Body`, `OwnedBody`, and `World::body_*`.
- Joint runtime wrappers for joint type/body ids, `collide_connected`, constraint tuning, local frames, wake helpers, and type-specific distance/prismatic/revolute/weld/wheel/motor getters/setters across `Joint`, `OwnedJoint`, and `World`.
- `ApiError::InvalidJointType` for recoverable `try_*` typed-joint runtime misuse when a valid joint is accessed through the wrong family surface.
- World runtime extras for `Profile` timings, `ExplosionDef`, `World::explode` / `try_explode`, and speculative collision control.
- `BodyBuilder::allow_fast_rotation`, computed body AABB helpers across `Body`, `OwnedBody`, and `World::body_aabb`, plus read-only `WorldHandle` runtime getters for gravity/counters/profile/awake-count/runtime-tuning state.
- Recoverable rotation round-tripping for `mint::RowMatrix2/ColumnMatrix2` and `glam::Mat2`, via `Rot::try_from(...)`, `RotFromMintError`, and `RotFromGlamError`.

### Changed
- Query internals now share reusable collection helpers instead of duplicating callback-to-`Vec` plumbing across each query entrypoint.
- Debug draw command collection now supports caller-owned buffer reuse and preserves nested polygon vertex / string storage when command shapes remain stable.
- Temporary polygon proxy point collection now uses a stack-first `SmallVec` path for Box2D's fixed-size proxy vertex limit.
- Contact, sensor, and chain segment extraction now share a common FFI-backed `Vec` fill helper instead of repeating `with_capacity + set_len` logic across handle types.
- Sensor valid-filter paths now reuse a single caller-owned buffer and filter in place instead of allocating a second `Vec`.
- Examples and crate docs now show both reusable-buffer hot paths and the safe character mover workflow.
- World-level callback coverage now treats material mixing as a first-class safe API beside custom filter and pre-solve.
- Collision/AABB regression tests now validate the public safe API instead of calling `boxdd_sys::ffi` directly.
- The testbed manifold viewer now uses the public safe collision API instead of `boxdd_sys::ffi::b2Collide*`.
- Owned/scoped `Shape`, `Body`, and `Chain` handles now share private helper implementations for geometry/material/state accessors, body naming, typed user-data plumbing, and common raw escape hatches, reducing internal drift risk without changing the public API.
- Breaking: `Body::transform` / `OwnedBody::transform` now return safe `Transform`; raw FFI access moved to `transform_raw` / `try_transform_raw`.
- Breaking: shape creation, editing, and geometry getters now use safe geometry values instead of raw `ffi::b2Circle` / `b2Segment` / `b2Capsule` / `b2Polygon`.
- Breaking: crate-owned geometry values now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()`; implicit `From<ffi::...>` conversions were removed for `Circle`, `Segment`, `ChainSegment`, `Capsule`, and `Polygon`, and `Polygon::new(raw)` was renamed to `Polygon::from_raw(raw)`.
- Breaking: `ShapeDefBuilder::filter` and `ChainDefBuilder::filter` now take `Filter`; raw Box2D escape hatches are named `filter_raw`.
- Breaking: `Filter` now crosses the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::b2Filter>` conversions.
- Breaking: `Shape::shape_type` / `OwnedShape::shape_type` now return safe `ShapeType`; raw access moved to `shape_type_raw` / `try_shape_type_raw`.
- Breaking: `Body::*contact_data*` and `Shape::*contact_data*` now use crate-owned `ContactData`; raw escape hatches are named `contact_data_raw` / `contact_data_into_raw` / `try_*_raw`.
- Breaking: `MassData` is now crate-owned, and its inertia field is renamed to Rust-style `rotational_inertia`.
- Breaking: `MassData` and `MotionLocks` now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::...>` conversions.
- Breaking: `BodyType`, `Aabb`, mover/query value types (`RayResult`, `Plane`, `CollisionPlane`, `PlaneSolverResult`), collision outputs (`SegmentDistanceResult`, `CastOutput`, `DistanceOutput`, `ToiState`, `ToiOutput`), and `Counters` now use explicit `from_raw(...)` and, where applicable, `into_raw()` APIs instead of implicit raw conversions.
- Breaking: collision input value types (`DistanceInput`, `ShapeCastPairInput`, `Sweep`, `ToiInput`) now use explicit named raw conversion APIs instead of implicit `From<Self> for ffi::...>` conversions.
- Breaking: `ManifoldPoint`, `Manifold`, and `ContactData` now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::...>` conversions.
- Breaking: `ShapeType` and `HexColor` now rely solely on their named raw conversion APIs (`from_raw(...)` / `into_raw()`) instead of compatibility `From` shims.
- Internal: mirrored `World` / `WorldHandle` query methods now share a single internal definition, reducing drift risk as the 0.3 query surface grows.
- Internal: joint creation entrypoints now share a single internal definition across all joint types, and `try_create_*_joint*` now consistently return `ApiError::InCallback` when called from callbacks.
- Internal: body/contact/sensor/joint event views now share a single world-level event-buffer borrow/cleanup helper, reducing drift risk in deferred-destroy handling.
- Internal: world-level circle/segment/capsule/polygon shape create/edit helpers now share single internal definitions instead of repeating identical geometry-to-FFI plumbing per shape type.
- Internal: safe and raw debug-draw paths now share callback panic forwarding and option wiring helpers, and `debug_draw_raw` has dedicated panic/in-callback regression coverage.
- Docs/tests: the remaining intentional raw escape hatches are now explicitly documented in the 0.3 workstream, and raw event/debug callback paths have dedicated regression coverage.
- Internal: deferred destroys scheduled while borrowing raw event buffers now wait for the outermost nested event view to finish, and raw body/sensor/joint event escape hatches have direct regression coverage.
- Internal: `Joint` / `OwnedJoint` now share private helpers for common state accessors, threshold controls, and raw/typed user-data plumbing, reducing one of the last large owned/scoped duplication pockets.
- Internal: world-level shape runtime helpers now share the same private implementation path as owned/scoped shape handles, reducing completeness drift across the three shape API styles.
- Internal: the new body runtime completeness slice shares single private helper paths for attached-shape/joint enumeration and body runtime controls instead of duplicating FFI plumbing across owned/scoped/id styles.
- Internal: computed body AABB and read-only `World` / `WorldHandle` runtime getters now share single helper paths, reducing one of the remaining handle-style drift pockets in the 0.3 runtime surface.
- Internal: `mint` and `glam` rotation validation now follow the same pure-rotation acceptance rules as `Transform` conversion, removing another asymmetry in the math interop surface.
- Internal: world-level joint runtime helpers now share the same private implementation path as owned/scoped joint handles, including type-specific joint family validation, reducing completeness drift across the three joint API styles.
- World-level runtime tuning helpers now expose matching `try_*` variants for callback-sensitive controls instead of forcing panic-only access on that slice.
- Breaking: raw world-id escape hatches now use explicit naming: `World::raw` / `WorldHandle::raw` moved to `world_id_raw`, and body/shape/chain `world_id` accessors moved to `world_id_raw` / `try_world_id_raw`.
- Breaking: `DebugDraw` / `RawDebugDraw` color parameters and collected command colors now use crate-owned `HexColor` instead of leaking `ffi::b2HexColor`.
- Docs: crate docs and README now spell out the threading / async model (`worker_count` vs `World: !Send/!Sync`) and the intended panic-by-default vs `try_*` error-handling split.

### Fixed
- World-space joint builders no longer clobber previously configured base fields such as `collide_connected` when filling runtime-computed body ids and local frames.
- Safe type-specific joint runtime APIs no longer rely on upstream Box2D family asserts alone; wrong-family `try_*` calls now fail with `ApiError::InvalidJointType`.
- `cgmath::Basis2 -> Rot` now reconstructs the rotation from the correct axis, so round-tripping through `cgmath` no longer flips the angle sign.

## [boxdd 0.2.0] - 2025-12-17

### Added
- Typed user data APIs for `World`/`Body`/`Shape`/`Joint` with automatic cleanup on destroy paths.
- `CallbackWorld` and `World::set_custom_filter_with_ctx` / `World::set_pre_solve_with_ctx` to access typed user data safely from Box2D callbacks.
- `cgmath` interop for `Transform <-> cgmath::Matrix3<f32>`.
- `cgmath`/`nalgebra` tuple conversions for `Aabb`.

### Changed
- Breaking: `World::debug_draw` / `World::debug_draw_raw` are now safe APIs (the old `unsafe` call sites must be updated).
- `with_*_events_view` are safe and automatically defer destruction while borrowing Box2D event buffers.
- Docs and README wording aligned around general “math interop” (not only `mint`).
 - Dependency: `boxdd` now requires `boxdd-sys >= 0.2.1` (CI packaging fixes live in `boxdd-sys 0.2.1`).

### Fixed
- Guarded raw `unsafe with_*_events` to defer destruction while borrowing Box2D event buffers.
- Multiple stale safety notes after refactors.

## [boxdd-sys 0.2.0] - 2025-12-17

### Notes
- No upstream Box2D revision change in this release (version bump aligned with `boxdd`).

## [boxdd-sys 0.2.1] - 2025-12-17

### Fixed
- CI packaging: add `package-bin` feature and declare the internal `package` binary so `prebuilt-binaries.yml` works.
- CI runner: switch Intel macOS builds from `macos-13` (retired) to `macos-15-intel`.
