<div align="center">

# boxdd-sys - Low-level FFI for Box2D v3 (C API)

[![Crates.io](https://img.shields.io/crates/v/boxdd-sys.svg?style=flat-square)](https://crates.io/crates/boxdd-sys)
[![Docs](https://docs.rs/boxdd-sys/badge.svg)](https://docs.rs/boxdd-sys)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](#license)

</div>

Builds upstream Box2D v3 C sources from `third-party/box2d` and exposes raw FFI in `boxdd_sys::ffi`.
High-level wrappers live in the companion crate `boxdd`.

## Build
- From source: builds vendored Box2D C via `cc`.
- System library (optional): opt into the attested `system` adapter with a caller-owned static
  archive, public header, pregenerated binding file, and `manifest.toml`:
  `BOXDD_SYS_PROVIDER=system BOX2D_LIB_DIR=/path/to/lib BOXDD_SYS_SYSTEM_MANIFEST=/path/to/manifest.toml`.
  The manifest binds the exact archive/header/binding SHA-256, target, precision, CRT, SIMD,
  validation identity, adapter source digest, recording contract, and required adapter symbols.
  Dynamic/name-only linking and silent `pkg-config` discovery are rejected.
- Bindings: uses pregenerated bindings by default to avoid requiring LLVM on CI.
  - `BOXDD_SYS_LINK_KIND` is accepted only as `static` (and is optional).
  - Force bindgen: enable the `bindgen` feature, set `BOXDD_SYS_FORCE_BINDGEN=1`, and ensure `libclang` is available.
- Docs.rs/offline: uses pregenerated bindings and skips native C build.

## Prebuilt Linking

Prebuilt archives use the same static manifest contract plus authenticated publisher provenance:
`BOXDD_SYS_PROVIDER=prebuilt BOXDD_SYS_PREBUILT_MANIFEST=/path/to/manifest.toml
BOXDD_SYS_PREBUILT_BUNDLE=/path/to/artifact.sigstore.json`.
The adapter requires exact Cosign 3.0.6 (override its path with `BOXDD_SYS_COSIGN`) and verifies a
signature over the archive's canonical `manifest.toml`. That signed manifest binds the publisher
workflow, release tag, source commit, target/precision/CRT/SIMD coordinates, ABI identities, and
the exact static archive SHA-256 before linking. The crate never downloads, extracts,
caches, or discovers a library by name. Missing provenance fails closed. A caller who explicitly
trusts a local package can run the package helper's `trust-local-system` command and select the
`system` adapter; that manifest deliberately carries no authenticated provenance claim.

The Sigstore trust anchor is shipped in the crate and pinned by SHA-256. The optional
`BOXDD_SYS_PREBUILT_TRUSTED_ROOT` override is accepted only when its contents have the exact same
digest as the crate-owned anchor, so callers cannot replace the publisher trust policy.

Generate a caller-trusted manifest directly from a compatible local archive, header, and binding
file with:

```text
cargo run -p boxdd-sys --features package-bin --bin package -- \
  attest-local-system \
  /provider-root/lib/libbox2d.a \
  /provider-root/include/box2d/box2d.h \
  /provider-root/bindings/bindings_pregenerated.rs \
  /provider-root/manifest.toml
```

All three inputs must be regular files below the output manifest's directory. The command creates
the manifest without overwriting an existing file. It proves exact compatibility with this crate;
it does not authenticate who produced the archive. `trust-local-system` performs the same explicit
trust conversion for an already verified prebuilt package manifest.

Release packaging requires `BOXDD_SYS_PACKAGE_SOURCE_COMMIT` and
`BOXDD_SYS_PACKAGE_RELEASE_TAG` (the corresponding GitHub Actions variables are accepted). Package
names include target, precision, static link kind, and applicable CRT identity.

## WASM (experimental)
- Targets
  - `wasm32-unknown-unknown`: compile-only by default, or use `BOXDD_SYS_PROVIDER=wasm-provider` to import Box2D symbols from a browser/Emscripten provider module.
  - `wasm32-wasip1`: compile-only qualification only; no WASI runtime is claimed.
- Modes
  - `BOXDD_SYS_PROVIDER=wasm-compile-only`: generate/check bindings and skip native C linkage.
  - `BOXDD_SYS_PROVIDER=wasm-provider`: import symbols from the precision-specific `box2d-sys-v1-single` or `box2d-sys-v1-double` module on `wasm32-unknown-unknown` only; `wasm32-wasip1` and Emscripten-target Rust builds are rejected, and the runtime requires the pinned Emscripten 6.0.3 SDK.
- Notes
  - No prebuilt for WASM targets.
  - Emscripten builds the standalone C provider only. Rust applications target `wasm32-unknown-unknown`; `wasm32-unknown-emscripten` Rust builds are not supported.
  - `wasm-provider` consumes the precision-specific checked-in contract under `abi/`; building this crate never discovers or executes Emscripten. Repository runtime qualification is owned by `xtask` and uses the SDK pinned by `xtask/toolchains/emscripten-sdk.toml`.
  - Maintainers validate both checked contracts with `cargo run -p xtask -- wasm-provider-contract --check` and atomically refresh the pair with `--write`; ordinary consumers do neither.
  - Node and Chromium provider smoke tests verify the runtime adapter identity and all required adapter symbols in both precision modes. GitHub Pages currently qualifies single precision only and rejects `BOXDD_WASM_PRECISION=double`.
  - The high-level `boxdd` crate removes Rust callback-table entry points on `wasm32`. The current provider does not qualify cross-module function pointers for world callbacks, callback-backed queries/tree traversal, raw task callbacks, replay mixers, or debug draw.
  - Bindgen requires libclang.

### Reproducible WASM bindings

Ordinary `wasm-compile-only` builds use the checked-in target- and precision-specific bindings and
do not require a WASI sysroot. Forced bindgen, or a missing checked-in binding, uses Cargo's exact
`TARGET`; `BOXDD_SYS_BINDGEN_TARGET` is only an equality assertion and cannot retarget generation.

`wasm32-unknown-unknown` generation uses only the repository-owned
`src/bindgen_headers/wasm32_unknown_unknown/math.h`. Its directory must contain exactly that one
regular, non-symlink file, whose SHA-256 is
`70e00e274e189af73ed321f6490ec3a0b0c58f00286e87fe7d257bb211bb367d`.
`wasm32-wasip1` generation instead requires `BOXDD_SYS_WASI_SYSROOT` to name a canonical wasi-libc
32 sysroot containing `include/wasm32-wasip1/math.h`. The complete header tree below that directory
is pinned to SHA-256
`0e80041ea13b42db5bcd5dc92d737da7c26e4e5a60b902413a41e09924f37687`.

Maintainers refreshing generated routes must provide that exact sysroot:

```text
BOXDD_SYS_WASI_SYSROOT=/path/to/wasi-libc-32/sysroot \
  cargo run -p xtask -- upstream-sync --refresh-routes
```

## Features
- `simd-avx2`: enable AVX2 on x86_64.
- `disable-simd`: disable all SIMD; overrides `simd-avx2`.
- `validate`: enable internal validation checks.
- `package-bin`: enable the internal `bin/package` helper used by CI to package prebuilt artifacts.

## Notes
- Requires a C toolchain. Bindgen requires `libclang` only when forced (`BOXDD_SYS_FORCE_BINDGEN=1`).
- Windows (MSVC) and Unix toolchains supported.
- `adapter::validate_snapshot` verifies the linked adapter identity before invoking the native
  validator. Its `SnapshotValidationError` distinguishes provider identity failures from native
  `SNAPSHOT_*` content-status failures.

## Acknowledgments
- Thanks to the Rust Box2D bindings project for prior art and inspiration: https://github.com/Bastacyclop/rust_box2d
- Huge thanks to the upstream Box2D project by Erin Catto: https://github.com/erincatto/box2d

## License
- MIT OR Apache-2.0. Upstream Box2D v3 is MIT-licensed.
