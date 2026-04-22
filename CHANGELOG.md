# Changelog

This project contains two crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

## [boxdd 0.3.0] - 2026-04-20

### Highlights
- Hot-path world queries, contact extraction, sensor-overlap reads, chain segment reads, debug-draw collection, and event snapshots now support reusable caller-owned buffers via `*_into` APIs, and overlap queries also support zero-allocation `visit_*` forms.
- The safe wrapper now covers Box2D's character-mover workflow end to end: `cast_mover`, `collide_mover`, `solve_planes`, `clip_vector`, and matching recoverable `try_*` variants.
- A new standalone `boxdd::collision` module exposes safe world-free distance, shape-cast, TOI, manifold, and AABB helpers using crate-owned value types.
- Runtime coverage is substantially broader across `World`, `WorldHandle`, `Body`, `Shape`, `Joint`, and `ContactId`, including read-only id-based mirrors on `WorldHandle`, richer event APIs, and direct `ContactId` inspection helpers.
- Core wrapper vocabulary is now crate-owned and explicit: ids, geometry values, `ShapeType`, `MassData`, `ContactData`, `Manifold`, `MotionLocks`, `HexColor`, and raw FFI crossings use named `from_raw(...)` / `into_raw()` boundaries.

### Added
- Reusable-buffer query APIs for overlap, ray-all, shape-cast, mover-plane collection, contact data, sensor overlaps, chain segments, debug-draw collection, and owned event snapshots.
- Safe standalone collision helpers for `ShapeProxy`, segment distance, GJK distance, shape cast, TOI, circle/capsule/segment/polygon manifold generation, and chain-segment manifolds.
- Crate-owned geometry values (`Circle`, `Segment`, `ChainSegment`, `Capsule`, `Polygon`) with validation plus world-free `mass_data`, `aabb`, `contains_point`, `ray_cast`, and transform helpers.
- Recoverable `try_*` validation paths across world stepping, world queries, mover helpers, geometry helpers, shape creation/editing, and standalone collision input types.
- Readable definition/config value objects and validation helpers for `BodyDef`, `ShapeDef`, `ChainDef`, `JointBase`, concrete joint defs, `WorldDef`, and `ExplosionDef`.
- World/runtime completeness coverage for body state, shape runtime queries, common and typed joint runtime APIs, `Profile`, explosions, speculative collision controls, and `BodyBuilder::allow_fast_rotation`.
- Read-only `WorldHandle::{body_*,shape_*,joint_*}` query surfaces and owned event snapshot mirrors.
- Math interop and rotation round-tripping improvements for `mint`, `glam`, `cgmath`, and `nalgebra`, plus global foundation helpers such as byte-count inspection, deterministic hashing, and timing utilities.
- New example coverage for the dedicated physics-thread ownership model and refreshed testbed/examples around the expanded safe API.

### Breaking Changes
- The crate now consistently uses explicit raw-boundary APIs. Crate-owned ids, math types, geometry values, filters, mass/contact/manifold values, colors, and several query/collision value types no longer rely on implicit raw `From` conversions.
- Geometry creation/editing and geometry getters now use crate-owned safe geometry values instead of raw Box2D structs.
- Raw escape hatches were renamed to make FFI boundaries explicit. This includes `world_id_raw`, `*_user_data_ptr_raw`, `with_*_events_raw(...)`, `transform_raw`, `shape_type_raw`, and `contact_data_raw*`.
- `Body::transform` / `OwnedBody::transform` now return safe `Transform`; raw access moved to `transform_raw` / `try_transform_raw`.
- `ContactIdExt` was removed; its inspection helpers now live directly on `ContactId`.
- `Chain` / `OwnedChain` runtime material access on open chains now uses visible live-segment indexing instead of Box2D's ghost-placeholder layout.
- `BodyDef::from_raw(...)` and `WorldDef::from_raw(...)` are now `unsafe`, because raw callbacks/pointers can flow into later safe runtime paths.
- `JointBase::default()` now mirrors upstream Box2D defaults instead of a partial zeroed approximation.
- `ShapeDefBuilder::filter(...)` and `ChainDefBuilder::filter(...)` now take the crate-owned `Filter`; raw paths are explicitly named `filter_raw(...)`.
- `MassData` is now crate-owned, and its inertia field is exposed as `rotational_inertia`.
- `PrismaticJointDef::max_motor_torque(...)` was removed in favor of the correct `max_motor_force(...)` naming.

### Migration Notes
- If your code previously relied on raw ids or raw geometry structs, switch to the crate-owned wrapper types and use `from_raw(...)` / `into_raw()` only at explicit FFI seams.
- If you previously used `ContactIdExt`, import nothing extra and call the same helpers directly on `ContactId`.
- If you used raw event or user-data pointer helpers, update call sites to the new `*_raw` names.
- For per-frame gameplay loops, prefer the new `*_into` and `visit_*` forms over the owned-`Vec` convenience methods.
- If you expose Box2D config values through editor/tooling flows, call `validate()` on `WorldDef`, `BodyDef`, `ShapeDef`, `JointBase`, and concrete joint defs before creation.

### Changed
- The safe API now front-loads many obvious Box2D preconditions instead of depending on upstream assert-enabled builds; recoverable `try_*` paths return `ApiError::InvalidArgument` or `ApiError::InCallback` where appropriate.
- Event access is now clearer and more symmetric: owned snapshots support buffer reuse, zero-copy views and raw event slices have matching `try_*` variants, and `WorldHandle` intentionally mirrors only the owned snapshot side.
- Examples, threading guidance, and error-handling docs were refreshed to emphasize the recommended safe-wrapper workflows instead of older raw-FFI-adjacent patterns.
- The optional Dear ImGui testbed stack was refreshed to the current `dear-imgui-*` generation used by the repository.

### Fixed
- World-space joint builders no longer drop previously configured base fields such as `collide_connected` when filling runtime-computed body ids and local frames.
- Wrong-family typed joint `try_*` calls now fail with `ApiError::InvalidJointType` instead of relying on upstream Box2D family asserts alone.
- `cgmath::Basis2 -> Rot` round-tripping no longer flips the angle sign.

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
