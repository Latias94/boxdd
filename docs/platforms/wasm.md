# WASM Status

WASM support is bound to the pinned Box2D 3.2.0 development snapshot
`56edae79f2949d86142b03450d5d60f63bcf5a6f`, the precision-specific provider ABI, Emscripten
6.0.4, and wasm-bindgen 0.2.126. A successful Cargo check is not a runtime qualification.

## Support Matrix

| Surface | Target | Status | Entry point |
| --- | --- | --- | --- |
| Rust compile check | `wasm32-unknown-unknown` | `boxdd-sys` and callback-free `boxdd`; compile-only by default | `cargo run -p xtask -- verify-wasm --compile-only` |
| Rust compile check | `wasm32-wasip1` | `boxdd-sys` and callback-free `boxdd`; no WASI runtime claim | `cargo run -p xtask -- verify-wasm --compile-only` |
| Provider runtime | `wasm32-unknown-unknown` | Node- and Chromium-qualified in single and double precision | `cargo run -p xtask -- verify-wasm --runtime` |
| Official provider package | Browser and Node | Signed whole-package provenance in single and double precision | Protected tag workflow; `cargo run -p xtask -- qualify-wasm-provider` verifies a signed package |
| GitHub Pages preview | Browser | Single-precision development preview only | `cargo run -p xtask -- build-pages-wasm` |
| Rust callback tables and debug draw | `wasm32` | Compile-time unavailable | Negative compile probes run through `verify-wasm --compile-only` |
| Multiple workers | Provider runtime | Unsupported | `WorkerCount` accepts one |

## Provider Model

`boxdd-sys` selects one explicit WASM adapter with `BOXDD_SYS_PROVIDER`:

- `wasm-compile-only`: pregenerated bindings and type/build qualification without Box2D runtime
  linkage.
- `wasm-provider`: imports the exact required `b2*` symbols from
  `box2d-sys-v2-single` or `box2d-sys-v2-double` on `wasm32-unknown-unknown` only. The Rust and
  Emscripten modules share one `WebAssembly.Memory`; `boxdd-sys` rejects this runtime adapter on
  `wasm32-wasip1`.
There is no prebuilt WASM archive adapter consumed by `boxdd-sys`, and selection never falls back
to another provider. The separately distributed official JavaScript/WASM runtime packages are
built and authenticated by repository-level `xtask` commands; they are not crate build inputs.

`wasm-provider` is a controlled final-binary route, not a dependency-local Cargo switch. Setting
only `BOXDD_SYS_PROVIDER=wasm-provider` fails closed because a dependency build script cannot
propagate the required linker arguments to an arbitrary downstream binary. The repository's
cross-platform `provider-smoke-app`, `provider-smoke`, `verify-wasm --runtime`, and
`build-pages-wasm` entry points inject the versioned final-link opt-in and the complete linker
contract together, then validate the real final `.wasm`. Package consumers use the separate
`wasm-compile-only` route. No supported path asks an application to reproduce the private opt-in
manually.

Provider ABI v2 has two non-overlapping heaps and one memory-growth owner. The provider places its
static data, 1 MiB stack, and Emscripten `emmalloc` heap below 64 MiB. Its bounded `_sbrk64` returns
`ENOMEM` instead of crossing that boundary. The Rust consumer places its data and stack at or above
64 MiB and exposes a `System` heap ending at the 128 MiB initial-memory boundary. The provider is
forbidden from growing memory; production allocation-driven growth belongs to Rust, up to the
shared 512 MiB maximum. The Pages qualification host performs one explicit external growth as a
view-refresh probe. This keeps ordinary Rust allocation semantics and avoids a cross-module general
allocator ABI.

A structural Wasm gate requires the consumer's ordered
`__data_end <= __stack_low < __stack_high <= __heap_base < __heap_end` layout to remain in the Rust
partition, with `__heap_end` exactly equal to 128 MiB. It requires the equivalent provider layout
to remain below 64 MiB, rejects provider `memory.grow` instructions and resize-heap imports, and
requires both modules to import the exact shared memory limits. Active data segments with dynamic
or negative offsets, arithmetic overflow, a nonzero memory index, or a range outside the owning
partition are also rejected. Applications may use normal Rust allocation; the final-binary gate,
not a dependency-local allocator override, proves that the address partitions remain disjoint.

Before runtime assets are assembled, the structural gate compares every function type imported by
the compiled consumer with the same-named export in the compiled provider. The provider's complete
public function namespace and memory contract are validated separately, without maintaining a
second hand-written C ABI type registry. Build and package identity bind the upstream SHA,
precision, provider ABI, target, memory model, memory partitions, and validation/SIMD policy. At
startup the runtime adapter separately checks the native identity fields it can observe directly:
upstream and effective-source identity, precision, adapter/private ABI, snapshot/recording versions,
and layout identity. The Rust-side check remains a defense-in-depth assertion after linking.

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
generating bindings. Maintainers can regenerate the checked-in bindings only with the pinned
sysroot:

```text
BOXDD_SYS_WASI_SYSROOT=/path/to/wasi-libc-32/sysroot \
  cargo run -p xtask -- upstream-sync --write
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
adapter identity in both precision modes. It also proves that the Rust-only `WorldDef` and
single-worker provider contract reach world creation without any raw task/material callback seam.
Allocator qualification interleaves retained Rust pressure with a live Box2D world, checks an
explicit 64 KiB-aligned Rust `Layout`, forces Rust `System` to grow the shared memory, verifies
allocation contents after each physics step, and requires Box2D byte counts to return to their
baseline after release. The provider is structurally forbidden from growing memory and refreshes
its Emscripten heap view after Rust growth.
The Pages browser proof grows the shared memory by one page only when the explicit proof query is
present, verifies stale JavaScript and Emscripten views are detached and refreshed, checks shared
read/write visibility, and then requires a later Bevy physics step. Normal Pages loads do not run
this proof path.
The browser test uses a local allowlisted HTTP server and the same generated artifacts as the Node
runner.

```text
rustup target add wasm32-unknown-unknown wasm32-wasip1

# Does not require Emscripten: boxdd-sys consumes a checked-in provider ABI contract.
cargo run -p xtask -- provider-smoke-app

# Install and activate the supported toolchain before runtime or Pages commands.
git clone https://github.com/emscripten-core/emsdk.git /absolute/path/emsdk
git -C /absolute/path/emsdk checkout 224ec5f9f2f72f09f9ce0e26d66bae7dbd8b692f
/absolute/path/emsdk/emsdk install 6.0.4
/absolute/path/emsdk/emsdk activate 6.0.4
source /absolute/path/emsdk/emsdk_env.sh

# Source-provider runtime qualification requires the supported toolchain.
cargo run -p xtask -- wasm-provider-contract --check
cargo run -p xtask -- provider-smoke
BOXDD_WASM_PRECISION=double cargo run -p xtask -- provider-smoke
cargo run -p xtask -- verify-wasm --runtime

# Build one unsigned release input. The protected workflow signs it before qualification.
cargo run -p xtask -- build-wasm-provider-package --precision single --output /tmp/boxdd-wasm
cargo run -p xtask -- build-wasm-provider-package --precision double --output /tmp/boxdd-wasm

cargo run -p xtask -- build-pages-wasm
```

`provider-smoke-app` only prepares the Rust side. `provider-smoke` performs the runtime proof.
`verify-wasm --runtime` is the reusable release gate: it runs the Node and Chromium proofs in both
precision modes. Neither a compile-only check nor a prepared app should be described as working
WASM physics.
`wasm-provider-contract --check` recompiles both precision-specific ABI probes without modifying the
repository. After an intentional source, binding, adapter, or Emscripten update, maintainers use
`wasm-provider-contract --write` to generate both canonical TOML files and refresh their manifest
digests. Inputs are revalidated before and after generation, and each output is installed atomically;
Git remains the recovery mechanism if the process is interrupted between files.
The `boxdd-sys` build consumes the selected checked-in contract under `boxdd-sys/abi` and never
discovers, downloads, extracts, caches, or executes Emscripten. Repository source-provider builds,
`verify-wasm --runtime`, and Pages builds are owned by `xtask`; these commands locate `emcc`, Node,
npm, and `wasm-opt` from the activated environment and reject an unsupported Emscripten or Node
version. Installing, updating, and activating the SDK remains the responsibility of Emsdk or CI.
By contrast, `qualify-wasm-provider` consumes an already-built signed package and never executes
Emscripten.

The Rust consumer build is separately pinned to the repository development Cargo/Rustc toolchain;
ambient Cargo environment overrides, compiler wrappers, target/profile overrides, and injected
flags are scrubbed before provider and Pages compilation. Repository and user-selected Cargo
configuration, including source and build configuration, remains part of the trusted local
toolchain boundary.

## Official Release Packages

The protected tag workflow builds exactly two runtime packages for the current 0.6.0 line:

- `boxdd-wasm-provider-0.6.0-wasm32-unknown-unknown-single.tar.gz`
- `boxdd-wasm-provider-0.6.0-wasm32-unknown-unknown-double.tar.gz`

Each canonical archive contains exactly `manifest.toml`, `checksums.sha256`, the matching
`provider/box2d-sys-v2-<precision>.js` and `.wasm`, the upstream Box2D license, and the project's
Apache-2.0 and MIT licenses. Its canonical provenance statement binds the complete outer archive
to the repository, protected workflow, tag, immutable commit and run, plus the full provider ABI,
source, Emscripten version, bindings, layout, precision, validation and SIMD identities.

An unprivileged build job provisions the pinned SDK and creates each archive. A protected OIDC job
signs the canonical statement. The later read-only qualification job snapshots the archive,
statement, Sigstore bundle and trust root, authenticates publisher and outer-archive identity, and
only then performs bounded extraction. It runs a fresh Rust consumer and the extracted provider
under Node and Chromium without provisioning or executing Emscripten. This ordering prevents an
unauthenticated JavaScript module or WASM binary from executing during qualification.

`qualify-wasm-provider` is therefore a release-consumer command, while
`build-wasm-provider-package` is a repository build command. Neither capability belongs in
`boxdd-sys`.

## Pages

The browser path uses `bevy_boxdd/examples/testbed_2d`, Bevy Web + egui, the single-precision v2
provider shim, and generated assets under `docs/pages`. It is a development preview, not the
portable signed distribution. Double-precision Pages generation is rejected; outside the Pages
build, both precisions are runtime-qualified under Node and Chromium.

```text
cargo run -p xtask -- build-pages-wasm
cargo run -p xtask -- validate-pages
npm ci --ignore-scripts
npx playwright install chromium
npm run test:pages-browser
```

The build uses the `wasm-release` profile, Cargo's locked wasm-bindgen support, and the activated
toolchain's Binaryen `wasm-opt -Oz`. Set `BOXDD_PAGES_WASM_PROFILE=debug` or `release` to override the
profile, and `BOXDD_PAGES_WASM_OPT=0` to skip optimization while debugging generated output.

`build-pages-wasm` requires clean commit-bound inputs outside `docs/pages`. It writes the canonical
`wasm/generated/boxdd-pages-runtime-v2.json` manifest for the provider JavaScript/WASM, Bevy
JavaScript/WASM, and provider shim. The manifest binds every byte length and SHA-256 to the v2
provider and adapter ABI, precision, target, crate version, upstream commit/tree, effective-source
digest, adapter-source digest, the selected WASM provider ABI-contract digest, Emscripten version,
recording contract, repository, workflow, and checkout commit. It rechecks the
clean input state and source identity before manifest creation and again after the loader is written.
`validate-pages` reparses the strict schema, requires canonical bytes and the exact asset set, then
recomputes every identity and digest from a clean non-Pages worktree.
The provider-contract digest identifies the ABI identity consumed by `boxdd-sys`; the recorded
Emscripten version identifies the compiler release used for the generated assets.

The generated loader binds only the stable v2 runtime contract. It fetches the strict manifest with
`no-store`, uses each asset SHA-256 as its cache key, and verifies all five assets with Web Crypto
before importing JavaScript or instantiating either WASM module. The provider ABI and partitioned
memory contract are checked while building the exact assets named by that manifest. Release-specific
source and artifact identities remain in the manifest and are validated by `validate-pages`; they
are not embedded in the cacheable loader. The committed loader intentionally has no runtime
contract, so only `build-pages-wasm` can produce a runnable current deployment.

The v2 Bevy app and shim live under `bevy-testbed/generated-v2`. Pages publishes only the cohort
generated from the current checkout. Running `generate-pages` removes ignored runtime output and
restores the committed non-runnable state, so stale deployment bytes cannot affect static Pages
validation.

`npm run test:pages-browser` serves only regular files rooted under `docs/pages`, rejects path
traversal and symbolic links, and opens the actual generated testbed in Chromium. With its
test-only query flag, the current loader grows the shared memory by one page, proves that the old
`WebAssembly.Memory` buffer views detached, then waits for a later real `b2World_Step` call and
exposes a live counter snapshot. The test also requires further physics steps after that proof.
This demonstrates the shipped Emscripten shim and wasm-bindgen glue still work after Rust-owned
memory growth; it is not a substitute for signed release provenance.

The Pages workflow keeps build permissions read-only, installs Emscripten 6.0.4 from a fixed Emsdk
commit, runs the Chromium proof before final validation, and allows
deployment only from protected `main` through the `github-pages` environment, where GitHub Pages
consumes an OIDC token. This authenticates the deployment but does not create a portable Sigstore
bundle. Tag-bound downloadable runtime provenance remains owned by the protected release signing
flow; Pages must not be presented as a substitute for that release artifact signature.

## Deliberate Boundaries

- `wasm32-wasip1` is compile-only. The removed `wasm_wasi_smoke` runtime example was not evidence
  of a supported WASI native provider.
- Current provider modules do not transport Rust function pointers through an Emscripten function
  table. On `wasm32`, Safe Rust therefore does not expose custom-filter, pre-solve, material-mix,
  foundation-hook, debug-draw/replay-draw, callback-backed `Query` operations acquired from a world
  or recording session, or dynamic-tree visitor entry points. `WorldDef` is a Safe Rust value and
  has no raw task/material callback-pointer fields or setters.
- Callback-free `Query::cast_ray_closest` and `Query::cast_mover` remain available. Overlap
  queries, all-hit ray casts, world shape casts, mover collision-plane collection, and dynamic-tree
  query/ray/box traversal are native-only until a shared function-table ABI receives runtime proof.
  The compile-only gate positively type-checks direct-output operations through borrow-scoped
  `Query` capabilities from `World` and `RecordingSession` before running negative callback probes.
- `DebugDrawCmd`, `DebugDrawOptions`, and `HexColor` remain portable value types, but no wasm32
  operation can ask Box2D to populate commands through a callback table.
- `verify-wasm --compile-only` builds `boxdd-sys` and `boxdd` in both precisions for both declared
  targets, checks the single- and double-precision `boxdd-provider-smoke` Rust consumer through the
  real `wasm-provider` cfg route on `wasm32-unknown-unknown`, then requires category-specific world,
  recording-session, replay, foundation, dynamic-tree, query, and debug-draw callback API probes
  to fail compilation with the named methods absent. This provider-routed check does not claim a
  linked runtime; Emscripten final-link and Node/Chromium behavior remain owned by the runtime gate.
  The compile-only gate binds every nested Cargo command and target-installation check to
  `BOXDD_VERIFY_TOOLCHAIN`, or to the `RUSTUP_TOOLCHAIN` inherited from `cargo +<version>` when the
  explicit override is absent. This keeps the Rust 1.95 and 1.97 coordinates independent.
- Current providers are single-worker. Shared memory or Rust atomics alone do not prove an
  Emscripten pthread, host, callback, and teardown contract.
- Safe Rust exposes no snapshot or recording bytes. Browser storage requires an
  application-owned, versioned schema rather than Box2D's private native object representation.
- General Bevy browser compatibility still depends on an application's renderer features and asset
  packaging; only the generated testbed configuration is qualified here.
