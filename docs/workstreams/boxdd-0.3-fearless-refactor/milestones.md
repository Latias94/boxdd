# boxdd 0.3 Fearless Refactor Milestones

## M1: Hot-Path Buffer Reuse

Status: shipped

Scope:

- reusable-buffer query APIs
- reusable-buffer extraction APIs
- shared callback collection helpers
- shared FFI `Vec` fill helpers
- tests, docs, examples, changelog, and version updates

Exit criteria:

- hot-path query and extraction APIs no longer force fresh `Vec` allocation
- `World` and `WorldHandle` expose the same reusable-buffer query surface
- docs clearly explain owned-returning vs reusable-buffer usage

Reference:

- `docs/workstreams/query-buffer-reuse/`

## M2: Character Mover Safe Surface

Status: shipped

Scope:

- `collide_mover` / `collide_mover_into` / `try_*` APIs
- safe plane and solver result value types
- safe `solve_planes` and `clip_vector`
- tests and examples for the full mover pipeline

Exit criteria:

- users can implement the Box2D character mover flow without reaching for raw FFI
- plane collection supports caller-owned buffer reuse
- examples demonstrate the intended safe workflow clearly

## M3: Surface Coherence Audit

Status: in progress

Scope:

- review remaining allocation-sensitive APIs
- reusable-buffer audit and cleanup for debug draw command collection
- review `World` / `WorldHandle` duplication and consolidate the mirrored query surface where the API intentionally stays symmetric
- review owned/scoped handle duplication outside the hottest paths
- consolidate the most mechanical `Shape` / `OwnedShape`, `Body` / `OwnedBody`, and `Chain` / `OwnedChain` internals behind shared private helpers
- consolidate the most mechanical joint creation entrypoints so joint-type additions cannot drift across scoped/id/owned/try variants
- consolidate event-buffer borrow / cleanup plumbing so all event-view APIs share the same lifetime and deferred-destroy path
- consolidate world-level shape create/edit helper families so geometry-type additions cannot drift across create/owned/try setter variants
- consolidate shared debug-draw callback bridging so safe/raw draw paths cannot drift in panic forwarding, callback locking, or option wiring
- document the remaining intentional raw escape hatches and keep callback-sensitive raw paths under regression tests
- consolidate the remaining high-churn joint-handle internals so scoped and owned joint handles share the same helper path for user data and threshold/state accessors

Exit criteria:

- the remaining duplication backlog is explicitly categorized as worth keeping or worth removing
- no obvious per-frame allocation trap remains undocumented or unaddressed on the main safe surface
- high-churn owned/scoped handle pairs no longer duplicate the same FFI access logic across every hot-path accessor
- joint creation families no longer duplicate per-type create/owned/id/try plumbing or callback-state handling
- event-view APIs no longer duplicate the borrow-event-buffers / process-deferred-destroys template in every module
- world-level shape create/edit families no longer duplicate the same geometry-to-FFI plumbing for each geometry type
- safe/raw debug-draw paths no longer duplicate the same callback panic bridge and option wiring, and the remaining raw path has direct regression coverage
- the remaining intentional raw surfaces are explicitly documented instead of being discovered only by source spelunking
- joint handles no longer duplicate the same user-data and threshold/state FFI plumbing across owned/scoped variants

## M4: Advanced Wrapper Coverage

Status: shipped

Scope:

- typed friction callback API
- typed restitution callback API
- standalone `collision` module for shape proxies, GJK distance, shape cast, and TOI
- `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)`

Exit criteria:

- advanced collision customization and low-level geometry algorithms no longer require raw `ffi` for normal use
- the next post-`0.3` wrapper-coverage push has a concrete backlog instead of scattered notes

## M5: Geometry Type Unification

Status: shipped

Scope:

- replace raw `ffi` geometry helper outputs in `shapes::helpers` with crate-owned geometry value types where practical
- review whether shape-creation entrypoints should accept the same geometry vocabulary used by `boxdd::collision`
- replace raw shape getter/setter geometry surfaces with the same crate-owned value types
- add standalone low-level helpers on geometry values where the upstream C API already exposes them directly
- make raw conversion on crate-owned geometry values explicit via `from_raw(...)` / `into_raw()`

Exit criteria:

- users can move between shape construction and standalone collision algorithms without dropping to raw `ffi`
- the remaining raw geometry exposure is explicit, narrow, and justified
- geometry values no longer rely on implicit raw `From` conversions on the public API boundary

## M6: Value-Type Coherence Audit

Status: shipped

Scope:

- review remaining public raw Box2D value types such as `ShapeType`, `MassData`, and contact-data structs
- convert the remaining user-facing value types to crate-owned wrappers where the safe API should own the vocabulary
- rename raw escape hatches explicitly with `*_raw` suffixes where keeping them is still justified

Exit criteria:

- `ShapeType`, `MassData`, `ContactData`, `Manifold`, and `ManifoldPoint` no longer leak raw `ffi` types through the main safe API
- raw escape hatches for shape type and contact extraction are explicit instead of silently sharing the primary method names

## M7: Remaining Raw Surface Audit

Status: in progress

Scope:

- review remaining public raw escape hatches such as `world_id`, raw event slices, and debug draw hooks
- make remaining crate-owned value types cross the raw boundary explicitly where the wrapper owns the vocabulary
- finish any obviously missing value-type/productization gaps left after the main `0.3` wrapper passes
- audit thread-model / async guidance so `worker_count`, worker-thread callbacks, and `World: !Send/!Sync` are documented together
- audit math interop completeness so `mint` stays aligned with the crate-owned `Vec2` / `Rot` / `Transform` / `Aabb` vocabulary
- clarify panic-by-default vs `try_*` error-handling guidance at the crate boundary
- productize live shape runtime wrappers for AABB, point tests, ray casts, computed mass data, and runtime event toggles across owned/scoped/id APIs
- productize the body runtime completeness slice around rotation, sleeping/awake/enabled/bullet/name state, attached ids, and body-level event toggles
- productize the first joint runtime completeness slice around joint metadata, constraint tuning, local frames, and wake helpers across owned/scoped/id APIs
- productize the type-specific joint runtime completeness slice around distance/prismatic/revolute/weld/wheel/motor getters/setters across owned/scoped/id APIs
- make typed joint `try_*` APIs reject wrong joint families with `ApiError::InvalidJointType` instead of depending on upstream asserts
- keep world-space joint builders coherent when they compute body ids / local frames at build time so previously configured base flags are preserved

Exit criteria:

- the remaining raw public surface is either clearly intentional or scheduled for removal
- crate-owned value types no longer rely on implicit raw conversions except for documented input-side or raw-escape-hatch exceptions
- the next completeness pass has a short, explicit backlog instead of scattered notes
- thread-model guidance no longer implies that internal worker threads make the public world API thread-safe
- math interop documentation and tests cover the intended `mint` bridge story explicitly
- common live-shape runtime queries and toggles no longer require raw `ffi` or an upstream-only mental model
- common body runtime controls and attached-id enumeration no longer require handle-only workarounds or ad-hoc allocations
- common joint runtime metadata and control no longer require raw `ffi` or per-handle-style workarounds
- type-specific joint runtime state and control no longer require world-only helpers, raw `ffi`, or upstream joint-family knowledge
- wrong-family typed joint `try_*` misuse reports `ApiError::InvalidJointType` instead of depending on Box2D assert builds
