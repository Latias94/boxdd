# boxdd 0.4 Upstream Realignment TODO

## Done

- [x] Audit every `boxdd` and `boxdd-sys` call site that depended on the local runtime chain material patch.
- [x] Confirm that `0.3.0` remains usable from crates.io and that the repository/CI failure comes from the submodule pointer, not from the published crate contents.
- [x] Move open-chain runtime material normalization into Rust so the safe `Chain` APIs keep visible-segment semantics without custom Box2D symbols.
- [x] Restore the Box2D submodule pointer to the official upstream commit used before the local patch.
- [x] Remove the boxdd-specific `b2Chain_*RuntimeSurfaceMaterial*` declarations from `boxdd-sys`.
- [x] Bump crate versions to the `0.4.0` line and record the release rationale in the changelog.

## Next

- [ ] Run the targeted regression checks for chain runtime materials, unchecked APIs, and general workspace compilation.
- [ ] Run the relevant CI/build matrix locally as far as practical after the submodule realignment.
- [ ] Prepare the final `0.4.0` release notes once the code and CI are green.

