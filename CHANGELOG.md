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
- In-place valid-filter variants for sensor overlap hot paths: `sensor_overlaps_valid_into` and `shape_sensor_overlaps_valid_into`.
- Workstream documentation under `docs/workstreams/query-buffer-reuse/` to track the 0.3 allocation-hotpath refactor plan, milestones, and cleanup backlog.
- Safe character mover APIs covering collision-plane collection and solver helpers: `collide_mover`, `collide_mover_into`, `solve_planes`, and `clip_vector`.
- A broader 0.3 umbrella workstream under `docs/workstreams/boxdd-0.3-fearless-refactor/` to track the rest of the fearless refactor plan.

### Changed
- Query internals now share reusable collection helpers instead of duplicating callback-to-`Vec` plumbing across each query entrypoint.
- Temporary polygon proxy point collection now uses a stack-first `SmallVec` path for Box2D's fixed-size proxy vertex limit.
- Contact, sensor, and chain segment extraction now share a common FFI-backed `Vec` fill helper instead of repeating `with_capacity + set_len` logic across handle types.
- Sensor valid-filter paths now reuse a single caller-owned buffer and filter in place instead of allocating a second `Vec`.
- Examples and crate docs now show both reusable-buffer hot paths and the safe character mover workflow.

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
