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
- System library (optional): link an existing `box2d` installed on the system.
  - Env: set `BOX2D_LIB_DIR=/path/to/lib` and optionally `BOXDD_SYS_LINK_KIND=static|dylib`.
  - Feature: enable `pkg-config` and ensure `box2d` is available via the system.
- Bindings: uses pregenerated bindings by default to avoid requiring LLVM on CI.
  - Note: crate features that affect the C build (e.g. `simd-avx2`, `disable-simd`, `validate`) are ignored when linking a system library. Set `BOXDD_SYS_STRICT_FEATURES=1` to fail the build if such features are enabled.
  - Force bindgen: set `BOXDD_SYS_FORCE_BINDGEN=1` and ensure `libclang` is available.
- Docs.rs/offline: uses pregenerated bindings and skips native C build.

## System Linking
- Supported via env or `pkg-config` (see above). No prebuilt download is provided by this crate.

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

## Notes
- Requires a C toolchain. Bindgen requires `libclang` only when forced (`BOXDD_SYS_FORCE_BINDGEN=1`).
- Windows (MSVC) and Unix toolchains supported.

## Acknowledgments
- Thanks to the Rust Box2D bindings project for prior art and inspiration: https://github.com/Bastacyclop/rust_box2d
- Huge thanks to the upstream Box2D project by Erin Catto: https://github.com/erincatto/box2d

## License
- MIT OR Apache-2.0. Upstream Box2D v3 is MIT-licensed.
