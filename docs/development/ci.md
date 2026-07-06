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
cargo run -p xtask -- generate-pages
cargo run -p xtask -- validate-pages
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126 --locked
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- build-pages-wasm
# Optional for local builds: expose Binaryen's wasm-opt on PATH or through EMSDK/upstream/bin for smaller Pages assets.
cargo nextest run -p boxdd --test api_coverage --test collision_validation --test joint_new_apis --test world_callbacks --test panic_across_ffi_is_caught --test world_and_queries --test dynamic_tree --test events_and_sensors --test world_destroy_and_recycle --test material_mix_callbacks --test user_data --test ffi_lifecycle --test buffer_reuse
cargo nextest run -p boxdd-sys --test layout
cargo nextest run -p bevy_boxdd --test plugin
cargo clippy -p boxdd --all-targets --all-features -- -D warnings
cargo clippy -p bevy_boxdd --all-targets --no-default-features -- -D warnings
$env:RUSTDOCFLAGS='-D warnings --cfg docsrs'; cargo doc --workspace --no-deps
cargo package -p boxdd-sys --allow-dirty --no-verify
cargo package -p boxdd --allow-dirty --no-verify
cargo package -p bevy_boxdd --allow-dirty --no-verify
```

Use `cargo test` only as a fallback when nextest is unavailable.

For a fresh version line, run packaging and publishing in dependency order. `boxdd-sys` packages first; `boxdd` package verification can resolve only after `boxdd-sys <version>` is available from crates.io; `bevy_boxdd` package verification can resolve only after `boxdd <version>` is available from crates.io.

## Gate rationale

- `api-coverage --check` scans vendored `include/box2d` headers for `B2_API` symbols and ensures every public C API has an explicit Rust status.
- `sample-parity --check` scans upstream sample registrations, preserves manual mappings, and rejects non-benchmark rows that fall back to bare upstream references without an explicit deferral.
- `generate-pages` rebuilds the GitHub Pages Bevy Web example index from `bevy_boxdd/examples/testbed_2d/scenes.rs`.
- `provider-smoke` builds a Rust `wasm32-unknown-unknown` app, builds an Emscripten Box2D provider module, and verifies the shared-memory runtime under Node.
- `build-pages-wasm` rebuilds the example index, compiles the Bevy + egui testbed with the `wasm-release` profile, runs `wasm-bindgen`, builds the Emscripten Box2D provider, runs `wasm-opt -Oz` when available, and writes runtime assets in `docs/pages/wasm/generated` plus `docs/pages/bevy-testbed/generated`.
- `validate-pages` rejects stale generated Pages HTML, stale loader JavaScript, missing runtime assets, and broken local links.
- `boxdd-sys` layout tests protect representative ABI assumptions at the raw FFI boundary.
- `bevy_boxdd` plugin tests verify ECS creation, transform sync, distance/revolute joint lifecycle, contact/sensor messages, entity ray/AABB query mappings, debug draw collection, recoverable input errors, and public non-send boundaries without adding Bevy dependencies to the core crate.

## CI shape

CI should keep heavy checks staged:

- Fast lint: `cargo fmt`, clippy for `boxdd-sys`, `boxdd`, `bevy_boxdd`.
- Core tests: nextest or cargo test for the targeted API coverage, layout, dynamic tree, and Bevy plugin suites.
- Feature matrix: focused `cargo check` for `boxdd` optional math/serialization features.
- Pages runtime: install `wasm32-unknown-unknown`, `wasm-bindgen-cli 0.2.126`, and Emscripten SDK, expose `emsdk/upstream/bin` so `wasm-opt` can be found, then run `cargo run -p xtask -- build-pages-wasm` and `cargo run -p xtask -- validate-pages`.
- Docs: set `RUSTDOCFLAGS` to `-D warnings --cfg docsrs`, then run `cargo doc --workspace --no-deps`.
- Packaging: `cargo package -p boxdd-sys --allow-dirty --no-verify`, then `boxdd`, then `bevy_boxdd` as metadata smoke checks in publish order. For a new shared workspace version, dependent package checks are expected to wait until the previous crate in the chain is visible on crates.io. Run full package verification without `--no-verify` before publishing each crate.
