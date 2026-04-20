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

Status: in progress

Scope:

- typed friction callback API
- typed restitution callback API
- identify candidates for a broader safe geometry / collision surface

Exit criteria:

- advanced collision customization no longer feels noticeably less productized than filter / pre-solve
- the next post-`0.3` wrapper-coverage push has a concrete backlog instead of scattered notes
