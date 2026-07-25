# Changelog

This project contains three crates:
- `boxdd`: safe, ergonomic Rust wrapper over the Box2D v3 C API
- `boxdd-sys`: low-level FFI bindings + vendored Box2D sources
- `bevy_boxdd`: Bevy ECS integration for `boxdd`

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [Unreleased]

This section is the draft for the coordinated `boxdd`, `boxdd-sys`, and `bevy_boxdd` 0.6.0
release. The release targets the pinned Box2D 3.2.0 development snapshot at
`56edae79f2949d86142b03450d5d60f63bcf5a6f`.

### Breaking Changes

- Live body, shape, joint, chain, and contact IDs are now bound to one Rust world registration.
  Direct `from_raw`/`into_raw` construction was replaced by authenticated `Raw*Id` values,
  `unbind`, and `World::bind_*_id`.
- Absolute world coordinates now use precision-aware `Position` and `WorldTransform`; `Vec2` and
  `Transform` remain local `f32` values. Enabling `double-precision` changes the complete native
  ABI and must be selected consistently across crates and providers.
- World overlap, ray, shape-cast, and mover queries now take an explicit absolute query origin.
  Query hits and debug draw points use `Position`.
- Standalone collision helpers now accept shape B relative to shape A and return `LocalManifold`.
  Runtime `ManifoldPoint` values expose local anchors instead of an implicitly narrowed absolute
  point.
- The process-global `set_length_units_per_meter` setter was removed. Call
  `initialize_foundation(FoundationConfig)` before the first safe native use.
- `CallbackWorld` and the `_with_ctx` callback APIs were removed. Worker callbacks receive only
  thread-safe IDs and values and cannot access owner-thread world state or typed user data.
- `WorldDef::worker_count` and its builder now use validated `WorkerCount`. Native builds use
  Box2D's built-in scheduler by default; current WASM adapters support one worker.
- The `serialize` feature, `serialize` module, registry-backed scene format, and
  `scene_serialize` example were removed. They have no compatibility shim.
- Joint definitions no longer wrap raw native definition structs. `JointBase::new` and checked
  world-anchor builders establish same-world and local-frame invariants before native creation.
- `BodyDef::into_raw() -> boxdd_sys::ffi::b2BodyDef` was removed. Use `into_raw_guard`, borrow the
  definition with `RawBodyDef::as_raw`, and keep the guard alive for the complete raw call.
- The permissive `pkg-config`, dynamic/name-only, and implicit system-library routes were removed.
  Non-vendored providers are explicit, static, exact-manifest adapters.
- Bevy transforms are local to a required `BoxddWorldOrigin` resource. Absolute queries, events,
  debug draw data, and joint anchors use `boxdd::Position`.
- The optional `cgmath` feature and its conversion/error APIs were removed because `cgmath 0.18`
  is unmaintained and affected by RustSec advisories RUSTSEC-2026-0196 and RUSTSEC-2026-0197.
  Migrate interop code to `glam`, `nalgebra`, or `mint`.
- `boxdd_sys::adapter::validate_snapshot` now returns `SnapshotValidationError`, separating a
  rejected adapter identity from a native snapshot status. Match `SnapshotValidationError::Status`
  when migrating code that previously compared the returned error directly with `SNAPSHOT_*`.
- Native body, shape, and joint type getters no longer assume that every provider output is a
  known closed-enum discriminant. Their `try_*` variants return
  `ApiError::InvalidNativeBodyType`, `InvalidNativeShapeType`, or `InvalidNativeJointType` and
  poison the world; the convenience getters retain their panic-on-error contract.

### Added

- Single- and double-precision generated bindings, ABI probes, world-space value types, and
  scalar-correct `mint`, `nalgebra`, `glam`, and `bytemuck` interop.
- `FoundationConfig`, panic-contained process hooks, frozen initialization, activity diagnostics,
  and coordinated ordinary/transient/exclusive-replay leases.
- Validated `WorkerCount`, `WorldCapacity`, world bounds/capacity inspection, runtime worker-count
  changes, contact recycling controls, orphan chain-segment handles, and zero-time-step behavior.
- Transactional `Snapshot` restore, integrity-checked `SnapshotImage` fresh-world loading,
  RAII `RecordingSession`, owned `Recording`, full-stream replay preflight, and epoch-bound
  `ReplayPlayer` views.
- `ReplayBodyView::try_body_type`, which reports an unknown native body type precisely and
  terminalizes the replay before any later native operation.
- Safe shape ray casts and standalone dynamic-tree AABB box casts through `TreeBoxCastInput`.
- Owned `BodyDef` names and creation-time sleep-threshold access through `BodyBuilder` and
  `BodyDef` getters.
- `cast_ray_closest_with_stats` and `try_cast_ray_closest_with_stats`, which retain broad-phase
  node/leaf visits on misses while the existing closest-ray APIs continue to return
  `Option<RayResult>`. Bevy exposes the corresponding entity-mapped result through
  `try_cast_ray_closest_entity_with_stats`.
- `BoxddWorldOrigin`, checked local/absolute conversions, atomic origin rebasing, and origin-aware
  Bevy transform, query, event, debug draw, and joint flows.
- A structured conformance contract for all 478 pinned exported functions plus ABI-bearing fields
  and callback capabilities.

### Changed

- `World`, `WorldHandle`, owned handles, callback owner state, snapshots, recording sessions, and
  replay players are explicitly owner-thread-only. Worker callback state is a separate
  `Send + Sync` capability with no world ownership.
- All C-to-Rust callbacks share panic containment, callback-depth tracking, conservative fallback,
  deferred destruction, and owner-boundary panic resumption.
- Snapshot images and recording streams are documented as compatibility-bound ABI artifacts, not
  long-term or cross-build persistence formats.
- Vendored source is the default provider; attested system, authenticated prebuilt, WASM provider,
  and compile-only WASM are distinct fail-closed adapters with no silent fallback.
- The workspace, package metadata, examples, interop features, and Bevy adapter target version
  `0.6.0` together with Rust 1.95 MSRV.

### Fixed

- Safe native output buffers reserve and validate actual capacity before C writes and publish only
  initialized elements.
- Wrong-world, recycled-slot, stale-generation, wrong-kind, and forged IDs are rejected before
  native access or registry mutation.
- Typed user-data closures no longer run under the global registry lock; conflicting reentry is
  reported and closure panics do not poison later access.
- Callback panics cannot unwind through C, and cleanup/drop panics are aggregated only after native
  teardown and lease release.
- Snapshot restore reconciles Safe IDs and typed user data transactionally; any failure after the
  native restore begins leaves a terminal world instead of a partially live one.
- Joint construction and runtime mutation validate finiteness, normalized axes, ordered limits,
  same-world bodies, and joint-family invariants before Box2D assertions can fire.
- Windows prebuilt release artifacts now use explicit `md` / `mt` CRT suffixes, and CI fails if a release build produces an ambiguous package name.

### Security

- Runtime-capable adapters verify exact upstream SHA, precision, target, CRT/SIMD/validation
  identity, source/archive digests, private ABI, snapshot layout, and recording contract before
  Safe Rust creates native state.
- Official prebuilt and WASM artifacts additionally require repository/workflow/commit/tag-bound
  publisher provenance. Caller-generated system manifests are explicitly local trust statements,
  not project authentication.
- External snapshot and recording bytes are checksummed and structurally/semantically preflighted
  before native restore or replay allocation.
- The public snapshot validator authorizes the linked runtime adapter before passing Rust-owned
  `SnapshotFacts` or `SnapshotEntry` output storage across the FFI boundary.
- The unmaintained, unsound optional `cgmath` integration was removed, and the yanked transitive
  `spin 0.10.0` package was advanced exactly to `0.10.1`; no general lockfile update was performed.

### Migration Notes

- This release intentionally provides no compatibility shims for obsolete 0.5 semantics.
- Follow [`docs/migration-0.5-to-0.6.md`](docs/migration-0.5-to-0.6.md) for API-by-API replacements,
  unsupported boundaries, provider selection, persistence, and Bevy world-origin migration.

## [0.5.0] - 2026-07-06

### Added
- Live Bevy + egui browser examples at <https://frankorz.com/boxdd/>. The examples cover common Box2D scenes such as Single Box, Large Pyramid, Restitution, Friction, Sensor Funnel, Bridge, and Revolute.
- `bevy_boxdd`, a Bevy 0.19 integration crate with fixed-step body and collider sync, contact and sensor messages, ECS-authored distance and revolute joints, entity-mapped ray/AABB queries, debug draw collection, and focused workflow examples.
- `boxdd::dynamic_tree`, a safe owned wrapper for Box2D's standalone broad-phase dynamic tree.
- Shape-specific standalone `shape_cast` / `try_shape_cast` helpers for circle, capsule, segment, and polygon geometry.

### Changed
- Browser examples now load smaller WASM builds by default and show byte-level download progress while the Box2D provider and Bevy runtime load.
- Updated optional math and native testbed dependencies to current releases: `nalgebra 0.35`, `glam 0.33`, and `dear-imgui-* 0.15`.
- More Box2D shape-cast and joint runtime APIs are available as safe wrappers, so fewer cases need `boxdd_sys::ffi`.

### Fixed
- Catching a Rust panic from a Box2D callback no longer poisons the internal panic payload mutex and prevents later `World::step` calls.

### Migration Notes
- Most `boxdd` 0.4 code can upgrade to 0.5 without source changes. The new Bevy and browser-example support is opt-in.
- If your app uses the optional `nalgebra` or `glam` features and also depends on those crates directly, update your direct dependencies to `nalgebra 0.35` and `glam 0.33`.
- If you build the native Dear ImGui testbed, update the Dear ImGui crates to `dear-imgui-rs 0.15`, `dear-imgui-winit 0.15`, and `dear-imgui-glow 0.15`.

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
