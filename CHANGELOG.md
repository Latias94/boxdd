# Changelog

This project contains two crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

## [boxdd 0.3.0] - 2026-04-20

### Added
- `World::body_linear_velocity` / `try_body_linear_velocity` and `World::body_angular_velocity` / `try_body_angular_velocity`, completing the id-based body runtime getter surface beside the existing transform/position/rotation helpers.
- Global foundation helpers for allocated-byte inspection, `ticks` / `milliseconds_since` / `milliseconds_and_reset`, `yield_now`, `HASH_INIT`, `hash_bytes`, and `is_valid_float` without dropping to `boxdd_sys::ffi`.
- Zero-allocation overlap visitor APIs: `visit_overlap_aabb`, `visit_overlap_polygon_points`, and `visit_overlap_polygon_points_with_offset`, plus matching `try_visit_*` entrypoints on `World` and `WorldHandle`.
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
- `ContactId` now exposes direct safe `is_valid` / `data` / `data_raw` inherent helpers and recoverable `try_*` variants, plus `ApiError::InvalidContactId` for stale contact ids.
- `ApiError::InvalidJointType` for recoverable `try_*` typed-joint runtime misuse when a valid joint is accessed through the wrong family surface.
- `ApiError::InvalidArgument` for recoverable safe-wrapper validation of obvious Box2D assert preconditions such as non-negative shape material scalars and ordered joint limit/range setters.
- `validate()` helpers on `BodyDef`, `ShapeDef`, `SurfaceMaterial`, `JointBase`, and concrete joint-definition value objects so engines can preflight definition state before crossing the FFI boundary.
- `ApiError::IndexOutOfRange` for recoverable range-checked runtime index misuse, starting with chain surface-material access.
- World runtime extras for `Profile` timings, `ExplosionDef`, `World::explode` / `try_explode`, and speculative collision control.
- `BodyBuilder::allow_fast_rotation`, computed body AABB helpers across `Body`, `OwnedBody`, and `World::body_aabb`, plus read-only `WorldHandle` runtime getters for gravity/counters/profile/awake-count/runtime-tuning state.
- Read-only `WorldHandle::body_*` mirrors for body-by-id runtime queries, covering transforms, velocities, point/vector conversions, mass data, damping/flags, motion locks, and attached shape/joint enumeration without requiring a mutable `World` borrow.
- Read-only `WorldHandle::shape_*` mirrors for shape-by-id runtime queries, covering material/body lookup, AABB/point/raycast/closest-point helpers, mass data, event flags, and reusable-buffer sensor-overlap reads without requiring a mutable `World` borrow.
- Read-only `WorldHandle::joint_*` mirrors for joint-by-id runtime queries, covering both common metadata and type-specific distance/prismatic/revolute/weld/wheel/motor getter families without requiring a mutable `World` borrow.
- World-level common joint runtime coverage now includes `joint_force_threshold` / `try_joint_force_threshold` / `set_joint_force_threshold` and the matching torque-threshold APIs instead of leaving those controls on owned/scoped handles only.
- Owned event snapshot mirrors on `WorldHandle`: `*_events`, `*_events_into`, and `try_*` now match `World` for body/contact/sensor/joint event snapshots without exposing borrowed/raw event-buffer APIs there.
- Recoverable rotation round-tripping for `mint::RowMatrix2/ColumnMatrix2` and `glam::Mat2`, via `Rot::try_from(...)`, `RotFromMintError`, and `RotFromGlamError`.
- Reusable-buffer world event snapshot APIs: `body_events_into`, `contact_events_into`, `sensor_events_into`, `joint_events_into`, plus matching `try_*` variants for recoverable callback-sensitive event reads.
- Safe polygon construction helpers via `Polygon::{square_polygon,rounded_box_polygon,offset_box_polygon,offset_rounded_box_polygon,offset_from_points,hull_is_valid}` and the matching `shapes::*` free helpers.
- Read-side definition APIs so `ShapeDef` exposes crate-owned getters for material/filter/flags, `ChainDef` exposes `points()` / `filter()` / `sensor_events_enabled()` plus `material_layout()`, and both definition builders can start from existing defs via `From<...>`.
- Read-side creation-definition APIs so `BodyDef`, `JointBase`, and all concrete `*JointDef` types expose safe getters, and `BodyDef` / `JointBase` now offer `builder()` plus `From<...> for ...Builder` round-tripping.
- Read-side world-configuration APIs so `WorldDef` exposes safe getters plus explicit `from_raw(...)` / `into_raw()` symmetry, and `ExplosionDef` can now be inspected through crate-owned getters instead of acting like a write-only config shell.
- Explicit raw conversion symmetry for configuration wrappers: `BodyDef`, `ShapeDef`, `JointBase`, and all concrete `*JointDef` types now expose named `from_raw(...)` / `into_raw()` escape hatches instead of trapping the raw boundary behind internal field access.
- A release-level completeness matrix under `docs/workstreams/boxdd-0.3-fearless-refactor/completeness-matrix.md` to record which wrapper areas are safe-covered, raw-only, intentionally omitted, or candidates after `0.3`.

### Changed
- Top-level examples and testbed scenes now use the public safe world/collision/joint APIs for body velocity reads, shape distance, world counters, and revolute limits instead of calling `boxdd_sys::ffi` directly for those workflows.
- Overlap query internals now route both `Vec` collection and reusable-buffer `*_into` forms through the same visitor-based callback path, reducing one more hot-path drift pocket.
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
- Runtime shape numeric setters and joint limit/range setters now validate their obvious Box2D assert preconditions in the safe wrapper first, so `try_*` callers receive `ApiError::InvalidArgument` instead of depending on upstream assert builds.
- Body creation, body mass-data mutation, and joint creation now front-load the obvious Box2D definition preconditions in the safe wrapper, so invalid defs fail as Rust panics or `ApiError::InvalidArgument` instead of depending on native assert builds.
- Breaking: `JointBase::default()` now mirrors Box2D's actual upstream defaults (`forceThreshold = FLT_MAX`, `torqueThreshold = FLT_MAX`, `constraintHertz = 60`, `constraintDampingRatio = 2`, and `drawScale = length_units_per_meter()`) instead of a partial zeroed approximation.
- Breaking: `ContactIdExt` has been removed; its `is_valid` / `data` / `data_raw` / `try_*` helpers now live directly on `ContactId`.
- Breaking: `Chain` / `OwnedChain` runtime material count/get/set helpers now use visible live-segment indexing on open chains instead of Box2D's raw ghost-placeholder material layout; recoverable out-of-range access returns `ApiError::IndexOutOfRange`.
- Breaking: `Body::transform` / `OwnedBody::transform` now return safe `Transform`; raw FFI access moved to `transform_raw` / `try_transform_raw`.
- Breaking: core math types `Vec2`, `Rot`, and `Transform` now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::...>` conversions, aligning the last core value types with the rest of the crate-owned 0.3 surface.
- Breaking: shape creation, editing, and geometry getters now use safe geometry values instead of raw `ffi::b2Circle` / `b2Segment` / `b2Capsule` / `b2Polygon`.
- Breaking: crate-owned geometry values now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()`; implicit `From<ffi::...>` conversions were removed for `Circle`, `Segment`, `ChainSegment`, `Capsule`, and `Polygon`, and `Polygon::new(raw)` was renamed to `Polygon::from_raw(raw)`.
- Breaking: `ShapeDefBuilder::filter` and `ChainDefBuilder::filter` now take `Filter`; raw Box2D escape hatches are named `filter_raw`.
- Breaking: `Filter` now crosses the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::b2Filter>` conversions.
- Breaking: `Shape::shape_type` / `OwnedShape::shape_type` now return safe `ShapeType`; raw access moved to `shape_type_raw` / `try_shape_type_raw`.
- Breaking: `Body::*contact_data*` and `Shape::*contact_data*` now use crate-owned `ContactData`; raw escape hatches are named `contact_data_raw` / `contact_data_raw_into` / `try_contact_data_raw_into` for consistency with the broader `*_raw` surface.
- Breaking: `MassData` is now crate-owned, and its inertia field is renamed to Rust-style `rotational_inertia`.
- Breaking: `MassData` and `MotionLocks` now cross the raw FFI boundary explicitly via `from_raw(...)` / `into_raw()` instead of implicit `From<ffi::...>` conversions.
- Breaking: `BodyId`, `ShapeId`, `JointId`, `ChainId`, and `ContactId` are now crate-owned value types with explicit `from_raw(...)` / `into_raw()` conversions, and the safe API no longer exposes mixed raw-ID entrypoints.
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
- Internal: the remaining low-level joint scalar/vector getter-setter macro layer in `joints/mod.rs` now uses private generic FFI helpers plus explicit per-joint impl functions, removing another macro-based drift pocket without changing the public API.
- Internal: material-mixing callback dispatch now uses const-generic trampolines instead of a generated macro table, eliminating the last internal `macro_rules!` dependency in the safe wrapper crate.
- Internal: `serialize.rs` now reuses crate-owned body/joint/shape helper layers for scene snapshots instead of mirroring large raw FFI getter tables, reducing drift risk between runtime APIs and serialized snapshots.
- Internal: world-level shape runtime helpers now share the same private implementation path as owned/scoped shape handles, reducing completeness drift across the three shape API styles.
- Internal: the new body runtime completeness slice shares single private helper paths for attached-shape/joint enumeration and body runtime controls instead of duplicating FFI plumbing across owned/scoped/id styles.
- Internal: the remaining high-frequency `Body` / `Shape` / `Chain` owned-vs-scoped enumeration helpers (`shapes`, `joints`, `sensor_overlaps`, `segments`, and related count/into variants) now share single private definitions, further reducing handle-style drift in the 0.3 runtime surface.
- Internal: the feature-gated `unchecked` extension traits now share single internal implementations across owned/scoped body, shape, joint, and chain handles instead of repeating the same raw FFI calls per handle type.
- Internal: computed body AABB and read-only `World` / `WorldHandle` runtime getters now share single helper paths, reducing one of the remaining handle-style drift pockets in the 0.3 runtime surface.
- Internal: `WorldHandle` body-by-id read-only runtime mirrors now reuse shared `body.rs` helper implementations instead of open-coding another raw FFI getter table.
- Internal: `WorldHandle` shape-by-id read-only runtime mirrors now reuse shared `shapes/mod.rs` helper implementations instead of introducing another divergent raw FFI getter table.
- Internal: common and typed joint runtime reads on `World` and `WorldHandle` now share the same private checked-read path, and world-id joint threshold control no longer lags behind owned/scoped handles.
- Internal: `mint` and `glam` rotation validation now follow the same pure-rotation acceptance rules as `Transform` conversion, removing another asymmetry in the math interop surface.
- Internal: world-level joint runtime helpers now share the same private implementation path as owned/scoped joint handles, including type-specific joint family validation, reducing completeness drift across the three joint API styles.
- World-level runtime tuning helpers now expose matching `try_*` variants for callback-sensitive controls instead of forcing panic-only access on that slice.
- World-level callback-sensitive execution helpers now expose matching recoverable entrypoints too: `World::try_step(...)`, `try_flush_deferred_destroys()`, `try_debug_draw_collect(...)`, `try_debug_draw_collect_into(...)`, `try_debug_draw(...)`, and `try_debug_draw_raw(...)` return `ApiError::InCallback` instead of forcing panic-only control flow.
- World-level custom filter / pre-solve registration now exposes matching `try_*` variants, including the compatibility `*_callback` helpers, and the panic / recoverable paths share the same internal callback-registration plumbing.
- Zero-copy and raw event-buffer visitors now expose matching recoverable entrypoints too: `try_with_*_events_view(...)` and `unsafe { try_with_*_events_raw(...) }` return `ApiError::InCallback` instead of leaving borrowed event access as a panic-only corner.
- Events docs now cover by-value, reusable-buffer, zero-copy, and raw access styles explicitly, and `ContactEvents` / `SensorEvents` implement `Default` for caller-owned buffer reuse.
- `serialize` registry snapshots now expose reusable-buffer reads too: `World::body_ids_into(...)` / `try_body_ids_into(...)` and `World::chain_records_into(...)` / `try_chain_records_into(...)` avoid per-call allocation on wrapper-owned metadata queries.
- Docs/design: `WorldHandle` now mirrors owned event snapshots only; borrowed/raw event-buffer APIs still intentionally stay on `World` because they are coupled to completed-step buffers and deferred-destroy flushing semantics.
- Breaking: serialize-time chain metadata now stays on crate-owned vocabulary: `World::chain_records()` returns `Filter`, `Vec<Vec2>`, and `ChainMaterialsRecord`, and the old public `ChainDef` raw clone helpers are no longer exposed.
- Breaking: `SurfaceMaterial` now behaves like a normal crate-owned value type: read access uses getters such as `friction()` / `restitution()` / `custom_color()`, `custom_color` uses crate-owned `HexColor`, builder-style mutation uses `with_*` methods, and raw interop is explicit through `from_raw(...)` / `into_raw()`.
- Breaking: `PrismaticJointDef::max_motor_torque(...)` was removed; prismatic creation-time motor configuration now uses the correct `max_motor_force(...)` name only.
- Breaking: raw pointer user-data escape hatches are now named explicitly: `set_user_data_ptr` / `user_data_ptr` and their `try_*` variants moved to `*_raw` forms across body/shape/joint handles.
- Breaking: raw event-slice visitors are now named explicitly: `with_contact_events`, `with_sensor_events`, `with_body_events`, and `with_joint_events` moved to `*_raw` forms to match the rest of the public raw seam vocabulary.
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
