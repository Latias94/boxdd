# boxdd 0.3 Fearless Refactor TODO

## Done

- [x] Ship reusable-buffer `*_into` / `try_*_into` query APIs for high-frequency world queries.
- [x] Ship reusable-buffer extraction APIs for contact data, sensor overlaps, world sensor overlap reads, and chain segments.
- [x] Add shared callback collection and FFI `Vec` fill helpers to remove repeated low-level plumbing.
- [x] Update docs, examples, versioning, tests, and changelog for the first `0.3.0` slice.
- [x] Record the broader `0.3.0` refactor scope in a dedicated workstream.

## In Progress

- [x] Add safe character mover plane collection APIs on `World` and `WorldHandle`.
- [x] Add safe mover-related value types:
  `Plane`, `MoverPlaneResult`, `CollisionPlane`, and `PlaneSolverResult`.
- [x] Add safe `solve_planes` and `clip_vector` helpers without requiring direct `ffi` usage.
- [x] Update the mover example to demonstrate the full safe workflow instead of only `cast_mover`.
- [x] Add mover-focused tests, including reusable-buffer behavior for plane collection.
- [x] Update `CHANGELOG.md` to reflect the expanded `0.3.0` scope.
- [x] Add typed safe friction / restitution mixing callbacks with panic forwarding and `user_material_id` inputs.
- [x] Add tests for material mixing callbacks, including panic propagation.
- [x] Add a standalone safe `collision` module for distance, shape cast, TOI, and reusable value types.
- [x] Add `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)` without exposing raw `ffi`.
- [x] Move low-level collision/AABB tests over to the public safe API.
- [x] Normalize body transform access around safe `Transform` and rename raw accessors explicitly.
- [x] Replace raw shape helper outputs and shape create/get/set APIs with crate-owned geometry value types.
- [x] Normalize shape and chain builder filter setters around the safe `Filter` type.
- [x] Move standalone shape geometry tests to the new safe geometry values.
- [x] Replace leaked raw value types on hot paths: `ShapeType`, `MassData`, `ContactData`, `Manifold`, and `ManifoldPoint`.
- [x] Rename raw shape/contact escape hatches explicitly with `*_raw` suffixes.
- [x] Productize standalone manifold collision helpers for circle/capsule/segment/polygon geometry in `boxdd::collision`.
- [x] Move the testbed manifold viewer over to the public safe collision API.
- [x] Replace `ffi::b2MotionLocks` with crate-owned `MotionLocks`.
- [x] Add a crate-owned `ChainSegment` geometry type and productize chain-segment manifold collision helpers.
- [x] Add reusable-buffer debug draw command collection and reuse nested polygon/string storage on stable command streams.
- [x] Rename raw world-id escape hatches explicitly to `world_id_raw` / `try_world_id_raw`.
- [x] Replace `ffi::b2HexColor` with crate-owned `HexColor` across debug-draw callbacks and collected commands.
- [x] Make crate-owned geometry raw conversions explicit with `from_raw(...)` / `into_raw()` for `Circle`, `Segment`, `ChainSegment`, `Capsule`, and `Polygon`.
- [x] Make `MassData` and `MotionLocks` use explicit raw conversions instead of implicit `From<ffi::...>` impls.
- [x] Make `Filter` use explicit raw conversions instead of implicit `From<ffi::b2Filter>` impls.

## Next

- [ ] Review remaining `World` / `WorldHandle` query duplication and decide whether selective consolidation is worth it.
- [ ] Audit remaining owned/scoped handle duplication outside the already-refactored hot paths.
- [ ] Review remaining public raw escape hatches and document which are intentional (`world_id_raw`, raw event slices, debug draw raw paths, etc.).
- [ ] Audit remaining crate-owned value types that still rely on implicit raw conversions and decide which should move to explicit `from_raw(...)` / `into_raw()` in `0.3`.

## Release Checklist

- [x] Run `cargo fmt --all`.
- [x] Run targeted mover tests.
- [x] Run `cargo nextest run -p boxdd`.
- [x] Review `README.md` and examples for `0.3.0` API consistency.
- [x] Finalize `CHANGELOG.md` wording for the full `0.3.0` release.
- [ ] Publish `boxdd 0.3.0`.
