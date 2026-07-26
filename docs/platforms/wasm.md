# WASM Status

WASM support is bound to the pinned Box2D 3.2.0 development snapshot
`56edae79f2949d86142b03450d5d60f63bcf5a6f`, the precision-specific provider ABI, Emscripten
6.0.3, and wasm-bindgen 0.2.126. A successful Cargo check is not a runtime qualification.

## Support Matrix

| Surface | Target | Status | Entry point |
| --- | --- | --- | --- |
| Rust compile check | `wasm32-unknown-unknown` | `boxdd-sys` and callback-free `boxdd`; compile-only by default | `cargo run -p xtask -- verify-wasm --compile-only` |
| Rust compile check | `wasm32-wasip1` | `boxdd-sys` and callback-free `boxdd`; no WASI runtime claim | `cargo run -p xtask -- verify-wasm --compile-only` |
| Provider runtime | `wasm32-unknown-unknown` | Node- and Chromium-qualified in single and double precision | `cargo run -p xtask -- verify-wasm --runtime` |
| GitHub Pages | Browser | Single-precision provider only | `cargo run -p xtask -- build-pages-wasm` |
| Rust callback tables and debug draw | `wasm32` | Compile-time unavailable | Negative compile probes run through `verify-wasm --compile-only` |
| Multiple workers | Provider runtime | Unsupported | `WorkerCount` accepts one |

## Provider Model

`boxdd-sys` selects one explicit WASM adapter with `BOXDD_SYS_PROVIDER`:

- `wasm-compile-only`: pregenerated bindings and type/build qualification without Box2D runtime
  linkage.
- `wasm-provider`: imports the exact required `b2*` symbols from
  `box2d-sys-v1-single` or `box2d-sys-v1-double` on `wasm32-unknown-unknown` only. The Rust and
  Emscripten modules share one `WebAssembly.Memory`; `boxdd-sys` rejects this runtime adapter on
  `wasm32-wasip1`.
There is no prebuilt WASM archive adapter. Selection never falls back to another provider.

At startup the runtime adapter checks the complete provider identity before Safe Rust creates
physics state: upstream SHA, precision, provider ABI/private ABI, snapshot/recording versions and
layout identity, target, validation/SIMD identity, import/export signatures, and memory contract.
A module with the right names but the wrong identity or function types is rejected.

## Binding Generation Contract

Compile-only users normally consume the six checked-in bindings selected by exact Rust target and
precision. That path does not run bindgen and does not require a WASI sysroot. Bindgen runs only
when `BOXDD_SYS_FORCE_BINDGEN=1` is set or the selected checked-in file is missing. Generation
always targets Cargo's `TARGET`; `BOXDD_SYS_BINDGEN_TARGET`, when present, must equal it exactly and
acts only as an assertion.

The two WASM targets intentionally use different header contracts:

- `wasm32-unknown-unknown` receives the repository-owned
  `boxdd-sys/src/bindgen_headers/wasm32_unknown_unknown` directory before Box2D's include root. It
  must contain exactly one regular, non-symlink `math.h`, pinned to SHA-256
  `70e00e274e189af73ed321f6490ec3a0b0c58f00286e87fe7d257bb211bb367d`. The shim declares the exact
  C17 math calls reachable from all public Box2D headers; WASI and host headers are not used.
- `wasm32-wasip1` requires an explicit canonical `BOXDD_SYS_WASI_SYSROOT` whose clang search root
  contains `include/wasm32-wasip1/math.h`. The full header tree at
  `include/wasm32-wasip1` must be wasi-libc 32 with SHA-256
  `0e80041ea13b42db5bcd5dc92d737da7c26e4e5a60b902413a41e09924f37687`. Missing files, identity
  drift, and path escapes fail before generation.

For either target, ambient include inputs such as `BINDGEN_EXTRA_CLANG_ARGS*`, `CPATH`,
`C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`, `OBJC_INCLUDE_PATH`, and `SDKROOT` are rejected while
generating bindings. Maintainers can regenerate all manifest routes only with the pinned sysroot:

```text
BOXDD_SYS_WASI_SYSROOT=/path/to/wasi-libc-32/sysroot \
  cargo run -p xtask -- upstream-sync --refresh-routes
```

Homebrew's wasi-libc 32 layout is typically
`/opt/homebrew/opt/wasi-libc/share/wasi-sysroot`; the version and tree digest, rather than the
installation path, are authoritative.

## Runtime Qualification

The low-level smoke application lives in `examples-wasm/provider-smoke`. Qualification builds the
Rust module with `BOXDD_SYS_PROVIDER=wasm-provider`, derives the exact imports it needs, builds the
pinned Emscripten provider, and runs both modules under Node and Chromium with shared memory. The
smoke covers world creation/destruction, stepping, absolute-position ABI, callback-free closest-ray
queries, standalone collision casts, representative shape and joint operations, memory growth, and
adapter identity in both precision modes. It also proves that Safe Rust validation rejects raw task
and material callback pointers before world creation. The browser test uses a local allowlisted HTTP
server and the same generated artifacts as the Node runner.

```text
rustup target add wasm32-unknown-unknown wasm32-wasip1
cargo install wasm-bindgen-cli --version 0.2.126 --locked

# Requires the immutable toolchain identity in boxdd-sys/emscripten-sdk.toml.
cargo run -p xtask -- provider-smoke-app
cargo run -p xtask -- provider-smoke
BOXDD_WASM_PRECISION=double cargo run -p xtask -- provider-smoke
cargo run -p xtask -- verify-wasm --runtime
```

`provider-smoke-app` only prepares the Rust side. `provider-smoke` performs the runtime proof.
`verify-wasm --runtime` is the reusable release gate: it runs the Node and Chromium proofs in both
precision modes. Neither a compile-only check nor a prepared app should be described as working
WASM physics.
The build-time provider identity probe requires `EMSDK` to name the clean, detached checkout and
installed release pinned by `boxdd-sys/emscripten-sdk.toml`. `PATH` discovery and
`BOXDD_EMSDK_REVISION` self-attestation are rejected. `BOXDD_SYS_EMCC`, when present, must resolve
to the canonical compiler inside that qualified checkout.

## Pages

The browser path uses `bevy_boxdd/examples/testbed_2d`, Bevy Web + egui, the single-precision v1
provider shim, and generated assets under `docs/pages`. Double-precision Pages generation is
rejected; outside the Pages build, both precisions are runtime-qualified under Node and Chromium.

```text
cargo run -p xtask -- build-pages-wasm
cargo run -p xtask -- validate-pages
npm ci --ignore-scripts
npx playwright install chromium
npm run test:pages-browser
```

The build uses the `wasm-release` profile and applies `wasm-opt -Oz` when Binaryen is available.
Set `BOXDD_PAGES_WASM_PROFILE=debug` or `release` to override the profile, and
`BOXDD_PAGES_WASM_OPT=0` to skip optimization while debugging generated output.

`build-pages-wasm` requires clean commit-bound inputs outside `docs/pages`. It writes the canonical
`wasm/generated/boxdd-pages-runtime-v1.json` manifest for the provider JavaScript/WASM, Bevy
JavaScript/WASM, and provider shim. The manifest binds every byte length and SHA-256 to the v1
provider and adapter ABI, precision, target, crate version, upstream commit/tree, effective-source
digest, adapter-source digest, canonical Emscripten SDK-contract digest, recording contract,
repository, workflow, and checkout commit. It rechecks the clean input state and source identity
before manifest creation and again after the loader is written. `validate-pages` reparses the strict
schema, requires canonical bytes and the exact asset set, then recomputes every identity and digest
from a clean non-Pages worktree.
The SDK digest identifies the repository-owned SDK contract only; it is not a signature or portable
provenance statement for a maintainer's local Emscripten installation tree.

The generated loader contains the manifest SHA-256 and identity as its deployment trust anchor. It
verifies the manifest and all five assets with Web Crypto before importing JavaScript or
instantiating either WASM module, then checks the provider's runtime adapter ABI before exposing it
to the Rust app. The committed loader intentionally has no runtime trust anchor; only
`build-pages-wasm` can produce a runnable deployment.

`npm run test:pages-browser` serves only regular files rooted under `docs/pages`, rejects path
traversal and symbolic links, and opens the actual generated testbed in Chromium. With its test-only
query flag, the loader grows the shared `WebAssembly.Memory`, proves the old buffer detached, then
waits for a later real `b2World_Step` call and exposes a live counter snapshot. The test also
requires further physics steps after that proof. This demonstrates the shipped Emscripten shim and
wasm-bindgen glue still work after memory growth; it is not a substitute for signed release
provenance.

The Pages workflow keeps build permissions read-only, derives its Emscripten, wasm-bindgen, and Node
versions from the canonical SDK contract, runs the Chromium proof before final validation, and allows
deployment only from protected `main` through the `github-pages` environment, where GitHub Pages
consumes an OIDC token. This authenticates the deployment but does not create a portable Sigstore
bundle. Tag-bound downloadable runtime provenance remains owned by the protected release signing
flow; Pages must not be presented as a substitute for that release artifact signature.

## Deliberate Boundaries

- `wasm32-wasip1` is compile-only. The removed `wasm_wasi_smoke` runtime example was not evidence
  of a supported WASI native provider.
- Current provider modules do not transport Rust function pointers through an Emscripten function
  table. On `wasm32`, Safe Rust therefore does not expose custom-filter, pre-solve, material-mix,
  foundation-hook, debug-draw/replay-draw, callback-backed world or recording-session query, or
  dynamic-tree visitor entry points. Raw task callback setters are also absent, and validating a
  raw world definition containing task or material callback pointers fails before world creation.
- Callback-free `World::cast_ray_closest` and `World::cast_mover` remain available. Overlap queries,
  all-hit ray casts, world shape casts, mover collision-plane collection, and dynamic-tree
  query/ray/box traversal are native-only until a shared function-table ABI receives runtime proof.
  The compile-only gate positively type-checks the direct-output operations through `World`,
  `WorldHandle`, and `RecordingSession` before running the negative callback probes.
- `DebugDrawCmd`, `DebugDrawOptions`, and `HexColor` remain portable value types, but no wasm32
  operation can ask Box2D to populate commands through a callback table.
- `verify-wasm --compile-only` builds `boxdd-sys` and `boxdd` in both precisions for both declared
  targets, then requires category-specific world, recording-session, replay, foundation,
  dynamic-tree, query, and debug-draw callback API probes to fail compilation with the named
  methods absent. The gate binds every nested Cargo command and target-installation check to
  `BOXDD_VERIFY_TOOLCHAIN`, or to the `RUSTUP_TOOLCHAIN` inherited from `cargo +<version>` when the
  explicit override is absent. This keeps the Rust 1.95 and 1.97 coordinates independent.
- Current providers are single-worker. Shared memory or Rust atomics alone do not prove an
  Emscripten pthread, host, callback, and teardown contract.
- Snapshot images and recording streams remain precision/provider/private-ABI-bound artifacts.
  Browser storage does not make them a stable cross-release persistence format.
- General Bevy browser compatibility still depends on an application's renderer features and asset
  packaging; only the generated testbed configuration is qualified here.
