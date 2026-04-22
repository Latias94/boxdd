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
- collapse the mirrored `Body` / `OwnedBody` runtime wrapper bodies behind one private handle layer while keeping ownership-only seams (`core_arc`, `as_id`, destroy/drop) separate
- collapse the mirrored `Chain` / `OwnedChain` runtime wrapper bodies behind one private handle layer so checked read/write forwarding keeps one internal source of truth
- collapse the mirrored `Shape` / `OwnedShape` runtime wrapper bodies behind one private handle layer while keeping explicit ownership-only seams (`as_id`, destroy/drop, `update_body_mass_on_drop`) separate
- close the `OwnedBody` local creation parity gap so stored body handles can create owned shapes and chains directly instead of detouring through `World::create_*_for_owned`
- collapse the follow-up `Body` / `OwnedBody` local creation convenience plumbing so the new parity layer does not reintroduce internal drift
- delete the now-obsolete world-level shape-create forwarding layer and collapse world-owned creation wrappers onto the same owned-handle helper pattern
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
- collapse the mirrored `Joint` / `OwnedJoint` common runtime wrapper bodies behind one private handle layer while keeping ownership-only destroy/drop and wake-on-drop seams explicit
- collapse the mirrored typed joint family wrappers so `Distance`, `Prismatic`, `Revolute`, `Weld`, `Wheel`, and `Motor` each route owned/scoped runtime methods through one private handle layer
- split the stabilized joint runtime layer out of the oversized `joints/mod.rs` file so common runtime-by-id APIs and their helper plumbing live behind an explicit module boundary
- split the stabilized typed joint runtime layer out of `joints/mod.rs` as well so family-specific runtime logic has its own module boundary instead of remaining embedded beside joint creation and definition code
- continue the typed-runtime decomposition by peeling fully contiguous family slices out of `runtime_typed.rs`, starting with `Distance`, before reordering the still-interleaved `Prismatic` / `Revolute` sections
- continue that decomposition again by peeling the contiguous `Weld` / `Wheel` / `Motor` tail into its own module so only the interleaved `Prismatic` / `Revolute` slice remains in the shared typed-runtime file
- finish that decomposition by deleting the temporary shared `runtime_typed.rs` file and giving the remaining `Prismatic` / `Revolute` runtime slices their own dedicated modules too
- finish the typed-runtime decomposition all the way down to per-family modules by deleting the temporary `runtime_typed_weld_wheel_motor.rs` grouping file and splitting `Weld`, `Wheel`, and `Motor` into their own dedicated modules too
- split `joints/base.rs` along its natural responsibility boundary so joint-handle runtime code and `JointBase` definition-builder value objects stop cohabiting one file
- continue that `joints/base.rs` decomposition by splitting the runtime-handle trait, joint user-data helper layer, and owned/scoped wrapper bodies into dedicated child modules so the post-`JointBase` follow-up sink also becomes a thin coordination root
- split the remaining joint creation/validation layer out of `joints/mod.rs` so the public joint root stops cohabiting with all create-time validation and generic creation plumbing
- split the oversized `query.rs` module along its natural boundaries so query value types, validation, raw FFI plumbing, checked wrappers, and `World` / `WorldHandle` entrypoints stop cohabiting one file
- split the oversized `body.rs` module along its natural boundaries so body value objects, runtime helper plumbing, and owned/scoped handle wrappers stop cohabiting one file
- continue that `body/runtime.rs` decomposition by splitting contact/attachment extraction, typed user-data plumbing, and the shared owned/scoped runtime-handle trait into dedicated child modules so the main post-split body sink also becomes a thin coordination root
- split the `SurfaceMaterial` / `ShapeDef` / `ShapeDefBuilder` value-object block out of `shapes/mod.rs` so shape-definition builders stop cohabiting with the large runtime-heavy shape root
- split the public `Shape` / `OwnedShape` wrapper bodies and body-local shape creation entrypoints out of `shapes/mod.rs` so the shape root stops cohabiting value objects, wrapper methods, and creation convenience layers
- split the remaining internal shape runtime / validation / creation helper layer out of `shapes/mod.rs` so the shape root can become a thin public module root instead of the last large internal implementation sink
- continue that shape-runtime decomposition by splitting validation, body-attached creation plumbing, and user-data/checked helper layers out of `shapes/runtime.rs` into dedicated child modules instead of leaving a second large implementation sink behind
- split the `WorldDef` / `WorldBuilder` / world-configuration validation layer out of `world.rs` so top-level world configuration stops cohabiting with callback plumbing and the main runtime surface
- split lightweight world-owned value objects such as `Counters`, `Profile`, and owned-handle count snapshots out of `world.rs` so the main world module stops mixing passive snapshot structs with runtime entrypoints
- split `WorldHandle` / `CallbackWorld` plus the stored-read-only world/body/shape/joint query surface out of `world.rs` so the main world module stops cohabiting mutable runtime entrypoints with callback-safe or stored-query APIs
- split the callback-heavy `World` runtime control layer out of `world.rs` so stepping, gravity/diagnostic getters, tuning helpers, and callback registration live behind a dedicated `world/runtime.rs` module instead of inflating the root
- continue that world runtime decomposition by splitting read-only helper plumbing, step/tuning control, and callback registration/trampolines into `world/runtime/{reads,control,callbacks}.rs` so the first large world child module also becomes a thin coordination root
- continue the `world/handle.rs` follow-up decomposition by splitting callback-safe user-data reads plus the stored world/body/shape query slices into `world/handle/{callback_world,world_reads,body_reads,shape_reads}.rs` so the stored-read-only path also becomes a thin coordination root
- continue the `query/world_api.rs` follow-up decomposition by splitting the explicit `World` and `WorldHandle` query entrypoints into `query/world_api/{world_queries,handle_queries}.rs` so the mirrored query surface stays explicit without one oversized two-receiver file
- continue the `world/creation.rs` follow-up decomposition by splitting body lifecycle, world-space joint-base builders, and shape/chain creation helpers into `world/creation/{body_lifecycle,joint_builders,shape_creation}.rs` so the creation/lifecycle surface also becomes a thin coordination root
- continue the `shapes/geometry.rs` follow-up decomposition by splitting per-type geometry implementations into `shapes/geometry/{circle,segment,chain_segment,capsule,polygon}.rs` so the geometry surface can evolve by value type without one oversized helper-and-impl sink
- continue the `world/body_api.rs` follow-up decomposition by splitting pure body reads/enumeration and mutable state/control helpers into `world/body_api/{reads,control}.rs` so the body-id runtime surface also becomes a thin coordination root
- split the world-owned creation/lifecycle layer out of `world.rs` so body/shape/chain creation, destroy helpers, and world-space joint-base builders live behind a dedicated `world/creation.rs` module instead of inflating the root
- split the remaining body-by-id runtime helper block out of `world.rs` so `World` body-id queries/mutations live behind a dedicated `world/body_api.rs` module instead of remaining in the root
- split the callback-sensitive scoped id-borrow helpers out of `world.rs` so `body` / `shape` / `joint` / `chain` borrowing lives behind a dedicated `world/borrow.rs` module
- split the remaining shape-by-id runtime block out of `world.rs` so `World` shape-id helpers live behind a dedicated `world/shape_api.rs` module instead of remaining in the root
- split the inline world regression tests out of `world.rs` so the root can stay focused on implementation coordination instead of mixing runtime code with test bodies
- after that root slimming pass, continue the world decomposition by targeting the largest child modules instead of rebuilding another oversized root

Exit criteria:

- the remaining duplication backlog is explicitly categorized as worth keeping or worth removing
- no obvious per-frame allocation trap remains undocumented or unaddressed on the main safe surface
- overlap queries support all three intended hot-path styles: owned `Vec`, reusable-buffer `*_into`, and zero-allocation `visit_*`
- high-churn owned/scoped handle pairs no longer duplicate the same FFI access logic across every hot-path accessor
- mirrored `Body` / `OwnedBody` runtime wrappers now share one internal source for state queries, transforms, force/impulse mutation, attached-id enumeration, event toggles, names, contact snapshots, and user-data forwarding, while ownership-only behavior stays explicit
- mirrored `Chain` / `OwnedChain` runtime wrappers no longer duplicate checked world-id, validity, segment, and material forwarding logic
- mirrored `Shape` / `OwnedShape` runtime wrappers now share one internal source for validity, identity, event toggles, geometry, material/filter state, user-data, and hot-path contact/sensor forwarding, while ownership-only behavior stays explicit
- `OwnedBody` no longer forces local shape/chain creation back through world-owned helper entrypoints when the safe surface already owns the relationship
- the `OwnedBody` parity work does not leave a second duplicated convenience layer behind; local create helpers now share one private wrapper path
- world-level owned creation helpers no longer add a separate forwarding layer on top of the shared body-attached shape/chain creation internals
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
- mirrored `Joint` / `OwnedJoint` common runtime wrappers now share one internal source for validity, joint metadata, attached body ids, collide-connected control, constraint tuning, local frames, thresholds, wake helpers, and user-data forwarding, while ownership-only destroy/drop behavior stays explicit
- mirrored typed joint runtime wrappers for owned/scoped handles now share internal sources for typed getters/setters and validated range mutation across `Distance`, `Prismatic`, `Revolute`, `Weld`, `Wheel`, and `Motor`, instead of maintaining drifting handle-specific copies
- `joints/mod.rs` no longer mixes stable runtime infrastructure and creation/definition code in one monolithic block; the common runtime-by-id layer now has an explicit module home that future follow-up splits can build on
- `joints/mod.rs` also no longer carries the entire typed runtime layer inline; family-specific runtime FFI glue and wrapper forwarding now live behind a separate module boundary that future family-by-family cleanup can refine further
- the typed runtime split has started moving from one big follow-up file to family-focused modules, with the fully contiguous `Distance` slice already separated so the remaining interleaved families can be untangled incrementally instead of in one risky rewrite
- the same incremental split has now peeled off the contiguous `Weld` / `Wheel` / `Motor` tail too, so the remaining shared typed-runtime file is reduced to the `Prismatic` / `Revolute` slice that actually needs structural reordering rather than simple extraction
- the typed runtime family split is now complete: `Distance`, `Prismatic`, `Revolute`, `Weld`, `Wheel`, and `Motor` each live in dedicated modules, and the last temporary shared typed-runtime staging files have been deleted
- `joints/base.rs` now focuses on joint handles, `JointType`, and common runtime helpers, while `JointBase` / `JointBaseBuilder` live in a separate definition module instead of inflating the same file
- `joints/base.rs` is now thinner still: the runtime-handle trait, joint user-data helper layer, and owned/scoped wrapper bodies live in `joints/base/{runtime_handle,user_data,owned,scoped}.rs`, leaving the root focused on shared types and raw/common helper functions instead of another mixed-responsibility sink
- `joints/mod.rs` is now mostly a thin public root with re-exports and a few shared helpers, while joint creation-time validation and generic creation plumbing have their own dedicated module
- the broad-phase query surface now has explicit module boundaries: value types/solver helpers, raw FFI/query visitors, checked validation wrappers, and `World` / `WorldHandle` methods live in separate files instead of one oversized `query.rs`
- the body surface now has explicit module boundaries too: body definitions/builders, runtime helper plumbing, and owned/scoped handle wrappers live in separate files instead of one oversized `body.rs`
- `body/runtime.rs` is now thinner as well: contact/attachment extraction lives in `body/runtime/attachments.rs`, typed user-data plumbing in `body/runtime/user_data.rs`, and the shared owned/scoped runtime-handle trait in `body/runtime/handle.rs`, leaving the root focused on core raw/helper functions instead of another mixed-responsibility sink
- the shape-definition value-object block now has its own module too, so `shapes/mod.rs` no longer mixes every runtime wrapper with `SurfaceMaterial`, `ShapeDef`, and `ShapeDefBuilder`
- the public shape wrappers and body-local shape creation convenience layer now have explicit module homes too, so `shapes/mod.rs` no longer mixes wrapper method bodies with value objects and creation entrypoints
- the remaining internal shape runtime and validation plumbing now lives in `shapes/runtime.rs`, leaving `shapes/mod.rs` as a thin public root with re-exports plus shared imports for child modules
- `shapes/runtime.rs` is no longer a single monolithic follow-up sink either: validation now lives in `shapes/runtime/validation.rs`, body-attached creation plumbing in `shapes/runtime/creation.rs`, and user-data plus checked helper plumbing in `shapes/runtime/user_data.rs`, leaving the root focused on core runtime queries and the shared handle trait
- the world-configuration value-object layer now has its own module too, so `world.rs` no longer mixes `WorldDef` / `WorldBuilder` / config validation with the callback-heavy runtime body
- lightweight world-owned stats and snapshot value objects now live in a dedicated metrics module too, so `world.rs` no longer carries passive `Counters` / `Profile` / owned-handle count structs beside the live runtime surface
- `WorldHandle` / `CallbackWorld` and their stored-read-only query methods now live behind a dedicated handle module too, so `world.rs` no longer mixes mutable runtime entrypoints with callback-safe or stored-query read APIs
- the callback-heavy `World` runtime control layer now lives in `world/runtime.rs`, so `world.rs` no longer mixes step/flush logic, gravity/diagnostic getters, tuning setters, and callback registration with world creation/id plumbing
- `world/runtime.rs` is now thin as well: read-only helper plumbing lives in `world/runtime/reads.rs`, step/tuning control in `world/runtime/control.rs`, and callback registration/trampolines in `world/runtime/callbacks.rs`, leaving the root as a coordination module shared by `World` and `WorldHandle`
- `world/handle.rs` is now thin as well: callback-safe user-data reads live in `world/handle/callback_world.rs`, world-level diagnostics in `world/handle/world_reads.rs`, body-by-id reads in `world/handle/body_reads.rs`, and shape-by-id reads in `world/handle/shape_reads.rs`, leaving the root focused on type definitions and module coordination
- `query/world_api.rs` is now thin as well: `World` query entrypoints live in `query/world_api/world_queries.rs` and `WorldHandle` mirrors live in `query/world_api/handle_queries.rs`, leaving the root focused on shared imports and module coordination
- `world/creation.rs` is now thin as well: body lifecycle lives in `world/creation/body_lifecycle.rs`, world-space joint-base helpers in `world/creation/joint_builders.rs`, and shape/chain creation plus destruction in `world/creation/shape_creation.rs`, leaving the root focused on shared imports and module coordination
- `shapes/geometry.rs` is now thinner as well: the per-type geometry implementations live in `shapes/geometry/{circle,segment,chain_segment,capsule,polygon}.rs`, leaving the root focused on shared validation/hull helpers, type definitions, and free constructors
- `world/body_api.rs` is now thinner as well: pure body reads/enumeration live in `world/body_api/reads.rs` and mutable state/control helpers live in `world/body_api/control.rs`, leaving the root focused on shared imports and module coordination
- the world-owned creation/lifecycle layer now lives in `world/creation.rs`, so `world.rs` no longer mixes body/shape/chain creation, destroy helpers, and world-space joint-base builders with the remaining id-scoped runtime accessors
- the body-by-id runtime getter/mutation surface now lives in `world/body_api.rs`, so `world.rs` no longer mixes those helpers with the remaining world coordination code
- the scoped id-borrow helper surface now lives in `world/borrow.rs`, so callback-sensitive handle borrowing no longer shares the same file as unrelated runtime/query code
- the shape-by-id runtime helper surface now lives in `world/shape_api.rs`, so `world.rs` no longer mixes those helpers with unrelated world coordination code
- the inline world regression tests now live in `world/tests.rs`, so `world.rs` acts as a thin coordination root instead of another mixed implementation-plus-tests sink
- the next world backlog is now explicit: do not re-open the thin root; split the largest child modules next if the maintenance payoff is still there

## M4: Advanced Wrapper Coverage

Status: shipped

Scope:

- typed friction callback API
- typed restitution callback API
- standalone `collision` module for shape proxies, GJK distance, shape cast, and TOI
- `Aabb::is_valid()` and `Aabb::ray_cast(origin, translation)`
- recoverable validation for standalone collision inputs and `try_*` entrypoints for segment-distance / distance / shape-cast / TOI / manifold helpers
- geometry value validation on `Circle` / `Segment` / `ChainSegment` / `Capsule` / `Polygon`, wired through shape create/edit entrypoints
- helper-specific validation plus recoverable `try_*` entrypoints for world-free geometry helpers such as mass/AABB/point/ray/transform operations on crate-owned geometry values

Exit criteria:

- advanced collision customization and low-level geometry algorithms no longer require raw `ffi` for normal use
- standalone collision helpers expose both panic-by-default and recoverable validation paths instead of relying on upstream asserts for malformed input
- world-free geometry helpers validate the inputs their Box2D helper actually requires, while preserving upstream-defined degenerate segment/capsule behavior instead of over-tightening everything to shape-construction validity
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
