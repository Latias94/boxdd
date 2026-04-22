# boxdd 0.4 Upstream Realignment Milestones

## M1: Upstream Source Recovery

Status: shipped

Scope:

- restore the Box2D submodule to the official upstream history
- remove repository dependence on a local-only Box2D commit
- delete the temporary runtime chain material FFI additions from `boxdd-sys`

Exit criteria:

- fresh repository checkouts can initialize submodules from the public upstream remote
- `boxdd-sys` no longer exposes custom FFI that is unavailable in the vendored upstream source

## M2: Safe API Preservation

Status: shipped

Scope:

- keep `Chain` runtime material APIs stable on the safe wrapper surface
- preserve visible-segment semantics for open chains in Rust
- avoid forcing downstream users back onto raw ghost-placeholder indexing

Exit criteria:

- `Chain::{surface_material_count,surface_material,set_surface_material}` still speak in visible live-segment indexing
- open-chain runtime material reads/writes no longer depend on custom C helpers

## M3: Release Hardening

Status: in progress

Scope:

- version bump to `0.4.0`
- changelog/workstream updates for the non-yanked `0.3.0` follow-up path
- targeted verification after the submodule realignment

Exit criteria:

- manifests, lockfile, and changelog all agree on the `0.4.0` follow-up
- regression checks pass on the restored official Box2D source layout
