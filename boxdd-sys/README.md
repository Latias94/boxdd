<div align="center">

# boxdd-sys - Low-level FFI for a pinned Box2D 3.2 snapshot

[![Crates.io](https://img.shields.io/crates/v/boxdd-sys.svg?style=flat-square)](https://crates.io/crates/boxdd-sys)
[![Docs](https://docs.rs/boxdd-sys/badge.svg)](https://docs.rs/boxdd-sys)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](#license)

</div>

Builds the Box2D 3.2.0 development snapshot at commit
`56edae79f2949d86142b03450d5d60f63bcf5a6f` from `third-party/box2d` and exposes its C API as raw
FFI in `boxdd_sys::ffi`. It is not ABI-compatible with an arbitrary Box2D 3.2 checkout. High-level
wrappers live in the companion crate `boxdd`.

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

```text
BOXDD_SYS_PROVIDER=prebuilt
BOXDD_SYS_PREBUILT_MANIFEST=/path/to/extracted/manifest.toml
BOXDD_SYS_PREBUILT_PROVENANCE=/path/to/artifact.tar.gz.provenance.toml
BOXDD_SYS_PREBUILT_BUNDLE=/path/to/artifact.tar.gz.provenance.sigstore.json
```

The adapter requires exact Cosign 3.0.6 (override its path with `BOXDD_SYS_COSIGN`) and verifies the
signature over the canonical TOML provenance statement. The statement binds the outer tar archive's
exact file name, byte size, and SHA-256; the strict complete member inventory and per-member
digests; the provider manifest and inner checksums digests; provider and ABI coordinates; and the
repository, workflow, workflow ref, source commit, release tag, run ID, and run attempt.

Repository qualification verifies the signed statement and exact outer archive before extracting
anything. `boxdd-sys` never downloads, extracts, or caches a package: it consumes an already-local
extracted directory, re-verifies the statement, manifest, and complete member inventory, and links
the exact verified static archive bytes. It never discovers a library by name. Missing or
inconsistent provenance fails closed. `PROVIDER_PROVENANCE_SHA256` reports the SHA-256 of the
verified signed statement, not the Sigstore bundle. A caller who explicitly trusts a local package
can run `xtask`'s `native-package trust-local-system` command and select the `system` adapter; that
manifest deliberately carries no authenticated provenance claim.

The Sigstore trust anchor is shipped in the crate and pinned by SHA-256. The optional
`BOXDD_SYS_PREBUILT_TRUSTED_ROOT` override is accepted only when its contents have the exact same
digest as the crate-owned anchor, so callers cannot replace the publisher trust policy.

Generate a caller-trusted manifest directly from a compatible local archive, header, and binding
file with:

```text
cargo run -p xtask -- native-package attest-local-system \
  /boxdd-sys-out/boxdd-build-identity.toml \
  /provider-root/lib/libbox2d.a \
  /provider-root/include/box2d/box2d.h \
  /provider-root/bindings/bindings_pregenerated.rs \
  /provider-root/manifest.toml
```

The build identity must be the explicit schema-v3 marker emitted by `boxdd-sys/build.rs`, and its
adjacent adapter identity must match. Native markers bind the full static-archive SHA-256, so the
attestation command rejects a different archive even when its embedded ABI identity is compatible.
All three provider inputs must be regular files below the output manifest's directory. The command
creates the manifest without overwriting an existing file. It proves exact compatibility with this
crate; it does not authenticate who produced the archive. `trust-local-system` performs the same
explicit trust conversion for an already verified prebuilt package manifest.

Release packaging is repository tooling, not part of the published FFI crate. It requires explicit
`--sys-out`, `--build-identity`, `--output`, `--source-commit`, and `--release-tag` arguments to
`cargo run -p xtask -- native-package build`. Package names include target, precision, static link
kind, and applicable CRT identity. Package headers come from the reviewed materialized effective
source, including public-header transformations, rather than from the unmodified submodule tree.

## WASM (experimental)
- Targets
  - `wasm32-unknown-unknown`: compile-only by default; repository `xtask` runtime and Pages entry points can build the controlled provider route that imports Box2D symbols from a browser/Emscripten provider module.
  - `wasm32-wasip1`: compile-only qualification only; no WASI runtime is claimed.
- Modes
  - `BOXDD_SYS_PROVIDER=wasm-compile-only`: generate/check bindings and skip native C linkage.
  - `wasm-provider`: import symbols from the precision-specific `box2d-sys-v2-single` or `box2d-sys-v2-double` module on `wasm32-unknown-unknown` only; `wasm32-wasip1` and Emscripten-target Rust builds are rejected. This is a controlled final-binary route: setting only `BOXDD_SYS_PROVIDER=wasm-provider` is deliberately rejected because dependency build scripts cannot configure an arbitrary downstream final link. Use the repository `xtask` provider, runtime, Pages, or package-consumer entry points, which inject the versioned opt-in and complete linker arguments together and validate the final Wasm. Official provider bytes are built with the pinned Emscripten 6.0.4 SDK, but consuming them does not require that SDK.
- Notes
  - There is no WASM prebuilt adapter consumed while building `boxdd-sys`. Official
    precision-specific JavaScript/WASM runtime packages are built, authenticated, extracted, and
    qualified by repository-level `xtask` commands and CI.
  - Emscripten builds the standalone C provider only. Rust applications target `wasm32-unknown-unknown`; `wasm32-unknown-emscripten` Rust builds are not supported.
  - Provider ABI v2 uses fixed, non-overlapping memory partitions. Emscripten static data, stack,
    and `emmalloc` stay below 64 MiB; Rust data, stack, and `System` allocation start at 64 MiB.
    The provider cannot grow memory or cross the partition boundary. Production allocation-driven
    growth belongs to Rust, up to the shared 512 MiB maximum. Final-Wasm validation checks both
    layouts, exact memory limits, data segments, provider growth instructions, and the Rust heap end.
  - `wasm-provider` consumes the precision-specific checked-in contract under `abi/`; building this crate never discovers, downloads, extracts, caches, or executes Emscripten. Repository source-provider builds, runtime smoke tests, and Pages builds use an activated Emscripten 6.0.4 toolchain; signed-package qualification consumes existing JavaScript/WASM bytes and does not use Emscripten.
  - Maintainers validate both checked contracts with `cargo run -p xtask -- wasm-provider-contract --check` and refresh both with `--write`; each output is installed atomically, while Git remains the recovery mechanism if a run stops between files. Ordinary consumers do neither.
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

Maintainers refreshing checked-in bindings must provide that exact sysroot:

```text
BOXDD_SYS_WASI_SYSROOT=/path/to/wasi-libc-32/sysroot \
  cargo run -p xtask -- upstream-sync --write
```

## Features
- `simd-avx2`: enable AVX2 on x86_64.
- `disable-simd`: disable all SIMD; overrides `simd-avx2`.
- `validate`: enable internal validation checks.

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
- MIT OR Apache-2.0. The pinned upstream Box2D snapshot is MIT-licensed.
