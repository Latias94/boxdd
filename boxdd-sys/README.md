<div align="center">

# boxdd-sys - Low-level FFI for Box2D v3 (C API)

[![Crates.io](https://img.shields.io/crates/v/boxdd-sys.svg?style=flat-square)](https://crates.io/crates/boxdd-sys)
[![Docs](https://docs.rs/boxdd-sys/badge.svg)](https://docs.rs/boxdd-sys)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](#license)

</div>

Builds upstream Box2D v3 C sources from `third-party/box2d` and exposes raw FFI in `boxdd_sys::ffi`.
High-level wrappers live in the companion crate `boxdd`.

## Build Modes
- Default (from source): builds vendored C via `cc` and generates bindings with bindgen.
- Prebuilt (optional): link a precompiled static `box2d` instead of building C.
  - Enable with the `prebuilt` feature or `BOXDD_SYS_USE_PREBUILT=1` (allows auto-download from Releases).
  - Directory: `BOXDD_SYS_LIB_DIR=/path/to/lib` (or legacy `BOX2D_LIB_DIR`).
  - URL: `BOXDD_SYS_PREBUILT_URL=https://.../(libbox2d.a|box2d.lib|*.tar.gz)`.
  - Release auto-download: tries `boxdd-prebuilt-<ver>-<target>-static[-md|mt].tar.gz` on GitHub Releases.
  - Force from-source: `BOXDD_SYS_FORCE_BUILD=1`.
  - Skip native C build: `BOXDD_SYS_SKIP_CC=1` (fast Rust-only iteration).
- Docs.rs/offline: uses pregenerated bindings when present and skips native C build.

## System Linking Helpers
- `dynamic-link` feature: prefer dynamic linking (`dylib=box2d`) when using prebuilt/system libs.
- `pkg-config` feature: probe system-installed `box2d` via `pkg-config` when no explicit lib dir/URL is provided.

## WASM (experimental)
- Targets
  - `wasm32-unknown-emscripten`: builds C when `EMSDK` is set.
  - `wasm32-wasip1`: prefers `WASI_SDK_PATH` for clang/sysroot; otherwise check-only.
  - `wasm32-unknown-unknown`: opt-in native C with `BOXDD_SYS_WASM_CC=1`.
- Notes
  - No prebuilt for WASM targets.
  - Bindgen requires libclang.

## Features
- `simd-avx2`: enable AVX2 on x86_64.
- `disable-simd`: disable all SIMD; overrides `simd-avx2`.
- `validate`: enable internal validation checks.

## Prebuilt Artifacts (CI)
- Workflow: `.github/workflows/prebuilt-binaries.yml`.
- Artifact name: `boxdd-prebuilt-<ver>-<target>-static[-md|mt].tar.gz` containing `lib/` and `include/box2d/`.
- Tags: `boxdd-sys-v*`.

## Notes
- Requires a C toolchain and libclang (bindgen).
- Windows (MSVC) and Unix toolchains supported.

## Acknowledgments
- Thanks to the Rust Box2D bindings project for prior art and inspiration: https://github.com/Bastacyclop/rust_box2d
- Huge thanks to the upstream Box2D project by Erin Catto: https://github.com/erincatto/box2d

## License
- MIT OR Apache-2.0. Upstream Box2D v3 is MIT-licensed.

