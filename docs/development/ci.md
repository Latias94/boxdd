# CI and Release Gates

This workspace uses CI to keep the Rust safe layer aligned with the vendored Box2D C API.

## Required local checks

Run these before opening a release PR:

```bash
cargo metadata --no-deps
cargo fmt --all -- --check
cargo check -p boxdd --no-default-features
cargo check -p boxdd-sys --no-default-features
cargo check -p bevy_boxdd --no-default-features
cargo run -p xtask -- api-coverage --check
cargo run -p xtask -- sample-parity --check
cargo run -p xtask -- validate-pages
cargo test -p boxdd --test api_coverage
cargo test -p boxdd-sys --test layout
cargo test -p boxdd --test dynamic_tree
cargo test -p bevy_boxdd --test plugin
cargo run -p boxdd --example dynamic_tree
```

Use `cargo nextest run` for the test commands when nextest is available.

## Gate rationale

- `api-coverage --check` scans vendored `include/box2d` headers for `B2_API` symbols and ensures every public C API has an explicit Rust status.
- `sample-parity --check` scans upstream sample registrations and prevents the parity matrix from silently drifting.
- `validate-pages` keeps the static GitHub Pages hub link-valid without requiring a browser runtime.
- `boxdd-sys` layout tests protect representative ABI assumptions at the raw FFI boundary.
- `bevy_boxdd` plugin tests verify the ECS adapter without adding Bevy dependencies to the core crate.

## CI shape

CI should keep heavy checks staged:

- Fast lint: `cargo fmt`, clippy for `boxdd-sys`, `boxdd`, `bevy_boxdd`.
- Core tests: nextest or cargo test for the targeted API coverage, layout, dynamic tree, and Bevy plugin suites.
- Feature matrix: focused `cargo check` for `boxdd` optional math/serialization features.
- Docs: `RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo doc --workspace --no-deps`.
- Packaging: `cargo package -p boxdd --allow-dirty`, `cargo package -p boxdd-sys --allow-dirty`, and `cargo package -p bevy_boxdd --allow-dirty` in release branches.
