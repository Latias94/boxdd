# Allocation Hotpaths TODO

## Done

- [x] Define the first milestone around hot-path query APIs only.
- [x] Add `*_into` / `try_*_into` query APIs on `World`.
- [x] Add matching `*_into` / `try_*_into` query APIs on `WorldHandle`.
- [x] Refactor shared callback collection internals in `query.rs`.
- [x] Replace temporary proxy point heap allocation with `SmallVec`.
- [x] Add `*_into` / `try_*_into` APIs for contact data, sensor overlaps, world sensor overlap reads, and chain segments.
- [x] Add a shared FFI `Vec` fill helper for non-callback extraction APIs.
- [x] Make sensor valid-filter paths reuse a single buffer instead of allocating twice.
- [x] Add tests for buffer reuse semantics and callback safety.
- [x] Update examples, crate docs, version, and changelog.

## Next

- [ ] Review debug draw command collection for reusable-buffer support.
- [ ] Review serialization helpers for shared raw-buffer fill utilities.
- [ ] Revisit `World` / `WorldHandle` query method duplication and decide whether a macro- or trait-based consolidation is worth the maintenance trade-off.
- [ ] Audit remaining owned/scoped handle duplicate implementations outside the allocation-hotpath surface.

## Release Checklist

- [x] Run the targeted test suite.
- [x] Run a full crate test sweep before tagging.
- [ ] Confirm release notes wording in `CHANGELOG.md`.
- [ ] Publish `boxdd 0.3.0`.
