# boxdd 0.3 Fearless Refactor

## Context

`boxdd 0.3.0` is intentionally larger than a narrow bug-fix release.

The first delivered slice already improved one of the crate's biggest practical gaps:
allocation-heavy hot-path APIs now expose reusable-buffer `*_into` variants.

That work solved a real gameplay problem reported by users, but it also highlighted a
broader issue: the crate is already a strong safe wrapper, yet some parts of the
official Box2D v3 surface are still exposed unevenly.

In particular:

- hot-path query APIs were previously ergonomic but allocation-hostile
- character mover / geometric controller APIs were only partially wrapped
- some callback and query surfaces are already strongly typed, while adjacent ones still
  require raw FFI knowledge
- `World` / `WorldHandle` and owned/scoped handle layers still contain repeated logic

`0.3.0` is the release where we deliberately clean this up instead of preserving every
legacy shape out of inertia.

## Problem Statement

The current crate is good enough for many projects, but it is not yet as complete or as
coherent as it should be for a flagship safe Box2D binding.

The main gaps are:

- gameplay hot paths still need auditing beyond the initial query buffer reuse work
- debug draw command collection still needs the same reusable-buffer treatment as the main query APIs
- the character mover flow is incomplete unless users drop to `boxdd_sys::ffi`
- some safe APIs still duplicate the same implementation patterns across handle styles
- the crate lacks a clearer release-level refactor plan that ties these efforts together
- some crate-owned value types still blur the raw FFI boundary with implicit conversions instead of explicit escape hatches
- `ContactId` values appear in event and snapshot APIs, but until this pass they still lacked a direct safe inspection path for validation or data reads
- the threading / async model is correct but still too easy to misread unless users inspect the source
- math interop coverage is useful but still uneven, especially around `mint` rotation / transform forms
- the panic-vs-`try_*` error-handling strategy is sound but not explicit enough at the crate boundary
- callback registration on `World` has historically been asymmetric: material-mixing callbacks gained recoverable `try_*` setup, while custom filter / pre-solve registration lagged behind on panic-only helpers
- owned world event snapshots have a safe zero-copy story via visitors, but until now the owned-copy path still forced fresh allocations unless users reworked their loop around borrowed views
- `WorldHandle` mirrors many read-only runtime helpers, and now also mirrors owned event snapshots; borrowed/raw event views are still different because they depend on step-local world buffers and deferred-destroy flush timing
- live chain material helpers used to leak Box2D's open-chain ghost-point placeholder layout instead of the runtime-visible live-segment vocabulary users actually manipulate
- the `shapes` module still carries too many responsibilities in one root file, making further correctness and completeness work harder to audit than the already-split `body`, `query`, and `joints` areas
- the first `shapes/mod.rs` split helped, but `shapes/runtime.rs` was still a second mixed-responsibility sink until the runtime helper, validation/creation, and user-data layers started moving into dedicated child modules
- `joints/base.rs` also remained a mixed-responsibility sink even after `JointBase` moved out, until the runtime-handle, user-data, owned, and scoped wrapper layers started moving into dedicated child modules
- the `world` module is still the largest mixed-responsibility hotspot in the crate, even after the world-config, lightweight stats, stored-handle, callback-heavy runtime, and creation/lifecycle layers are peeled off
- the follow-up world decomposition is now underway: `world/body_api.rs`, `world/borrow.rs`, `world/shape_api.rs`, and `world/tests.rs` have already peeled most mixed concerns out of the root, but the next follow-up should target the largest remaining world submodules instead of declaring victory too early
- `world/runtime.rs` was the first of those large child modules to show the same pattern, because read-only helper plumbing, step/tuning control, and callback registration/trampolines were still cohabiting one file until they started moving into dedicated child modules too
- `body/runtime.rs` shows the same follow-up pattern as well: contact/attachment collection, typed user-data plumbing, and the shared owned/scoped runtime-handle trait were still cohabiting one file until they started moving into dedicated child modules
- `world/handle.rs` followed the same pattern after the root split: callback-safe user-data reads, world-level diagnostics, body-by-id reads, and shape-by-id reads were still cohabiting one file until they started moving into dedicated child modules too
- `query/world_api.rs` followed the same pattern too: the explicit `World` and `WorldHandle` query surfaces intentionally stayed public and parallel, but they no longer needed to cohabit one file once the module root became a pure coordination layer
- `world/creation.rs` followed the same pattern too: body lifecycle, world-space joint-base helpers, and shape/chain creation helpers were still cohabiting one file even though they are maintained on different cadence and belong to different follow-up concerns
- `shapes/geometry.rs` followed the same pattern too: shared geometry validation/hull helpers, five crate-owned value types, and the free constructor helpers all lived in one file until the per-type implementations started moving into dedicated child modules
- `world/body_api.rs` followed the same pattern too: pure body reads/enumeration and mutable state/control helpers lived in one file until they started moving into dedicated child modules

If we do not address these now, the likely outcome is a sequence of small additive
patches that preserve avoidable duplication and keep advanced users half inside the safe
API and half inside raw FFI.

## Goals

- Make `0.3.0` a coherent safe-wrapper upgrade, not only an allocation patch.
- Productize the Box2D character mover flow as an ergonomic safe API.
- Keep hot-path APIs friendly to per-frame reuse patterns.
- Identify and remove redundant implementation patterns where the maintenance cost is not justified.
- Make raw FFI escape hatches explicit where the crate owns the safe vocabulary.
- Track larger follow-up refactors explicitly so the crate does not drift back toward ad-hoc growth.

## Non-Goals

- Preserve every existing API shape if a clearer breaking refactor is materially better.
- Wrap every last upstream helper in `0.3.0`.
- Introduce abstraction layers that reduce clarity just to avoid some local duplication.

## Design Principles

### 1. Safe First, Not Thin First

The crate should expose upstream concepts faithfully, but not at the expense of ergonomic
or allocation-hostile APIs. A good safe wrapper should encode the common usage pattern,
not merely rename C functions.

This also means hiding upstream storage quirks when they are not part of the runtime concept
users actually manipulate. Open-chain ghost-point material placeholders are a creation-time
detail, so runtime chain material APIs should speak in live-segment indexing instead.

The same principle applies to obvious parameter preconditions. If Box2D expects ordered
ranges or non-negative finite material scalars, the safe wrapper should reject those
arguments explicitly instead of making behavior depend on upstream assert builds.
The same policy also applies to creation-time definition objects and shared defaults:
safe `*Def` wrappers should preflight obvious native invariants, and crate-owned
default constructors must mirror upstream defaults instead of approximating them.

That policy is about the helper's real contract, not about blindly reusing the most
restrictive validation everywhere. Some standalone geometry helpers legitimately support
degenerate values even though those same values are not valid shape-construction input.
For example, zero-length segments can still produce defined miss/AABB behavior, and
zero-length capsules still have circle-like mass/point/ray semantics upstream. The safe
wrapper should therefore validate each helper against the preconditions Box2D actually
requires, then expose recoverable `try_*` variants for the same contract.

### 2. Hot Paths Must Be Reusable

If an API is plausibly called every frame, the safe surface should not force fresh heap
allocation. Convenience methods can remain, but reusable-buffer variants should exist when
the call pattern justifies them.

### 3. Complete Flows Beat Isolated Entry Points

Wrapping `b2World_CastMover` alone is not enough. The useful product surface is the
character-mover flow:

1. cast the mover
2. collect contact planes
3. solve planes
4. clip velocity

Users should not need raw FFI to finish the workflow.

### 4. Duplication Must Earn Its Keep

Some duplication is acceptable for API clarity. Repeated callback plumbing, raw `Vec`
fill patterns, or near-identical handle implementations should be consolidated when the
result is simpler and easier to audit.

For owned/scoped handle pairs, prefer small private free-function helpers over macro
layers or trait indirection. The goal is to keep the public API explicit while making
the shared FFI path single-sourced internally.

### 5. Raw Escape Hatches Should Be Loud

If `boxdd` owns a user-facing value type, crossing into raw Box2D structs should be
explicit in the API surface. Implicit `From<ffi::...>` conversions are convenient for
internal plumbing, but they make the public safe vocabulary too porous and hide where
FFI boundaries actually exist.

### 6. Intentional Raw Seams Must Stay Narrow

`0.3.0` is not trying to delete every low-level interop hook. The point is to make the
kept seams explicit, justified, and regression-tested.

The main intentional raw escape hatches are:

- raw ids and raw world-id accessors such as `world_id_raw` for integration with ID-style
  storage or external systems already built around Box2D ids
- explicit raw conversion points on crate-owned value types via `from_raw(...)` /
  `into_raw()`
- raw event-slice visitors `unsafe { with_*_events_raw(...) }` for zero-copy advanced
  consumers that need direct access to Box2D event buffers
- `debug_draw_raw` for render backends that want zero-copy vertex slices and `CStr`
  strings instead of the safe converted callback surface
- raw user-data pointer APIs (`set_user_data_ptr_raw`, `user_data_ptr_raw`) for interop with
  existing pointer-based ownership schemes
- `unsafe from_raw(...)` on pointer/callback-bearing config wrappers such as `BodyDef` and
  `WorldDef`, because those raw structs can carry borrowed strings or executable callbacks

These seams are worth keeping only if:

- the safe path already exists for normal use
- the raw path is clearly named or `unsafe`
- callback locking / deferred-destroy / panic forwarding behavior stays aligned with the
  safe path
- regression tests cover the callback-sensitive raw paths

### 7. Completeness Must Be Auditable

`0.3.0` is large enough that "we think coverage is pretty good" is not a sufficient release
statement anymore.

The workstream now keeps a dedicated completeness matrix at
`docs/workstreams/boxdd-0.3-fearless-refactor/completeness-matrix.md` and classifies each
major area as:

- `safe-covered`
- `raw-only`
- `intentional omission`
- `candidate after 0.3`

This keeps the remaining gaps explicit and stops real omissions from hiding next to
deliberate exclusions.

## Release Scope

### Delivered in the first `0.3.0` slice

- reusable-buffer query APIs
- reusable-buffer contact / sensor / chain extraction APIs
- reusable-buffer debug draw command collection
- shared FFI `Vec` fill helpers
- hot-path docs, tests, examples, version bump, changelog updates

### Targeted next in this workstream

- character mover collision-plane collection APIs
- safe plane / collision-plane / solver result types
- `solve_planes` and `clip_vector` safe entry points
- docs and examples that show the intended mover flow
- typed friction / restitution callbacks
- standalone collision geometry helpers for distance, shape cast, TOI, and AABB validation/ray cast
- crate-owned wrapper cleanup for remaining leaked Box2D value types (`ShapeType`, `MassData`, contact data, and manifolds)
- explicit raw geometry conversions for crate-owned shape geometry values
- live shape runtime completeness cleanup so AABB / point test / ray cast / mass data / event toggles stay aligned across owned/scoped/id styles
- body runtime completeness cleanup so rotation / sleeping / awake-enabled-bullet-name state / attached ids / computed body AABB / fast-rotation setup stay aligned across owned/scoped/id styles
- joint runtime completeness cleanup so common joint metadata plus type-specific distance/prismatic/revolute/weld/wheel/motor state/control stay aligned across owned/scoped/id styles, and wrong-family `try_*` calls return `ApiError::InvalidJointType`
- world runtime extras cleanup so diagnostics/tuning helpers like `Profile`, explosions, speculative collision, and callback-sensitive tuning toggles plus callback-registration helpers live on the same main safe surface with matching `try_*` coverage and mirrored read-only access on `WorldHandle`
- safe `ContactId` helpers so contact ids coming from events or snapshots can be validated and resolved into crate-owned/raw contact data without dropping to `boxdd_sys::ffi`
- math-interop completeness cleanup so `mint` stays a first-class bridge instead of a partially-covered feature, including recoverable inbound conversion for crate-owned rotation values
- explicit threading / async documentation and examples that preserve the current `!Send` / `!Sync` design instead of weakening it
- clearer crate-level error-handling guidance for panic-by-default vs `try_*` usage
- callback-sensitive world execution helpers such as stepping, deferred-destroy flushing, and debug draw should follow the same panic-by-default plus `try_*` split as the rest of the runtime surface
- borrowed event-buffer access should follow the same rule too, so zero-copy views and raw event slices are not left behind as panic-only APIs once the owned snapshot surface has `try_*` coverage
- reusable-buffer event snapshot APIs so callers that need owned event data can still avoid per-frame allocation churn without dropping to raw or borrowed-only views
- reusable-buffer wrapper metadata snapshots so crate-owned serialize registries such as `body_ids()` / `chain_records()` do not regress into allocation-only APIs beside the main query/event surface
- serialize-time chain metadata cleanup so `ChainDef` helpers and `World::chain_records()` stay on crate-owned `Filter` / `Vec2` / `SurfaceMaterial` vocabulary instead of leaking raw `ffi` collections back into the public surface
- definition value-object cleanup so `ShapeDef` / `ChainDef` can be inspected as normal crate-owned config values instead of acting like builder-only write shells
- split the oversized `shapes/mod.rs` root along value-object, creation, owned/scoped wrapper, and internal runtime-helper boundaries so shape follow-up work no longer lands in one giant file
- keep decomposing `shapes/runtime.rs` after that first root split too, so validation, body-attached creation plumbing, and user-data/checked helper layers stop cohabiting with the remaining runtime handle core
- creation-definition cleanup so `BodyDef`, `JointBase`, and concrete joint defs no longer act like write-only shells, and obvious naming mistakes on config-only APIs are corrected even when that requires a breaking change
- world-config cleanup so top-level setup values such as `WorldDef` and `ExplosionDef` follow the same readable crate-owned value-object rules as the rest of the safe API
- continue decomposing the oversized `world.rs` root after the config/stats split so stored read-only `WorldHandle` queries and the mutable `World` runtime surface stop cohabiting one giant file
- keep pushing that `world.rs` decomposition after the handle, runtime, and creation splits too, so the remaining mutable `World` body can later separate the last id-scoped runtime/event concerns without reintroducing another mixed-responsibility root
- after extracting `world/body_api.rs`, `world/borrow.rs`, `world/shape_api.rs`, and `world/tests.rs`, treat `world.rs` as a coordination root and keep future world follow-up work focused on the largest child modules rather than letting the root regrow
- keep decomposing `joints/base.rs` after the `JointBase` split too, so runtime-handle helpers, joint user-data plumbing, and owned/scoped wrapper bodies stop cohabiting in one follow-up sink
- keep decomposing the large world child modules too, starting with `world/runtime.rs`, so read-only helper plumbing, step/tuning control, and callback registration stop cohabiting in one follow-up sink even after the root itself is slimmed down
- keep decomposing `body/runtime.rs` after the main `body.rs` split too, so attachment/contact helpers, typed user-data plumbing, and the shared runtime-handle trait stop cohabiting in one follow-up sink
- keep decomposing `world/handle.rs` after the main `world.rs` split too, so callback-safe user-data reads and the world/body/shape stored-query slices stop cohabiting in one follow-up sink
- keep decomposing `query/world_api.rs` after the main `query.rs` split too, so the mirrored `World` and `WorldHandle` query entrypoints stay explicit without rebuilding another oversized two-receiver file
- keep decomposing `world/creation.rs` after the main `world.rs` split too, so body lifecycle, world-space joint-base builders, and shape/chain creation helpers stop cohabiting in one follow-up sink
- keep decomposing `shapes/geometry.rs` after the earlier shape-module split too, so per-type geometry behavior can move independently without rebuilding another oversized value-types-plus-helpers sink
- keep decomposing `world/body_api.rs` after the main `world.rs` split too, so pure body reads/enumeration and mutable state/control helpers stop cohabiting in one follow-up sink
- config raw-boundary cleanup so builder-oriented wrappers such as `BodyDef`, `ShapeDef`, `JointBase`, and concrete joint defs cross back to raw Box2D structs through explicit named escape hatches when users truly need that seam
- keep borrowed/raw event APIs centered on `World`; if `WorldHandle` mirrors event reads, keep that mirror limited to owned snapshots only

### Planned follow-up audit items

- unify standalone collision geometry helpers with shape-construction helper types
- continue the `World` / `WorldHandle` duplication review after the query surface consolidation, keeping only the duplication that is still clearer than a shared internal definition
- owned / scoped handle duplication review outside the hottest paths
- continue collapsing purely mechanical per-type API families such as joint creation entrypoints when the public surface stays identical but the internal drift risk drops
- continue the completeness audit after shipping the live-shape runtime wrappers, especially for any remaining body/joint/world-handle runtime gaps
- keep `shapes/mod.rs` as a thin public root after the split, and route future shape internals into dedicated submodules instead of rebuilding another giant mixed-responsibility file
- after introducing `shapes/runtime/validation.rs`, `shapes/runtime/creation.rs`, and `shapes/runtime/user_data.rs`, keep future shape follow-up work landing in dedicated runtime child modules instead of rebuilding another oversized `shapes/runtime.rs`
- after introducing `body/runtime/attachments.rs`, `body/runtime/user_data.rs`, and `body/runtime/handle.rs`, keep future body follow-up work landing in those child modules instead of rebuilding another oversized `body/runtime.rs`
- keep `joints/base.rs` as a thin coordination root after the handle split too; future joint-handle follow-up work should land in child modules instead of rebuilding another mixed-responsibility file
- keep `world.rs` moving toward a thin coordination root too: future world follow-up work should land in dedicated submodules instead of rebuilding another giant mixed-responsibility file around `World` / `WorldHandle`
- after introducing `world/runtime/reads.rs`, `world/runtime/control.rs`, and `world/runtime/callbacks.rs`, keep future runtime follow-up work landing in those child modules instead of rebuilding another oversized `world/runtime.rs`
- after introducing `world/handle/{callback_world,world_reads,body_reads,shape_reads}.rs`, keep future stored-query follow-up work landing in those child modules instead of rebuilding another oversized `world/handle.rs`
- after introducing `query/world_api/{world_queries,handle_queries}.rs`, keep future query-entrypoint follow-up work landing in those child modules instead of rebuilding another oversized `query/world_api.rs`
- after introducing `world/creation/{body_lifecycle,joint_builders,shape_creation}.rs`, keep future creation/lifecycle follow-up work landing in those child modules instead of rebuilding another oversized `world/creation.rs`
- after introducing `shapes/geometry/{circle,segment,chain_segment,capsule,polygon}.rs`, keep future geometry follow-up work landing in those child modules instead of rebuilding another oversized `shapes/geometry.rs`
- after introducing `world/body_api/{reads,control}.rs`, keep future body-id runtime follow-up work landing in those child modules instead of rebuilding another oversized `world/body_api.rs`
- after `world/definition.rs`, `world/metrics.rs`, `world/handle.rs`, `world/runtime.rs`, `world/creation.rs`, `world/body_api.rs`, `world/borrow.rs`, `world/shape_api.rs`, and `world/tests.rs`, the next world decomposition pass should focus on splitting the largest runtime/query child modules instead of reopening the thin root
- keep world-space joint builders behaviorally coherent when runtime-computed frames or body ids are filled, so base flags such as `collide_connected` are not silently lost
- keep callback-sensitive event-buffer borrowing on a single internal path so deferred-destroy behavior cannot diverge across body/contact/sensor/joint views
- keep callback-registration plumbing on a single internal path so panic-by-default and recoverable `try_*` callback setup stay behaviorally aligned
- apply the same single-source rule to geometry-specific world helpers when circle/segment/capsule/polygon entrypoints are mechanically identical apart from the Box2D function they call
- keep intentional raw seams such as `debug_draw_raw` only when they share the same panic forwarding, callback lock semantics, and regression coverage as the safe path
- continue the same private-helper consolidation on joint handles when owned/scoped variants still repeat the same state, threshold, and user-data FFI plumbing
- audit the remaining intentional raw boundaries such as debug draw/raw color hooks and raw event/debug escape hatches, and confirm each one is still worth keeping
- continue value-type cleanup for remaining raw Box2D structs that still leak through public APIs
- continue auditing intentional raw seams such as debug draw/raw color paths and raw event/debug hooks so the kept escape hatches stay explicit and justified
- keep serialize-time chain capture on crate-owned value/layout types so scene/snapshot helpers do not re-leak raw Box2D point/material collections through convenience records
- keep `WorldHandle` event mirroring intentionally narrow: do not add borrowed/raw event-buffer APIs there unless a concrete use-case proves the added surface is worth the lifecycle complexity

## Current Intentional Omissions

After the latest world-level completeness pass, the remaining upstream `b2World_*`
functions not wrapped on the main safe surface are intentionally low priority:

- `b2World_DumpMemoryStats`, which writes a fixed debug file (`box2d_memory.txt`) and
  does not fit the normal ergonomic/runtime API story
- `b2World_RebuildStaticTree`, which upstream explicitly labels as internal testing

These do not block `0.3.0` completeness for the main safe wrapper. If a real production
use-case appears later, they can be revisited deliberately instead of being added just
to chase one-to-one API parity.

The crate still intentionally omits broader `WorldHandle` event mirroring beyond owned snapshots.
Owned snapshots now exist on both `World` and `WorldHandle`, but borrowed/raw event APIs stay on
`World` because they are bound to:

- the completed step's world-owned event buffers
- deferred-destroy flushing that happens around borrowed event-buffer access
- a shorter, more stateful lifecycle than the rest of `WorldHandle`'s cheap stored-query role

The preferred `WorldHandle` event boundary is therefore now explicit:

- allowed: owned snapshot helpers (`*_events` / `*_events_into` / `try_*`)
- not added in `0.3.0`: borrowed/raw event views

`ContactId`, on the other hand, is no longer an intentional omission for `0.3.0`.
Upstream only exposes validity and data-fetch helpers for contacts, so the crate now treats
that tiny surface as part of the normal safe runtime API directly on `ContactId` itself instead
of hiding it behind a separate extension trait or a mostly-empty contact handle wrapper.

## Release Strategy

`0.3.0` may include breaking API cleanup where it materially improves the crate.

The bar is:

- the new surface is more coherent than the old one
- the change removes legacy shape or duplication instead of adding another layer beside it
- the result is easier to explain, test, and maintain

This workstream is the umbrella record for that effort.
