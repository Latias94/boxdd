# CI and Release Gates

This workspace uses CI to keep the Rust safe layer aligned with the vendored Box2D C API.

See [Upstream Synchronization Contract](upstream-sync.md) for the revision transition and rollback model.

## Required local checks

Run these before opening a release PR:

```powershell
cargo metadata --format-version 1 --no-deps
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo check -p boxdd --no-default-features
cargo check -p boxdd-sys
cargo check -p bevy_boxdd --no-default-features
cargo check -p bevy_boxdd --examples
cargo run -p xtask -- verify-toolchains
cargo run -p xtask -- verify-precision-contract
cargo run -p xtask -- upstream-sync --check
cargo run -p xtask -- api-coverage --check
cargo run -p xtask -- sample-parity --check
cargo run -p xtask -- generate-pages
cargo run -p xtask -- validate-pages
rustup target add wasm32-unknown-unknown wasm32-wasip1
cargo install wasm-bindgen-cli --version 0.2.126 --locked
cargo run -p xtask -- verify-wasm --runtime
cargo run -p xtask -- build-pages-wasm
npm ci --ignore-scripts
npx playwright install chromium
npm run test:pages-browser
# Optional for local builds: expose Binaryen's wasm-opt on PATH or through EMSDK/upstream/bin for smaller Pages assets.
cargo nextest run --workspace
cargo nextest run -p boxdd --test collision_validation --test joint_new_apis --test world_callbacks --test panic_across_ffi_is_caught --test world_and_queries --test dynamic_tree --test events_and_sensors --test world_destroy_and_recycle --test material_mix_callbacks --test user_data --test ffi_lifecycle --test buffer_reuse
cargo nextest run -p boxdd-sys --test layout
cargo nextest run -p boxdd -p boxdd-sys --features boxdd/double-precision
cargo nextest run -p bevy_boxdd
cargo nextest run -p bevy_boxdd --features double-precision
cargo nextest run -p boxdd --test serde_values --features serde
cargo nextest run -p boxdd --test serde_values --features "double-precision serde"
cargo nextest run -p boxdd --test mint_interop --test nalgebra_interop --test glam_interop --test bytemuck_api --features "double-precision mint nalgebra glam bytemuck"
cargo test -p boxdd-sys --features package-bin --bin package
cargo test -p boxdd-sys --features "package-bin,double-precision,validate,disable-simd" --bin package
cargo test -p boxdd-sys --features "package-bin,simd-avx2" --bin package
cargo check -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed
cargo run -p xtask -- verify-feature-matrix
cargo run -p xtask -- verify-compile-fail
cargo run -p xtask -- verify-wasm --compile-only
cargo run -p xtask -- verify-packages
cargo run -p xtask -- release-contract --check
cargo +nightly-2026-05-27 run -p xtask -- verify-miri
cargo +nightly-2026-05-27 run -p xtask -- verify-sanitizers --address
cargo +nightly-2026-05-27 run -p xtask -- verify-sanitizers --undefined
cargo +nightly-2026-05-27 run -p xtask -- verify-sanitizers --thread
cargo run -p xtask -- verify-semver
cargo clippy --locked -p boxdd-sys --all-targets -- -D warnings
cargo clippy --locked -p boxdd-sys --all-targets --features "double-precision validate disable-simd" -- -D warnings
cargo clippy --locked -p boxdd-sys --features package-bin --bin package -- -D warnings
cargo clippy --locked -p boxdd-sys --features "package-bin,double-precision,validate,disable-simd" --bin package -- -D warnings
cargo clippy --locked -p boxdd-sys --features "package-bin,simd-avx2" --bin package -- -D warnings
cargo clippy --locked -p boxdd --all-targets --features "serde mint nalgebra glam bytemuck unchecked" -- -D warnings
cargo clippy --locked -p boxdd --all-targets --features "double-precision serde mint nalgebra glam bytemuck unchecked validate disable-simd" -- -D warnings
cargo clippy --locked -p bevy_boxdd --all-targets --no-default-features -- -D warnings
cargo clippy --locked -p bevy_boxdd --all-targets --features double-precision -- -D warnings
$env:RUSTDOCFLAGS='-D warnings --cfg docsrs'; cargo doc --workspace --no-deps
$env:RUSTDOCFLAGS='-D warnings --cfg docsrs'; cargo doc -p boxdd --no-deps --features double-precision
$env:RUSTDOCFLAGS='-D warnings --cfg docsrs'; cargo doc -p bevy_boxdd --no-deps --features double-precision
cargo package -p boxdd-sys --allow-dirty --no-verify
cargo package -p boxdd --allow-dirty --no-verify
cargo package -p bevy_boxdd --allow-dirty --no-verify
```

Use `cargo test` only as a fallback when nextest is unavailable, except for the package-helper
coordinates above, which are intentionally explicit `cargo test` unit gates.

`verify-miri` keeps strict-provenance, symbolic-alignment, and leak checking enabled for ordinary
pure-Rust suites. It disables only Miri's end-of-process leak report for the explicit callback and
foundation tests whose contract intentionally retains state rather than dropping it at an unsafe
thread or callback boundary. The `xtask` model test pins that allowlist.

AddressSanitizer keeps LeakSanitizer enabled for the complete mixed C/Rust suite. Three exact tests
in `owned_destruction` and `replay` intentionally retain a losing panic payload while preserving
the primary panic or completing all cleanup; they run as named follow-up tests with only
LeakSanitizer disabled. The allowlist is model-tested and does not weaken buffer, use-after-free,
or double-free detection for those tests.

The mixed Rust/C sanitizer commands require the compiler and runtime support available on the
Linux CI runners. A platform or linker incompatibility fails closed before any test executes.
ThreadSanitizer additionally requires `rust-src` and rebuilds the
standard library with `-Z build-std`; the gate never suppresses sanitizer ABI mismatch diagnostics.
`verify-semver` accepts the intentional `0.5.0` to `0.6.0` break and also requires the pinned
SemVer tool to reject the same API delta when forced to patch-release rules.

`verify-wasm --compile-only` qualifies `boxdd-sys` and the callback-free `boxdd` surface in both
precisions on both installed WASM targets without selecting a runtime provider. It also compiles
category-specific negative probes and requires foundation/world/replay callbacks, callback-backed
world and recording-session queries, dynamic-tree traversal, raw task callbacks, and debug draw to
be absent. A probe that successfully compiles is a gate failure. `wasm32-wasip1` is intentionally compile-only;
`boxdd-sys` rejects `BOXDD_SYS_PROVIDER=wasm-provider` there. Runtime provider smoke is reserved for
the `wasm32-unknown-unknown` Emscripten route.

The CI supply-chain gate installs `cargo-audit 0.22.2` and runs it against the committed
`Cargo.lock`. `wayland-scanner 0.31.11` resolves to `quick-xml 0.41.0`, so the repository carries
no advisory-ignore configuration. Audit warnings for unmaintained or yanked crates remain visible
and are not suppressed.

`verify-packages` creates an isolated local git-index registry, packages `boxdd-sys`, `boxdd`, and
`bevy_boxdd` in dependency order, checks the normalized manifests, project and upstream licenses,
the crate-owned Sigstore root, and runs fresh consumers whose lockfiles must resolve every internal
crate from that registry. `release-contract` additionally checks the protected tag/commit, exact
Box2D submodule checkout, clean source state, archive ABI/symbol identity, canonical manifest
signatures (including the archive digest and CRT/SIMD coordinates), and signature workflow policy.
Release archive parsing rejects non-canonical entries and bounds each entry, the returned file
total, the entry count, and the complete decompressed tar stream, including metadata headers.
Release artifact names include both `github.run_id` and `github.run_attempt` so a rerun cannot
consume an earlier attempt's inputs.

The Linux `system-provider` matrix runs Rust 1.95.0 and 1.97.1 in both precision modes. Each
coordinate builds a distinct vendored static archive, creates a caller-trusted system attestation
from that exact archive, umbrella header, and matching pregenerated bindings, and passes only that
artifact directory to `qualify-native-provider`. The Rust helper creates its own temporary package,
extraction, consumer, target, and empty `CARGO_HOME` roots; rejects Cargo configuration anywhere in
the temporary working directory's ancestor search path; safely extracts the freshly produced
`.crate` with byte, stream, and entry-count limits; rewrites the fixture dependency with structured
TOML; and requires `cargo metadata --locked` to resolve the direct `boxdd-sys` manifest exactly
inside that extraction root and outside the checkout. Cargo only builds the consumer. The helper
then validates Cargo's structured artifact message, directly executes the exact binary inside the
temporary target root, and supplies the expected identities and per-run nonce only to that process.
The consumer creates, steps, queries, and destroys a world through the `system` provider, validates
the manifest/archive digests, and writes the matching receipt after the lifecycle succeeds.
Local system manifests bind caller-supplied bytes and identity; they are compatibility evidence,
not publisher provenance.

The protected prebuilt workflow builds ten target/precision/CRT artifacts once, signs their
canonical manifests after aggregate validation, and then qualifies every artifact with fresh
consumers under both Rust 1.95.0 and 1.97.1. Each qualification coordinate uses the selected
toolchain explicitly for `cargo package`, `cargo generate-lockfile`, `cargo metadata --locked`, and
`cargo build`, and consumes the packaged crate source rather than the checkout dependency. The helper
selects exactly one downloaded target/precision/CRT archive, adjacent Sigstore bundle, and resolved
Cosign executable. The provider itself receives only those already-local verified inputs; it has no
downloader, fallback, or provider cache. Before invoking Cargo, the helper removes ambient provider,
runner, compiler-wrapper, Rust flags, C toolchain, bindgen, `RUSTC_BOOTSTRAP`, and unstable-Cargo
overrides, sets the isolated Cargo home, and runs every Cargo command from the isolated temporary
root so checkout-local configuration is outside Cargo's search path. Runtime identity and receipt
variables are absent from every Cargo process and exist only for the direct consumer execution.

For local development against a dirty checkout, the helper accepts an explicit `--allow-dirty`
flag so `cargo package` can exercise uncommitted source. The CI and protected release workflows do
not pass this flag, and `release-contract` rejects any workflow that adds it.

Running `release-contract --check` locally validates repository identity and workflow policy, but it
does not manufacture or prove a positive OIDC identity. The signed positive path exists only in the
protected tag workflow, where GitHub issues the token and the later read-only jobs verify the bundle
against the repository, workflow, tag, and immutable commit.

For a fresh version line, run packaging and publishing in dependency order. `boxdd-sys` packages first; `boxdd` package verification can resolve only after `boxdd-sys <version>` is available from crates.io; `bevy_boxdd` package verification can resolve only after `boxdd <version>` is available from crates.io.

## Dependency maintenance policy

The 0.6 ABI-stabilization maintenance window permits exactly two direct dependency updates:
`bevy_egui 0.41.1` and `env_logger 0.11.11`. The latter requires `env_filter 2.0.0`
transitively. Keep `glow 0.17` while `dear-imgui-glow 0.15` uses that graphics-context
type, and reject unrelated direct dependency or lockfile churn.

Security and implementation changes are reviewed separately from those ordinary updates. The 0.6
work removes the optional `cgmath 0.18` integration because of RUSTSEC-2026-0196 and
RUSTSEC-2026-0197, advances the yanked transitive `spin 0.10.0` package exactly to `0.10.1`, and
adds `yaml_serde` for structured workflow-policy validation. These changes do not authorize a
general lockfile refresh.

GitHub Actions are pinned to full immutable commit SHAs with human-readable version comments.
The Pages provider similarly pins Emscripten 6.0.3 to SDK revision
`db04e88298d9916fc51fcd3743045ca3eb695127` and installs `wasm-bindgen-cli 0.2.126`
exactly.

Every checkout sets `persist-credentials: false`. CI jobs declare `contents: read` explicitly. The
prebuilt release workflow is split into read-only build/content validation, an OIDC-only signing
job, signed-content revalidation and authenticated prebuilt-provider qualification, followed by a
contents-write-only draft publisher. Release jobs require a protected tag. The OIDC signing job also
requires the protected `release` environment; repository settings must enforce required reviewers
for that environment and the `v*`/`boxdd-sys-v*` tag ruleset. Immediately before creating the
draft, the publisher resolves the current lightweight or annotated tag through the Git Data API
and requires its peeled commit to equal the immutable workflow `GITHUB_SHA`.

`release-contract` treats the release workflow and the native-provider CI job as executable supply
chain policy, not merely configuration. It accepts only the reviewed workflow, job, strategy,
environment, and step keys; fixes each protected step's order, name, pinned action or run mode; and
binds the complete structured YAML value of every protected step to a reviewed SHA-256 digest.
For the reviewed workflow running the committed verifier on an uncompromised hosted runner,
workflow or job defaults, extra jobs or actions, step-level `PATH` injection, alternate shells,
working directories, and command additions fail closed. When a protected step must change, review
the semantic diff first and update its policy metadata in the same change; never refresh a digest
solely to make the gate pass.

Inside GitHub Actions, the policy source is the workflow blob addressed by the immutable
`GITHUB_SHA`, read through the reviewed absolute system Git path with Git replace objects and
ambient `GIT_*` overrides disabled. The checkout's workflow index entry must also remain an ordinary
tracked entry: `assume-unchanged` and `skip-worktree` are rejected rather than trusted as evidence
of a clean checkout. All other release identity reads use the same fixed Git binary, disable replace
objects, and remove ambient `GIT_*` redirections in GitHub Actions. With no `GITHUB_SHA`, local
development intentionally validates the working-tree workflow and preserves the caller's normal Git
environment so edits and alternate local repository layouts can be tested before commit.

This self-check is a consistency and regression gate, not an independent security boundary. It
still trusts the hosted runner and the verifier implementation from the same commit, and it cannot
prove its own definition before repository-controlled workflow steps execute. Repository rulesets,
required external checks, protected-tag review, and the protected `release` environment must prevent
an adversarial workflow definition from reaching privileged jobs.

## Gate rationale

- `upstream-sync --check` validates the manifest as the sole revision authority, the exact gitlink and clean checkout, sorted target source paths, all seven reviewed recording-source Git identities, the operation registry parsed from the pinned Git object, and named artifact identities.
- `api-coverage --check` validates the structured API and recording-wire contracts against vendored headers, canonical public Rust paths, real test evidence, supported provider modes, precision-specific link symbols, explicit recording capability classes, and ABI fingerprints.
- `sample-parity --check` scans upstream sample registrations, preserves manual mappings, and rejects non-benchmark rows that fall back to bare upstream references without an explicit deferral.
- `generate-pages` rebuilds the GitHub Pages Bevy Web example index from `bevy_boxdd/examples/testbed_2d/scenes.rs`.
- `provider-smoke` builds a Rust `wasm32-unknown-unknown` app, builds an Emscripten Box2D provider module, and verifies the shared-memory runtime under Node.
- `verify-wasm --runtime` runs that Node proof and the matching Chromium proof in both precision modes; CI and local release qualification use this same entry point.
- `build-pages-wasm` requires clean commit-bound inputs, rebuilds the example index, compiles the Bevy + egui testbed with the `wasm-release` profile, runs `wasm-bindgen`, builds the Emscripten Box2D provider, runs `wasm-opt -Oz` when available, and writes runtime assets in `docs/pages/wasm/generated` plus `docs/pages/bevy-testbed/generated`. It also emits a canonical manifest that binds the provider JS/WASM, Bevy JS/WASM, and shim digests to the ABI, precision, upstream, effective-source, canonical Emscripten SDK contract, repository/workflow, and checkout identities; source state is checked before and after the long build.
- `validate-pages` rejects stale generated Pages HTML, a loader that does not pin the exact manifest, non-canonical or identity-mismatched manifests, missing/extra/digest-mismatched runtime assets, and broken local links. The browser verifies all manifest-bound bytes before dynamic import or WASM instantiation and checks the runtime adapter ABI before handing the provider to Rust. `npm run test:pages-browser` then grows the actual shared memory, proves old-buffer detachment, and requires post-growth and continuing Box2D steps in Chromium.
- `boxdd-sys` layout tests protect representative ABI assumptions at the raw FFI boundary.
- `bevy_boxdd` plugin tests verify ECS creation, transform sync, distance/revolute joint lifecycle, contact/sensor messages, entity ray/AABB query mappings, debug draw collection, recoverable input errors, and public non-send boundaries without adding Bevy dependencies to the core crate.

## CI shape

CI should keep heavy checks staged:

- Fast lint: `cargo fmt` plus default and double-precision clippy coordinates for `boxdd-sys`,
  `boxdd`, and `bevy_boxdd`, without enabling mutually exclusive feature combinations.
- Native matrix: default and double-precision `boxdd`/`boxdd-sys` nextest suites and C ABI probes
  run on Linux, macOS, and Windows. Linux additionally runs full workspace integration.
- Downstream and features: Bevy single/double tests, double-precision math interoperability,
  single/double serde runtime round trips, and the feature-gated ImGui + Glow example all compile
  or execute explicitly. `verify-feature-matrix` retains the broader Rust 1.95/1.97.1 coordinates.
- Provider matrix: Rust 1.95.0/1.97.1 fresh packaged-crate consumers run against independently built
  and attested system archives in single/double precision. Protected prebuilt consumers repeat both
  toolchains for every target/precision/CRT artifact after signed aggregate verification;
  isolated-registry package consumers and WASM runtime quadrants run in their dedicated jobs.
- Pages runtime: install `wasm32-unknown-unknown`, `wasm-bindgen-cli 0.2.126`, and Emscripten 6.0.3 at revision `db04e88298d9916fc51fcd3743045ca3eb695127`, expose `emsdk/upstream/bin` so `wasm-opt` can be found, then run `cargo run -p xtask -- build-pages-wasm`, install the pinned Playwright dependencies and Chromium, run `npm run test:pages-browser`, and finally run `cargo run -p xtask -- validate-pages`.
- Docs: set `RUSTDOCFLAGS` to `-D warnings --cfg docsrs`, run workspace rustdoc, then build `boxdd`
  and `bevy_boxdd` rustdoc again in double precision.
- Packaging: `cargo package -p boxdd-sys --allow-dirty --no-verify`, then `boxdd`, then `bevy_boxdd` as metadata smoke checks in publish order. For a new shared workspace version, dependent package checks are expected to wait until the previous crate in the chain is visible on crates.io. Run full package verification without `--no-verify` before publishing each crate.

`release-contract --check` parses each CI job and rejects removal of the compiler, platform,
precision, workspace, package-helper, provider, WASM, Miri, sanitizer, documentation, or supply-chain
gate. Its provider checks require the exact Rust 1.95.0/1.97.1 axes, reject matrix includes or
excludes, and bind qualification to one exact unconditional `qualify-native-provider` command.
Conditions, shell wrappers, `continue-on-error`, default-toolchain substitutions, and
`--allow-dirty` all fail the policy. This keeps the checked-in workflow and the local Verification
Contract aligned.
