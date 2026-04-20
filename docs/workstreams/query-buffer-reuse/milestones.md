# Allocation Hotpaths Milestones

## M1: Query Core

Status: shipped in the workstream branch

Scope:

- additive `*_into` query APIs
- shared internal collection helpers
- stack-first proxy point buffering
- tests, examples, docs, version bump, changelog

Exit criteria:

- no breaking API removals
- `World` and `WorldHandle` expose the same reusable-buffer query surface
- docs clearly explain when to use owned-returning vs reusable-buffer APIs

## M2: Adjacent Hot Paths

Status: shipped in the workstream branch

Scope:

- sensor overlap APIs
- contact data collection APIs
- chain segment collection APIs
- shared FFI `Vec` fill helper for non-callback extraction code

Exit criteria:

- consistent `*_into` naming across adjacent high-frequency read APIs
- no unnecessary duplication between safe, try, and unchecked surfaces
- sensor valid-filter paths reuse a single allocation path

## M3: Allocation Policy Review

Status: planned

Scope:

- identify remaining per-frame allocation hotspots in the safe wrapper
- decide where borrowed views beat reusable `Vec` APIs
- document crate-wide guidance for allocation-sensitive APIs

Exit criteria:

- a documented rule of thumb for choosing between owned return values, `*_into`, and borrowed views
- a backlog of remaining refactors with release targeting
