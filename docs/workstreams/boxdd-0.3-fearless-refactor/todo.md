# boxdd 0.3 Fearless Refactor TODO

## Done

- [x] Ship reusable-buffer `*_into` / `try_*_into` query APIs for high-frequency world queries.
- [x] Ship reusable-buffer extraction APIs for contact data, sensor overlaps, world sensor overlap reads, and chain segments.
- [x] Add shared callback collection and FFI `Vec` fill helpers to remove repeated low-level plumbing.
- [x] Update docs, examples, versioning, tests, and changelog for the first `0.3.0` slice.
- [x] Record the broader `0.3.0` refactor scope in a dedicated workstream.
- [x] Audit whether async, multithreading, error handling, and math-interop changes belong in `0.3.0`, and keep the release boundary explicit instead of adding shallow abstractions.
- [x] Expand `mint` interop to cover rotation matrices plus column-major 2D transform matrices.
- [x] Document the threading / async model around `worker_count`, worker-thread callbacks, and dedicated physics-thread usage.
- [x] Align crate docs / README error-handling guidance around panic-by-default vs `try_*`.
- [x] Add a dedicated physics-thread example to demonstrate the recommended multi-thread integration pattern.

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
- [x] Consolidate shared `Shape` / `OwnedShape` internals for geometry, material, filter, and sensor-capacity accessors.
- [x] Consolidate shared `Body` / `OwnedBody` internals for state, transform, force/impulse, mass, and common flag accessors.
- [x] Consolidate shared `Chain` / `OwnedChain` internals for validity, segment/material access, and common raw escape hatches.
- [x] Make `BodyType`, `Aabb`, mover/query value types, collision outputs, and `Counters` use explicit `from_raw(...)` / `into_raw()` APIs where applicable instead of implicit raw conversions.
- [x] Make collision input value types (`DistanceInput`, `ShapeCastPairInput`, `Sweep`, `ToiInput`) cross the raw boundary explicitly with named `into_raw()` / `from_raw()` APIs instead of implicit conversions.
- [x] Make contact/manifold value types (`ManifoldPoint`, `Manifold`, `ContactData`) use explicit raw conversion APIs instead of implicit `From<ffi::...>` shims.
- [x] Remove the remaining compatibility `From` shims for `ShapeType` and `HexColor` in favor of their existing named raw conversion APIs.
- [x] Consolidate the mirrored `World` / `WorldHandle` query method definitions behind a single internal source so future query additions cannot drift between the two handle styles.
- [x] Consolidate the repeated joint creation entrypoints behind a single internal definition and normalize `try_create_*_joint*` to return `ApiError::InCallback` when called from callbacks.
- [x] Consolidate event-buffer borrow / deferred-destroy plumbing behind a shared `World` helper so body/contact/sensor/joint event views stay behaviorally aligned.
- [x] Consolidate the repeated world-level shape create/edit entrypoints so circle/segment/capsule/polygon helpers share a single internal definition.
- [x] Consolidate shared debug-draw callback panic plumbing and option wiring across the safe and raw debug-draw paths, and add dedicated `debug_draw_raw` regression coverage.
- [x] Consolidate shared `Joint` / `OwnedJoint` internals for common state accessors, threshold controls, and raw/typed user-data plumbing.

## Next

- [ ] Audit any remaining owned/scoped handle duplication outside the already-refactored internals and confirm it is worth keeping.
- [x] Review remaining public raw escape hatches and document which are intentional (`world_id_raw`, raw event slices, debug draw raw paths, etc.).
- [ ] Add more targeted regression coverage where intentional raw escape hatches still rely on callback-sensitive or zero-copy behavior.
- [ ] Continue the completeness audit against upstream Box2D v3 and record any intentionally unwrapped or raw-only areas that should be revisited after `0.3.0`.

## Release Checklist

- [x] Run `cargo fmt --all`.
- [x] Run targeted mover tests.
- [x] Run `cargo nextest run -p boxdd`.
- [x] Review `README.md` and examples for `0.3.0` API consistency.
- [x] Finalize `CHANGELOG.md` wording for the full `0.3.0` release.
- [ ] Publish `boxdd 0.3.0`.
