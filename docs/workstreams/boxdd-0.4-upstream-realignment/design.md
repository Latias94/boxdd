# boxdd 0.4 Upstream Realignment

`boxdd 0.3.0` shipped with a temporary local Box2D patch that normalized open-chain
runtime material access to visible live segments. The published crates remain usable, but
repository checkouts and CI became fragile because the submodule pointer referenced a
local-only Box2D commit.

`0.4.0` is the cleanup release for that mistake:

- keep `0.3.0` published instead of yanking it
- move back to the official upstream Box2D submodule history
- remove the boxdd-specific runtime chain-material symbols from `boxdd-sys`
- preserve the `boxdd` safe API semantics by implementing visible-segment normalization in Rust

The key design choice is that `boxdd` keeps the user-facing runtime chain material
vocabulary (`surface_material_count`, `surface_material`, `set_surface_material`) while
`boxdd-sys` stops pretending the local patch is part of the upstream FFI surface.

This makes the maintenance model honest again:

- `boxdd-sys` tracks official vendored Box2D sources
- `boxdd` owns wrapper-level semantic normalization in Rust when upstream runtime helpers are inconsistent

