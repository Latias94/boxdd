# Changelog

This project contains three crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources
- `bevy_boxdd`: Bevy ECS integration for `boxdd`

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

### Added
- Added a GitHub Pages experience for boxdd: the root page is now a Bevy Web example index, and each example route runs the shared Bevy + egui + `boxdd` WASM testbed in the browser.
- Added official Box2D sample-aligned Bevy WASM scenes for Stacking, Benchmark, Bodies, Continuous, Shapes, Events, and Joints, including Single Box, Tilted Stack, Circle Stack, Large Pyramid, Body Type, Kinematic, Skinny Box, Restitution, Friction, Filter, Sensor Funnel, Contact, Bridge, and Revolute.
- Added a generated Bevy testbed loader and provider shim that connect the Rust Bevy wasm module to the Emscripten `box2d-sys-v0` provider through shared `WebAssembly.Memory`.
- Added a browser-ready WASM provider runtime with `examples-wasm/provider-smoke` and `xtask provider-smoke`, so the shared-memory Box2D provider can be verified outside the Bevy Pages UI.
- Added Bevy entity-mapped AABB overlap helpers: `BoxddPhysicsContext::try_overlap_aabb_entities` and `try_overlap_aabb_entities_into` return `BoxddShapeHit` values with the native `ShapeId` and mapped Bevy `Entity`.
- Added Bevy product workflow examples for AABB overlap highlighting, debug draw rendering with Gizmos, compound child colliders, and collision category/mask filters.
- Added executable FFI lifecycle coverage for public `!Send`/`!Sync` guarantees, explicit destroy user-data cleanup, and raw pointer replacement semantics.
- Added `bevy_boxdd`, a Bevy 0.19 adapter crate with fixed-step body/collider sync, contact and sensor messages, recoverable plugin errors, ECS-authored distance and revolute joints, entity-mapped ray queries, debug draw command collection, and examples for the main Bevy workflows.
- Added `boxdd::dynamic_tree`, a safe owned wrapper for Box2D's standalone broad-phase dynamic tree.
- Added `xtask` validation for Box2D API coverage, official sample parity, and the generated Pages example index.
- Added machine-checked API coverage docs and fixtures for the vendored Box2D `B2_API` surface.
- Added `boxdd-sys` ABI/layout tests for representative FFI structs and smoke symbols.
- Added shape-specific standalone `shape_cast` / `try_shape_cast` helpers for circle, capsule, segment, and polygon geometry.
- Added default-running lifecycle tests for callback locks, panic containment, query callbacks, dynamic-tree callbacks, event-view deferred destruction, reusable event buffers, and world destroy/recycle behavior.

### Changed
- Updated dependency requirements to current crates.io releases: `nalgebra 0.35`, `glam 0.33`, and the native Dear ImGui testbed stack `dear-imgui-rs 0.15`, `dear-imgui-winit 0.15`, and `dear-imgui-glow 0.15`; refreshed `Cargo.lock` for the latest compatible transitive crates.
- Pages are generated from `bevy_boxdd/examples/testbed_2d/scenes.rs` so the published example list stays aligned with the runnable Bevy Web scenes.
- `xtask build-pages-wasm` now builds both the Emscripten Box2D provider assets and the Bevy testbed wasm assets required by Pages.
- Updated Pages CI to install `wasm-bindgen-cli 0.2.126` before building the browser runtime assets.
- Centralized workspace metadata and shared dependencies across `boxdd`, `boxdd-sys`, `bevy_boxdd`, and `xtask`.
- Updated the Bevy ray-query example to use entity-mapped `BoxddPhysicsContext` helpers instead of requiring users to combine native ray casts with manual shape-to-entity lookups.
- Tightened official sample parity so non-benchmark upstream samples must map to real Rust artifacts or carry an explicit deferral rationale.
- Reduced raw-only API coverage by adding safe wrappers for straightforward shape-cast and joint runtime gaps; remaining raw/omitted rows now carry explicit rationale.

### Fixed
- Fixed callback panic recovery so catching a Rust panic from a Box2D callback no longer poisons the internal panic payload mutex and prevents later `World::step` calls.

### Migration Notes
- If your app also depends directly on `nalgebra` or `glam` and uses `boxdd`'s optional interop features, align those direct dependencies with `nalgebra 0.35` and `glam 0.33` so conversions use the same crate versions.
- Native testbed integrations should use the `dear-imgui-* 0.15` crates with this workspace; browser Pages examples continue to use Bevy + egui instead of Dear ImGui.
- Browser builds should use `BOXDD_SYS_WASM_MODE=provider` for `wasm32-unknown-unknown`; the default wasm mode remains compile-only unless `BOXDD_SYS_WASM_CC` or a supported source toolchain is configured.
- Existing Bevy body, collider, contact, sensor, and transform-sync code does not need to change; the new Bevy APIs are additive.
- For Bevy ray casts, prefer `BoxddPhysicsContext::try_cast_ray_closest_entity`, `try_cast_ray_all_entities`, or `try_cast_ray_all_entities_into` when you want ECS entities back with the native hit data.
- For Bevy area queries, prefer `BoxddPhysicsContext::try_overlap_aabb_entities` or `try_overlap_aabb_entities_into` when you want ECS entities back with native `ShapeId` values.
- For Bevy joints, spawn a `JointDescriptor::distance(...)` or `JointDescriptor::revolute(...)` entity that references two entities with `RigidBody`; after fixed update, read `BoxddJoint` only if you need the native `JointId`.
- For Bevy debug rendering, collect `boxdd::DebugDrawCmd` values with `BoxddPhysicsContext::try_debug_draw_collect_into` and render them in your app or tooling, keeping renderer dependencies outside `bevy_boxdd`.

## [boxdd 0.4.0] - 2026-04-22

### Changed
- `Chain::{surface_material_count,surface_material,set_surface_material}` keep runtime-visible live-segment semantics on open chains, but that normalization now lives in Rust instead of a custom Box2D patch.
- The safe wrapper no longer depends on boxdd-only `b2Chain_*RuntimeSurfaceMaterial*` symbols in `boxdd-sys`.

### Notes
- `0.3.0` remains published as-is; this follow-up release is the clean path back to official upstream Box2D sources for repository checkouts and CI.

## [boxdd-sys 0.4.0] - 2026-04-22

### Changed
- Vendored Box2D sources realign with the official upstream submodule commit instead of a local-only patched commit.
- Removed the boxdd-specific runtime chain material FFI additions from the published bindings surface.

## [boxdd 0.3.0] - 2026-04-22

### Upgrade Summary
- Hot-path APIs are first-class now: keep the simple `Vec`-returning calls for one-off use, or move per-frame code to `*_into` and `visit_*`.
- The safe API now covers the full character-mover flow plus standalone collision helpers in `boxdd::collision`.
- Runtime coverage is much broader across `World`, `WorldHandle`, `Body`, `Shape`, `Joint`, and `ContactId`.
- Examples and testbed now teach the intended `0.3` workflows directly: reusable buffers, overlap vs cast queries, stored `WorldHandle` reads, dedicated physics-thread ownership, and optional `mint` interop.

### Breaking Changes
- Crate-owned ids, math types, geometry values, filters, manifolds, and contact data now cross the raw boundary explicitly through named `from_raw(...)` / `into_raw()` APIs.
- Geometry creation, editing, and geometry getters now use crate-owned safe geometry types instead of raw Box2D structs.
- Raw escape hatches were renamed to make FFI seams obvious, including names such as `world_id_raw`, `transform_raw`, `*_user_data_ptr_raw`, `with_*_events_raw(...)`, and `contact_data_raw_*`.
- `Body::transform` and `OwnedBody::transform` now return the safe `Transform` type; raw access moved to `transform_raw` / `try_transform_raw`.
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
- Math interop was expanded across `mint`, `glam`, `cgmath`, and `nalgebra`, and the threading/error-handling docs now make the intended single-owner physics model much clearer.
- The example catalog was reorganized by workflow, with focused samples for reusable-buffer queries, event snapshots vs views, stored `WorldHandle` reads, and optional `mint` interop.
- The optional Dear ImGui testbed stack was refreshed to the current `dear-imgui-*` generation used by this repository.
- The interactive testbed was reorganized around a central scene registry plus scene-local state blocks, and its overlap/cast/event/material demos now follow the intended `0.3` reusable-buffer and live-update workflows more faithfully with a clearer grouped control panel.

### Fixed
- World-space joint builders no longer drop previously configured base fields such as `collide_connected` when filling runtime-computed body ids and local frames.
- Wrong-family typed joint `try_*` calls now fail with `ApiError::InvalidJointType` instead of relying on upstream Box2D family asserts alone.
- `cgmath::Basis2 -> Rot` round-tripping no longer flips the angle sign.
- The materials testbed scene now keeps conveyor body ids and applies preset-driven transform updates correctly instead of silently desynchronizing the UI from the live world.

## [boxdd-sys 0.3.0] - 2026-04-22

### Changed
- Low-level bindings now cover the newer chain runtime surface-material functions used by `boxdd 0.3`.
- Release metadata now aligns with the `boxdd 0.3` publish order, so downstream verification resolves the intended `boxdd-sys 0.3.0` crate instead of the older `0.2.1` release.

### Notes
- No separate upstream Box2D revision change is introduced in this release beyond the binding surface needed by the safe wrapper.

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
