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
- add zero-allocation visitor-style overlap queries so hot paths can stream hits or short-circuit without building result vectors
- review `World` / `WorldHandle` duplication and consolidate the mirrored query surface where the API intentionally stays symmetric
- review owned/scoped handle duplication outside the hottest paths
- consolidate the most mechanical `Shape` / `OwnedShape`, `Body` / `OwnedBody`, and `Chain` / `OwnedChain` internals behind shared private helpers
- normalize live chain material helpers around visible segment indexing so open-chain ghost placeholder entries stay a `ChainDef` detail instead of leaking through the runtime API
- normalize obvious Box2D assert preconditions into explicit safe-wrapper argument validation where the public runtime API already owns the semantics
- extend that validation policy to creation-time definition objects and shared default constructors so `try_*` creation paths do not depend on native assert builds
- extend that validation policy to `World::step`, world query/cast/mover entrypoints, and standalone collision inputs so the last obvious runtime assert pockets are encoded in Rust-side contracts too
- close the remaining safe mover-pipeline contract gap by validating `solve_planes` / `clip_vector` inputs and exposing recoverable `try_*` variants for the solver stage too
- consolidate the most mechanical joint creation entrypoints so joint-type additions cannot drift across scoped/id/owned/try variants
- consolidate event-buffer borrow / cleanup plumbing so all event-view APIs share the same lifetime and deferred-destroy path
- add reusable-buffer event snapshot getters so owned event extraction does not force fresh allocations beside the existing zero-copy views
- consolidate world-level shape create/edit helper families so geometry-type additions cannot drift across create/owned/try setter variants
- consolidate shared debug-draw callback bridging so safe/raw draw paths cannot drift in panic forwarding, callback locking, or option wiring
- document the remaining intentional raw escape hatches and keep callback-sensitive raw paths under regression tests
- consolidate the remaining high-churn joint-handle internals so scoped and owned joint handles share the same helper path for user data and threshold/state accessors

Exit criteria:

- the remaining duplication backlog is explicitly categorized as worth keeping or worth removing
- no obvious per-frame allocation trap remains undocumented or unaddressed on the main safe surface
- overlap queries support all three intended hot-path styles: owned `Vec`, reusable-buffer `*_into`, and zero-allocation `visit_*`
- high-churn owned/scoped handle pairs no longer duplicate the same FFI access logic across every hot-path accessor
- live chain material count/get/set helpers no longer leak Box2D's open-chain ghost placeholder indexing through the safe runtime surface
- obvious range/value misuse on the main runtime setter surface no longer depends on Box2D assert builds for failure behavior
- obvious creation-time def misuse on body/joint creation paths no longer depends on Box2D assert builds, and shared joint-base defaults match upstream semantics
- `World::step` and the world query/cast/mover hot paths no longer depend on Box2D assert builds for invalid AABBs, vectors, radii, or sub-step counts
- the safe character mover pipeline now validates the solver/clipping stage too, instead of stopping at world collision-plane collection
- joint creation families no longer duplicate per-type create/owned/id/try plumbing or callback-state handling
- event-view APIs no longer duplicate the borrow-event-buffers / process-deferred-destroys template in every module
- owned event snapshots no longer force fresh allocation when callers need persistent copies instead of borrowed event views
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
- recoverable validation for standalone collision inputs and `try_*` entrypoints for distance/shape-cast/TOI

Exit criteria:

- advanced collision customization and low-level geometry algorithms no longer require raw `ffi` for normal use
- standalone collision helpers expose both panic-by-default and recoverable validation paths instead of relying on upstream asserts for malformed input
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
- replace low-value example/testbed `boxdd_sys::ffi` call sites with equivalent safe APIs once the wrapper surface is complete enough
- make remaining crate-owned value types cross the raw boundary explicitly where the wrapper owns the vocabulary
- productize public opaque ids (`BodyId`, `ShapeId`, `JointId`, `ChainId`, `ContactId`) as crate-owned value types so the safe API stops leaking mixed raw-id seams
- align core math types (`Vec2`, `Rot`, `Transform`) with the same explicit raw-boundary rule as the rest of the crate-owned API
- close the remaining low-risk global foundation utility gap with byte-count, timing, yield, hash, and float-validation helpers
- build a release-level completeness matrix so the final `0.3.0` gap list is explicit instead of implicit
- close the remaining obvious `ContactId` gap with direct safe validation and data-fetch helpers
- expand `WorldHandle` event support with owned snapshots only, while keeping borrowed/raw event-buffer APIs on `World`
- expand `WorldHandle` beyond world-level diagnostics to pure body-by-id read-only runtime queries where no mutable borrow or step-buffer lifetime coupling is involved
- expand `WorldHandle` further to pure shape-by-id read-only runtime queries so query-produced `ShapeId` values can stay on the stored-handle path without bouncing back to `&World`
- expand `WorldHandle` further to common joint-by-id read-only runtime queries so `body_joints()` results can stay on the stored-handle path for metadata inspection
- expand `WorldHandle` further from common joint-by-id reads to typed joint-by-id read-only runtime queries so those same ids can stay on the stored-handle path for family-specific inspection too
- finish any obviously missing value-type/productization gaps left after the main `0.3` wrapper passes
- audit thread-model / async guidance so `worker_count`, worker-thread callbacks, and `World: !Send/!Sync` are documented together
- audit math interop completeness so `mint` stays aligned with the crate-owned `Vec2` / `Rot` / `Transform` / `Aabb` vocabulary
- clarify panic-by-default vs `try_*` error-handling guidance at the crate boundary
- make the `World` vs `WorldHandle` event API split explicit so it is treated as an intentional lifecycle/design choice instead of an accidental completeness gap
- remove the remaining serialize-time chain metadata leaks so `ChainDef` / `World::chain_records()` stop exposing raw `ffi::b2Vec2` / `b2SurfaceMaterial` collections where crate-owned value types already exist
- make definition-side value objects such as `ShapeDef` / `ChainDef` readable in the same crate-owned vocabulary used by their builders instead of forcing write-only configuration shells
- make creation-time config types such as `BodyDef`, `JointBase`, and concrete joint defs readable and correctly named instead of relying on builder-only mutation or legacy misnomers
- make top-level world config values such as `WorldDef` / `ExplosionDef` readable in the same crate-owned vocabulary instead of leaving them as setup-only shells
- tighten pointer/callback-bearing config raw re-entry so `BodyDef::from_raw(...)` / `WorldDef::from_raw(...)` become `unsafe`, and validate safe world configuration/tuning inputs before entering Box2D
- tighten remaining raw pointer user-data seams so their naming is as explicit as the rest of the `*_raw` surface
- tighten raw event-buffer visitors so borrowed FFI-slice APIs also use explicit `*_raw` naming instead of looking like ordinary safe zero-copy helpers
- finish raw-boundary symmetry for builder/config wrappers so the remaining config values do not require direct field access just to cross back to Box2D structs
- extend reusable-buffer extraction to wrapper-owned serialize metadata so the crate's own body/chain snapshots do not force fresh `Vec` allocation on every read
- extend recoverable error-handling symmetry to callback-sensitive world execution helpers so stepping, deferred-destroy flushing, and debug draw no longer require panic-only control flow
- extend the same recoverable error-handling symmetry to borrowed event-buffer entrypoints so zero-copy views and raw event slices return `ApiError::InCallback` instead of panicking by default
- productize live shape runtime wrappers for AABB, point tests, ray casts, computed mass data, and runtime event toggles across owned/scoped/id APIs
- productize the body runtime completeness slice around rotation, sleeping/awake/enabled/bullet/name state, attached ids, and body-level event toggles
- productize the first joint runtime completeness slice around joint metadata, constraint tuning, local frames, and wake helpers across owned/scoped/id APIs
- productize the type-specific joint runtime completeness slice around distance/prismatic/revolute/weld/wheel/motor getters/setters across owned/scoped/id APIs
- make typed joint `try_*` APIs reject wrong joint families with `ApiError::InvalidJointType` instead of depending on upstream asserts
- productize remaining world runtime extras such as `Profile`, explosions, speculative collision control, and matching `try_*` coverage for callback-sensitive world tuning and callback registration
- close the body/world-handle follow-up pass so `allow_fast_rotation`, computed body AABB, and read-only world runtime getters no longer require raw `ffi` or handle-style-specific workarounds
- keep world-space joint builders coherent when they compute body ids / local frames at build time so previously configured base flags are preserved

Exit criteria:

- the remaining raw public surface is either clearly intentional or scheduled for removal
- the release has a concrete completeness matrix instead of relying on scattered TODO bullets and source inspection
- `WorldHandle` mirrors owned event snapshots with a clear lifecycle boundary, while borrowed/raw event-buffer APIs remain intentionally `World`-only
- `WorldHandle` is usable as a stored read-only body query helper instead of being limited to world-level diagnostics only
- `WorldHandle` is also usable as a stored read-only shape query helper for the ids returned by world/query hot paths instead of forcing those follow-up reads back onto `&World`
- `WorldHandle` is also usable as a stored read-only common joint query helper for ids returned by body/world enumeration paths instead of forcing those follow-up reads back onto `&World`
- `WorldHandle` is also usable as a stored read-only typed joint query helper so family-specific runtime inspection does not have to bounce back to `&World` after `body_joints()` or similar id-producing paths
- crate-owned value types no longer rely on implicit raw conversions except for documented input-side or raw-escape-hatch exceptions
- body/shape/joint/chain/contact ids no longer leak raw `ffi::*Id` types through the normal safe wrapper surface
- the next completeness pass has a short, explicit backlog instead of scattered notes
- thread-model guidance no longer implies that internal worker threads make the public world API thread-safe
- math interop documentation and tests cover the intended `mint` bridge story explicitly
- the `WorldHandle` event surface has a concrete lifecycle story: owned snapshots are mirrored, borrowed/raw event buffers remain `World`-only
- serialize-time chain capture no longer exposes raw point/material/filter storage when crate-owned value/layout types already define the public vocabulary
- definition-side config values no longer require raw field knowledge or builder replay just to inspect material/filter/flag state
- creation-time config values no longer require source spelunking or replaying builder calls just to inspect body/joint setup, and wrong config names are removed instead of preserved indefinitely
- top-level world configuration values no longer require raw-field knowledge or one-shot builder calls just to inspect a setup object before reuse
- crate-owned `Rot` no longer has a one-way-only `mint` story; row/column-major rotation matrices round-trip with recoverable validation
- common live-shape runtime queries and toggles no longer require raw `ffi` or an upstream-only mental model
- common body runtime controls and attached-id enumeration no longer require handle-only workarounds or ad-hoc allocations
- computed body AABB, fast-rotation setup, and read-only world runtime getters are available on the main safe surface without `World`/`WorldHandle` drift
- common joint runtime metadata and control no longer require raw `ffi` or per-handle-style workarounds
- type-specific joint runtime state and control no longer require world-only helpers, raw `ffi`, or upstream joint-family knowledge
- wrong-family typed joint `try_*` misuse reports `ApiError::InvalidJointType` instead of depending on Box2D assert builds
- common world runtime diagnostics/tuning extras and callback-registration helpers no longer hide in side modules or panic-only seams when recoverable `try_*` behavior is appropriate
- `ContactId` no longer requires raw FFI for direct validity checks or contact-data inspection
