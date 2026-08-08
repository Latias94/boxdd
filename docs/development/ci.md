# CI And Release Checks

CI keeps the Safe Rust API aligned with the vendored Box2D source while avoiding a second compiler
or a second copy of the workflow policy in `xtask`.

The verification boundary is deliberately split:

- rustc and Clippy own names, types, receivers, traits, features, and target compilation;
- trybuild owns ownership and lifetime compile-fail contracts;
- nextest owns Rust behavior;
- the C ABI probe owns cross-language layout, symbol, precision, and callback compatibility;
- Miri, mixed Rust/C ASan and TSan, and C UBSan driven through Rust own unsafe-memory checks;
- `xtask` owns deterministic generated files, provider runtime smoke tests, package contents, and
  release artifact integrity.

## Contributor Checks

For ordinary changes, run:

```bash
cargo fmt --all -- --check
cargo clippy --locked -p boxdd-sys -p boxdd -p bevy_boxdd --all-targets -- -D warnings
cargo clippy --locked -p boxdd-sys --all-targets --features "double-precision validate disable-simd" -- -D warnings
cargo clippy --locked -p boxdd --all-targets --features "double-precision serde mint nalgebra glam bytemuck validate disable-simd" -- -D warnings
cargo clippy --locked -p bevy_boxdd --all-targets --features double-precision -- -D warnings
cargo clippy --locked -p xtask --all-targets -- -D warnings
cargo nextest run --locked --workspace
cargo nextest run --locked -p boxdd -p boxdd-sys --features boxdd/double-precision
cargo nextest run --locked -p bevy_boxdd --features double-precision
cargo test --locked -p boxdd --doc
cargo test --locked -p bevy_boxdd --doc
cargo run --locked -p xtask -- upstream-sync --check
cargo run --locked -p xtask -- api-inventory --check
cargo run --locked -p xtask -- recording-wire-codegen --check
```

Run the specialized commands only when their boundary changes:

```bash
cargo run --locked -p boxdd-abi-probe --bin abi-ctest --no-default-features
cargo test --locked -p boxdd-abi-probe --test abi --no-default-features
cargo run --locked -p boxdd-abi-probe --bin abi-ctest --no-default-features --features double-precision
cargo test --locked -p boxdd-abi-probe --test abi --no-default-features --features double-precision
cargo +1.95.0 check --locked --workspace --all-targets
cargo +1.95.0 check --locked -p bevy_boxdd --all-targets --features double-precision
cargo +1.95.0 check --locked -p boxdd --all-targets --features serde
cargo +1.95.0 check --locked -p boxdd --all-targets --features mint
cargo +1.95.0 check --locked -p boxdd --all-targets --features nalgebra
cargo +1.95.0 check --locked -p boxdd --all-targets --features glam
cargo +1.95.0 check --locked -p boxdd --all-targets --features bytemuck
cargo check --locked -p boxdd --all-targets --features simd-avx2
cargo check --locked -p boxdd --all-targets --features "double-precision simd-avx2"
cargo check --locked -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed
cargo check --locked -p boxdd --example testbed_imgui_glow --features "imgui-glow-testbed double-precision"
cargo run --locked -p xtask -- verify-wasm --compile-only
cargo run --locked -p xtask -- wasm-provider-contract --check
cargo run --locked -p xtask -- verify-wasm --runtime
cargo run --locked -p xtask -- verify-packages
cargo audit --deny unsound --file Cargo.lock
cargo semver-checks check-release --manifest-path boxdd-sys/Cargo.toml
cargo semver-checks check-release --manifest-path boxdd/Cargo.toml
cargo semver-checks check-release --manifest-path bevy_boxdd/Cargo.toml
cargo +nightly-2026-05-27 run --locked -p xtask -- verify-miri
cargo +nightly-2026-05-27 run --locked -p xtask -- verify-sanitizers --address
cargo +nightly-2026-05-27 run --locked -p xtask -- verify-sanitizers --undefined
cargo +nightly-2026-05-27 run --locked -p xtask -- verify-sanitizers --thread
```

Use nextest for ordinary Rust tests. The core doctest command validates public `boxdd` examples,
while the Bevy doctest command compiles every Rust block in the 0.6 migration guide. The
compiler-backed ABI binary uses `cargo run` because `ctest` provides its own harness; callback and
precision fixtures remain normal Rust tests. Run the AVX2 compile contracts on native x86_64 Linux
or Windows hosts.

## CI Jobs

Each job covers a distinct axis:

- `compiler-baseline`: workspace and optional-feature compilation on Rust 1.95;
- `lint`: formatting, Clippy, generated-file checks, Linux tests, docs, ABI probes, and native
  single/double-precision AVX2 compilation;
- `build`: native tests and ABI probes on macOS and Windows, plus native single/double-precision
  AVX2 compilation on Windows;
- `system-provider`: fresh caller-attested system artifacts in both precisions on MSRV and current
  Rust;
- `wasm`: compile-only targets, provider-routed single/double Rust cfg paths on MSRV/current Rust,
  and callback availability boundaries;
- `provider-runtime`: real single- and double-precision Node and Chromium execution;
- `security`: dependency audit, packaged-crate consumers, positive 0.6 SemVer checks, and a
  forced-patch negative witness for the intentional break;
- `miri` and `sanitizers`: focused unsafe-boundary suites. Rust and C are instrumented for ASan and
  TSan; UBSan instruments the C side because rustc has no UBSan mode, while the Rust harness drives
  the same FFI paths with ordinary overflow, alignment, and debug assertions enabled.

Default workspace tests run once on Linux. Additional runs must exercise a different precision,
feature-dependent test, target, provider, compiler, or operating system.

## WASM Prerequisites

Runtime and Pages checks require Node 22.16.0 and an activated Emscripten 6.0.4 installation from
the repository-pinned Emsdk commit. `boxdd-sys` never downloads an SDK. See
[WebAssembly Support](../platforms/wasm.md) for setup and runtime limits.

`build-pages-wasm` uses the locked `wasm-bindgen-cli-support` crate and the activated Binaryen
tool. Browser checks require the repository's npm lockfile and Playwright Chromium. They exercise
only the runtime generated from the current checkout; Pages does not preserve historical loader
or asset cohorts.

## Release Boundary

Release preparation moves the accumulated entries from `Unreleased` into a versioned Changelog
section dated with the actual release date before creating the protected tag. Development commits
must not describe an unreleased version as already released.

`verify-packages` packages the three publishable crates and runs fresh single- and double-precision
native consumers, plus compile-only WASM consumers. Native and WASM provider workflows additionally
execute their real provider contracts.

The protected release workflow runs `release-contract --check --artifacts <directory>` after all
archives, manifests, provenance statements, signatures, and checksums have been collected. The
reusable exact-commit qualification also builds and validates Pages assets and runs their Chromium
smoke test before producing its qualification receipt. The release contract
command rejects calls without an artifact directory; it does not provide a repository-only green
check. It validates the release version, tag and commit identity, clean vendored source, exact
archive contents, checksums, provider manifests, provenance statements, signatures, and trusted
root. It does not parse GitHub Actions YAML or claim that local execution proves an OIDC identity.
Action syntax and policy are reviewed in the workflow itself and checked with standard workflow
tooling.

The `contents: write` publication job does not check out the repository or execute Cargo, npm, or
other repository code. It downloads the exact immutable signed aggregate and its canonical
publication receipt by artifact ID, verifies their recorded digests and complete 49-file inventory,
and uploads only the receipt-authorized bytes.

Repository commands assume the locally selected Git, Rust toolchain, C compiler, and optional SDK
are trusted developer tools. They isolate Git configuration, hooks, and process-injection
environment variables, but do not implement a second executable resolver or operating-system trust
store inside `xtask`.

The release workflow calls the same CI workflow at the tagged commit and publishes only after every
required job succeeds for that exact SHA.
