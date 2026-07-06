# CI and Release Gates

This workspace uses CI to keep the Rust safe layer aligned with the vendored Box2D C API.

## Required local checks

Run these before opening a release PR:

```powershell
cargo metadata --format-version 1 --no-deps
cargo fmt --all -- --check
cargo check -p boxdd --no-default-features
cargo check -p boxdd-sys --no-default-features
cargo check -p bevy_boxdd --no-default-features
cargo check -p bevy_boxdd --examples
cargo run -p xtask -- api-coverage --check
cargo run -p xtask -- sample-parity --check
cargo run -p xtask -- validate-pages
cargo nextest run -p boxdd --test api_coverage --test collision_validation --test joint_new_apis --test world_callbacks --test panic_across_ffi_is_caught --test world_and_queries --test dynamic_tree --test events_and_sensors --test world_destroy_and_recycle --test material_mix_callbacks --test user_data --test buffer_reuse
cargo nextest run -p boxdd-sys --test layout
cargo nextest run -p bevy_boxdd --test plugin
cargo clippy -p boxdd --all-targets --all-features -- -D warnings
cargo clippy -p bevy_boxdd --all-targets --no-default-features -- -D warnings
$env:RUSTDOCFLAGS='-D warnings --cfg docsrs'; cargo doc --workspace --no-deps
cargo package -p boxdd --allow-dirty --no-verify
cargo package -p boxdd-sys --allow-dirty --no-verify
cargo package -p bevy_boxdd --allow-dirty --no-verify
```

Use `cargo test` only as a fallback when nextest is unavailable.

## Gate rationale

- `api-coverage --check` scans vendored `include/box2d` headers for `B2_API` symbols and ensures every public C API has an explicit Rust status.
- `sample-parity --check` scans upstream sample registrations, preserves manual mappings, and rejects non-benchmark rows that fall back to bare upstream references without an explicit deferral.
- `validate-pages` keeps the static GitHub Pages hub link-valid without requiring a browser runtime.
- `boxdd-sys` layout tests protect representative ABI assumptions at the raw FFI boundary.
- `bevy_boxdd` plugin tests verify ECS creation, transform sync, distance/revolute joint lifecycle, contact/sensor messages, entity query mappings, debug draw collection, and recoverable input errors without adding Bevy dependencies to the core crate.

## CI shape

CI should keep heavy checks staged:

- Fast lint: `cargo fmt`, clippy for `boxdd-sys`, `boxdd`, `bevy_boxdd`.
- Core tests: nextest or cargo test for the targeted API coverage, layout, dynamic tree, and Bevy plugin suites.
- Feature matrix: focused `cargo check` for `boxdd` optional math/serialization features.
- Docs: set `RUSTDOCFLAGS` to `-D warnings --cfg docsrs`, then run `cargo doc --workspace --no-deps`.
- Packaging: `cargo package -p boxdd --allow-dirty --no-verify`, `cargo package -p boxdd-sys --allow-dirty --no-verify`, and `cargo package -p bevy_boxdd --allow-dirty --no-verify` as metadata smoke checks. Run full package verification without `--no-verify` before publishing.
