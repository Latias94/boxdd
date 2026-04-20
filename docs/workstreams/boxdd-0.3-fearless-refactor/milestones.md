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

Status: planned

Scope:

- review remaining allocation-sensitive APIs
- review `World` / `WorldHandle` duplication
- review owned/scoped handle duplication outside the hottest paths

Exit criteria:

- the remaining duplication backlog is explicitly categorized as worth keeping or worth removing
- no obvious per-frame allocation trap remains undocumented

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

Exit criteria:

- users can move between shape construction and standalone collision algorithms without dropping to raw `ffi`
- the remaining raw geometry exposure is explicit, narrow, and justified

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

Status: planned

Scope:

- review remaining public raw escape hatches such as `world_id`, raw event slices, and debug draw hooks
- decide whether standalone manifold-generation helpers (`b2Collide*`) should become safe collision APIs

Exit criteria:

- the remaining raw public surface is either clearly intentional or scheduled for removal
- the next completeness pass has a short, explicit backlog instead of scattered notes
