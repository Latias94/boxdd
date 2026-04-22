# Changelog

This project contains two crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

## [boxdd 0.3.0]

### Upgrade Summary
- Per-frame hot paths are first-class now. Keep the simple `Vec`-returning APIs for one-off use, or switch to `*_into` and `visit_*` when you want reusable buffers or zero result-container allocation.
- The safe API now covers Box2D's character mover flow end to end: `cast_mover`, `collide_mover`, `solve_planes`, `clip_vector`, and matching recoverable `try_*` variants.
- `boxdd::collision` is now a real standalone geometry surface for world-free distance queries, shape casts, TOI, manifolds, and `Aabb::ray_cast`, so advanced collision code no longer has to drop to `boxdd_sys::ffi`.
- Runtime coverage is much broader across `World`, `WorldHandle`, `Body`, `Shape`, `Joint`, and `ContactId`, including read-only `WorldHandle` follow-up queries, richer event access, runtime tuning, explosions, and typed joint state/control.
- Docs and examples now point users toward the intended `0.3` workflows: hot-path reuse, overlap vs cast queries, standalone collision, dedicated physics-thread ownership, stored `WorldHandle` reads, and optional `mint` math interop.

### Breaking Changes
- Crate-owned ids, math types, geometry values, filters, manifolds, contact data, and related helper values now cross the Box2D raw boundary explicitly through named `from_raw(...)` / `into_raw()` APIs instead of implicit conversions.
- Geometry creation, editing, and geometry getters now use crate-owned safe geometry types instead of raw Box2D structs.
- Raw escape hatches were renamed to make FFI seams obvious. Expect names such as `world_id_raw`, `transform_raw`, `*_user_data_ptr_raw`, `with_*_events_raw(...)`, `shape_type_raw`, and `contact_data_raw_*`.
- `Body::transform` and `OwnedBody::transform` now return the safe `Transform` type. Raw access moved to `transform_raw` / `try_transform_raw`.
- `ContactIdExt` was removed. Its inspection helpers now live directly on `ContactId`.
- Open-chain runtime material access now uses visible live-segment indexing instead of Box2D's ghost-placeholder storage layout.
- `BodyDef::from_raw(...)` and `WorldDef::from_raw(...)` are now `unsafe`, because raw callbacks and pointers can flow back into later safe runtime paths.
- `MassData` is now crate-owned, its inertia field is named `rotational_inertia`, and `PrismaticJointDef::max_motor_torque(...)` was removed in favor of `max_motor_force(...)`.

### Migration Notes
- If you previously passed raw Box2D ids or geometry structs through the safe wrapper, switch to the crate-owned wrapper types and keep raw conversions at explicit FFI seams only.
- If you used `ContactIdExt`, remove that import and call the same helpers directly on `ContactId`.
- If you used raw event, transform, or user-data-pointer helpers, update call sites to the new `*_raw` names.
- For gameplay code that runs every frame, prefer `*_into` and `visit_*` over the owned-`Vec` convenience APIs.
- For editor or tooling flows, call `validate()` on `WorldDef`, `BodyDef`, `ShapeDef`, `ChainDef`, `JointBase`, and concrete joint defs before creating runtime objects.

### Also Changed
- The safe API now front-loads many obvious Box2D preconditions instead of depending on upstream assert-enabled builds. Recoverable `try_*` paths return `ApiError::InvalidArgument`, `ApiError::InCallback`, or `ApiError::InvalidJointType` where appropriate.
- Event access is more consistent: owned snapshots support buffer reuse, zero-copy views and raw event slices have matching `try_*` forms, and `WorldHandle` intentionally mirrors only the owned-snapshot side.
- Math interop was expanded and rounded out across `mint`, `glam`, `cgmath`, and `nalgebra`, and the threading/error-handling docs now make the intended single-owner physics model much clearer.
- The example catalog was reorganized by workflow, and the focused samples now more clearly separate reusable-buffer queries, event snapshots vs views, stored `WorldHandle` reads, and optional `mint` interop.
- The optional Dear ImGui testbed stack was refreshed to the current `dear-imgui-*` generation used by this repository.
- The interactive testbed was reorganized around a central scene registry plus scene-local state blocks, and its overlap/cast/event/material demos now follow the intended `0.3` reusable-buffer and live-update workflows more faithfully with a clearer grouped control panel.

### Fixed
- World-space joint builders no longer drop previously configured base fields such as `collide_connected` when filling runtime-computed body ids and local frames.
- Wrong-family typed joint `try_*` calls now fail with `ApiError::InvalidJointType` instead of relying on upstream Box2D family asserts alone.
- `cgmath::Basis2 -> Rot` round-tripping no longer flips the angle sign.
- The materials testbed scene now keeps conveyor body ids and applies preset-driven transform updates correctly instead of silently desynchronizing the UI from the live world.

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
