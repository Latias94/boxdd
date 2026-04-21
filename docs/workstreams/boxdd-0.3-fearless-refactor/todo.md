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
- [x] Productize live shape runtime wrappers for AABB, point tests, ray casts, computed mass data, runtime event toggles, and the missing `try_sensor_overlaps_valid` symmetry.
- [x] Productize the body runtime completeness slice: rotation, sleep/awake/enabled/bullet/name controls, attached `shapes/joints` enumeration with reusable buffers, and body-level contact/hit event toggles.
- [x] Productize the first joint runtime completeness slice: joint type/body ids, `collide_connected`, constraint tuning, local frames, and wake helpers across `Joint`, `OwnedJoint`, and `World::joint_*`.
- [x] Fix world-space joint builders so runtime-computed body ids/local frames preserve previously configured base flags such as `collide_connected`.
- [x] Productize the type-specific joint runtime completeness slice: distance/prismatic/revolute/weld/wheel/motor getters/setters across `Joint`, `OwnedJoint`, and `World`, and return `ApiError::InvalidJointType` for wrong-family `try_*` calls.
- [x] Productize the remaining world runtime extras slice: `Profile` timings, `ExplosionDef`/`World::explode`, speculative collision control, and matching `try_*` coverage for callback-sensitive world tuning helpers.
- [x] Close the body/world-handle follow-up gaps: `BodyBuilder::allow_fast_rotation`, computed body AABB across owned/scoped/id views, and read-only `WorldHandle` mirrors for runtime diagnostics/tuning getters.
- [x] Close the `mint` rotation round-trip gap so crate-owned `Rot` values support recoverable inbound conversion from row/column-major `mint` rotation matrices.
- [x] Close the remaining obvious rotation interop asymmetry by adding recoverable inbound conversion from `glam::Mat2` to crate-owned `Rot`, and add round-trip coverage for `cgmath` / `nalgebra` rotation adapters.
- [x] Close the remaining callback-registration symmetry gap so custom filter / pre-solve setup and compatibility `*_callback` helpers expose matching `try_*` variants instead of forcing panic-only mutation.
- [x] Audit the remaining upstream world-level C API helpers and record that `b2World_DumpMemoryStats` / `b2World_RebuildStaticTree` stay intentionally unwrapped for `0.3.0` because they are debug/internal-only seams, not core safe-wrapper surface area.
- [x] Add reusable-buffer and recoverable `try_*` snapshot APIs for world body/contact/sensor/joint events so callers can keep owned event data without per-frame allocation churn.
- [x] Record the `World` vs `WorldHandle` event split as an intentional `0.3.0` design decision: borrowed/raw event reads stay on `World`, and any `WorldHandle` mirror should be limited to owned snapshots only.
- [x] Narrow the serialize-time chain metadata seam so `ChainDef` raw point/material helpers stay crate-private and `World::chain_records()` returns crate-owned `Filter` / `Vec2` / material-layout values instead of raw `ffi` collections.
- [x] Productize the remaining shape-material geometry seams: add safe rounded-box polygon helpers and make `SurfaceMaterial` a full crate-owned value type with getters plus explicit `from_raw(...)` / `into_raw()` conversion.
- [x] Complete the definition-side value-object cleanup so `ShapeDef` has read-side getters, `ChainDef` exposes safe points/filter/material-layout inspection, and both builders can resume from an existing definition value.
- [x] Continue creation-time definition cleanup so `BodyDef`, `JointBase`, and concrete `*JointDef` types are inspectable value objects, and remove the misnamed prismatic `max_motor_torque(...)` creation alias in favor of `max_motor_force(...)`.
- [x] Finish the world-config slice so `WorldDef` and `ExplosionDef` are readable value objects instead of builder-only or write-only configuration shells.
- [x] Audit raw definition-object safety boundaries so pointer/callback-bearing config wrappers stop re-entering the safe API through safe `from_raw(...)`, and add explicit `WorldDef` validation plus world-tuning argument checks.
- [x] Tighten the remaining raw pointer user-data escape hatches so body/shape/joint APIs use explicit `*_raw` naming and keep regression coverage for the preserved pointer seam.
- [x] Tighten raw event-buffer visitors so direct FFI-slice access uses `with_*_events_raw(...)` naming instead of blending in with the safe zero-copy event views.
- [x] Finish raw-boundary symmetry for configuration wrappers so `BodyDef`, `ShapeDef`, `JointBase`, and concrete joint defs all expose named `from_raw(...)` / `into_raw()` escape hatches.
- [x] Extend the reusable-buffer story to wrapper-owned serialize metadata so `World::body_ids()` and `World::chain_records()` also have `*_into(...)` / `try_*_into(...)` forms for allocation-sensitive tooling.
- [x] Extend callback-sensitive world execution helpers with matching `try_*` entrypoints so stepping, deferred-destroy flushing, and debug draw are not panic-only APIs.
- [x] Extend borrowed event-buffer APIs with matching `try_*` entrypoints so zero-copy views and raw event slices are not panic-only corners of the runtime surface.
- [x] Align the remaining raw contact-data buffer APIs with the explicit `*_raw` naming scheme by renaming `contact_data_into_raw(...)` to `contact_data_raw_into(...)`, and consolidate the body/shape handle implementations behind shared private definitions.
- [x] Continue the owned/scoped duplication audit by consolidating the remaining high-frequency enumeration helpers (`Body::{shape_count,shapes,joint_count,joints}`, `Shape::sensor_overlaps*`, and `Chain::segments*`) behind single private definitions without changing the public API.
- [x] Continue the feature-gated duplication audit by consolidating the `unchecked` body/shape/joint/chain extension trait implementations so owned/scoped handles share the same internal raw FFI definitions.
- [x] Close the remaining obvious contact inspection gap by adding direct safe `ContactId` helpers plus `ApiError::InvalidContactId` instead of forcing users back to raw `ffi::b2Contact_*`.
- [x] Add a release-level completeness matrix that classifies major wrapper areas as `safe-covered`, `raw-only`, `intentional omission`, or `candidate after 0.3`.
- [x] Expand `WorldHandle` event support with owned snapshots only (`*_events`, `*_events_into`, `try_*`) while keeping borrowed/raw event-buffer APIs on `World`.
- [x] Replace the temporary `World` / `WorldHandle` event-snapshot macro layer with private free-function helpers so the mirror stays explicit and aligned with the workstream's anti-macro duplication rules.
- [x] Replace the remaining `Body` / `Shape` / `Chain` public helper macros (`contact_data`, attachment enumeration, sensor overlaps, chain segments) with private free-function helpers plus explicit owned/scoped method definitions.
- [x] Consolidate the remaining `Body` / `Shape` / `Joint` owned-vs-scoped identity and user-data helper layers (`world_id_raw`, `parent_chain_id`, `is_valid`, raw pointer access, typed user data) behind shared private functions.
- [x] Replace the mirrored `World` / `WorldHandle` query macro layer with private checked-query helpers plus explicit method definitions for the reusable-buffer and mover/query surface.
- [x] Replace the remaining `World` / `WorldHandle` read-only getter macros (`gravity`, runtime snapshots, tuning getters) with private checked-read helpers plus explicit method definitions.
- [x] Remove the last `world.rs` shape create/set macro layer so world-owned geometry creation and editing helpers use ordinary private functions plus explicit methods.
- [x] Replace the `joints/mod.rs` world joint-creation macro layer with generic private creation helpers plus explicit per-family scoped/id/owned/`try_*` methods.
- [x] Start decomposing the typed joint runtime macro layer by replacing the `Distance` joint family with shared kind-checked helpers plus explicit `World` / `OwnedJoint` / `Joint` methods.
- [x] Remove the remaining typed-joint runtime public macro layer by replacing the `Prismatic`, `Revolute`, `Weld`, `Wheel`, and `Motor` families with explicit methods and deleting the obsolete macro definitions.
- [x] Replace the remaining `unchecked.rs` feature-gated handle implementation macros with shared private `unsafe` helpers plus explicit `World` / owned / scoped trait impls.
- [x] Productize the remaining global Box2D utility slice with public deterministic math/version helpers plus `Vec2` / `Rot` / `Transform` / `Plane` validity checks instead of leaving tests and advanced callers on raw `ffi`.
- [x] Expand polygon-construction helpers with square, offset-box, offset-rounded-box, offset-from-points, and hull-validation APIs so crate-owned geometry no longer needs raw FFI for those upstream builders.
- [x] Remove the remaining low-level joint scalar/vector runtime macro layer in `joints/mod.rs` by consolidating it behind private generic FFI helpers plus explicit per-joint impl functions.
- [x] Replace the `material_mix_registry` trampoline macro with const-generic callback trampolines so the crate no longer relies on internal `macro_rules!` expansion for callback dispatch tables.
- [x] Refactor `serialize.rs` to reuse crate-owned body/joint/shape runtime helper layers instead of duplicating raw FFI getters for snapshots and scene extraction.
- [x] Make core math value types (`Vec2`, `Rot`, `Transform`) use explicit `from_raw(...)` / `into_raw()` APIs instead of implicit raw `From` conversions so the crate-owned vocabulary has one consistent FFI boundary rule.
- [x] Productize the remaining low-risk global foundation helpers (`allocated_byte_count`, timing ticks/milliseconds, deterministic hashing, and float validation) while explicitly deferring process-wide allocator/log/assert hooks past `0.3`.
- [x] Add zero-allocation overlap visitor APIs (`visit_overlap_*` / `try_visit_overlap_*`) and route overlap collection helpers through the same visitor core so hot-path query styles cannot drift.
- [x] Productize opaque Box2D ids (`BodyId`, `ShapeId`, `JointId`, `ChainId`, `ContactId`) as crate-owned value types with explicit `from_raw(...)` / `into_raw()` boundaries, and remove the remaining mixed raw-id seams from the safe API.
- [x] Sweep the top-level examples and testbed scenes for low-value raw FFI usage now covered by the safe API, and switch body-velocity reads, shape distance, world counters, and revolute limits over to the crate-owned wrapper surface.
- [x] Expand `WorldHandle` from world-level diagnostics into a true stored read-only body query helper by mirroring the pure `body_*` runtime getter/enumeration surface and routing it through shared body helper implementations.
- [x] Expand `WorldHandle` further into a stored read-only shape query helper by mirroring the pure `shape_*` runtime getter/enumeration surface used after world/query `ShapeId` results and routing it through shared shape helper implementations.
- [x] Expand `WorldHandle` again into a stored read-only common joint query helper, and close the remaining world-id joint threshold gap so common joint runtime metadata/control no longer drifts between owned/scoped/id styles.
- [x] Finish the `WorldHandle` joint follow-up by mirroring the typed distance/prismatic/revolute/weld/wheel/motor read-only runtime getter surface so `body_joints()` results can stay on the stored-handle path for family-specific inspection too.

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
- [x] Normalize live chain surface-material counts and indexing around visible segments so open-chain ghost placeholder entries no longer leak through the safe runtime API.
- [x] Front-load obvious Box2D assert preconditions in the safe runtime surface so shape numeric setters and joint range/limit setters return recoverable errors instead of depending on upstream assert builds.
- [x] Front-load obvious creation-time definition preconditions in the safe wrapper (`BodyDef`, `ShapeDef`, shared `JointBase`, and concrete joint defs), and fix `JointBase::default()` to mirror upstream Box2D defaults instead of a partial zeroed approximation.
- [x] Front-load the remaining obvious Box2D assert preconditions on `World::step`, world query/cast/mover entrypoints, and standalone collision inputs so invalid vectors/AABBs/radii/fractions fail in the Rust wrapper before crossing FFI.
- [x] Add recoverable standalone collision validation via `ShapeProxy::try_new(...)`, `validate()` on collision input value objects, and `try_shape_distance` / `try_shape_cast` / `try_time_of_impact`.
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
- [x] Add more targeted regression coverage where intentional raw escape hatches still rely on callback-sensitive or zero-copy behavior.
- [x] Continue the completeness audit against upstream Box2D v3 and record any intentionally unwrapped or raw-only areas that should be revisited after `0.3.0`.
- [x] Re-evaluate the lightweight contact-inspection slice: move `ContactIdExt` onto `ContactId` as inherent methods and record that a first-class `Contact` handle remains an intentional omission for `0.3.0`.
- [ ] Revisit the remaining `candidate after 0.3` entries from the completeness matrix and decide which ones deserve the first post-`0.3` wrapper pass.
- [x] Keep the task-system / multithread stepping seam explicit and raw-only for `0.3`, but add named `unsafe` `WorldDef` / `WorldBuilder` helpers so advanced users no longer need direct `ffi::b2WorldDef` field surgery.

## Release Checklist

- [x] Run `cargo fmt --all`.
- [x] Run targeted mover tests.
- [x] Run `cargo nextest run -p boxdd`.
- [x] Review `README.md` and examples for `0.3.0` API consistency.
- [x] Finalize `CHANGELOG.md` wording for the full `0.3.0` release.
- [ ] Publish `boxdd 0.3.0`.
