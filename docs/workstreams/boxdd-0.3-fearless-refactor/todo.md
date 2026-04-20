# boxdd 0.3 Fearless Refactor TODO

## Done

- [x] Ship reusable-buffer `*_into` / `try_*_into` query APIs for high-frequency world queries.
- [x] Ship reusable-buffer extraction APIs for contact data, sensor overlaps, world sensor overlap reads, and chain segments.
- [x] Add shared callback collection and FFI `Vec` fill helpers to remove repeated low-level plumbing.
- [x] Update docs, examples, versioning, tests, and changelog for the first `0.3.0` slice.
- [x] Record the broader `0.3.0` refactor scope in a dedicated workstream.

## In Progress

- [x] Add safe character mover plane collection APIs on `World` and `WorldHandle`.
- [x] Add safe mover-related value types:
  `Plane`, `MoverPlaneResult`, `CollisionPlane`, and `PlaneSolverResult`.
- [x] Add safe `solve_planes` and `clip_vector` helpers without requiring direct `ffi` usage.
- [x] Update the mover example to demonstrate the full safe workflow instead of only `cast_mover`.
- [x] Add mover-focused tests, including reusable-buffer behavior for plane collection.
- [x] Update `CHANGELOG.md` to reflect the expanded `0.3.0` scope.

## Next

- [ ] Audit whether debug draw command collection should gain reusable-buffer APIs.
- [ ] Review remaining `World` / `WorldHandle` query duplication and decide whether selective consolidation is worth it.
- [ ] Audit remaining owned/scoped handle duplication outside the already-refactored hot paths.
- [ ] Design typed safe friction / restitution callback APIs comparable in quality to custom filter and pre-solve.
- [ ] Identify geometry / collision helpers that still force users into raw `boxdd_sys::ffi`.

## Release Checklist

- [x] Run `cargo fmt --all`.
- [x] Run targeted mover tests.
- [x] Run `cargo nextest run -p boxdd`.
- [x] Review `README.md` and examples for `0.3.0` API consistency.
- [x] Finalize `CHANGELOG.md` wording for the full `0.3.0` release.
- [ ] Publish `boxdd 0.3.0`.
