#![allow(clippy::approx_constant)]

#[cfg(all(
    target_family = "wasm",
    not(any(
        all(target_arch = "wasm32", target_os = "unknown", target_env = ""),
        all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")
    ))
))]
compile_error!("boxdd-sys supports WASM targets only for wasm32-unknown-unknown and wasm32-wasip1");

include!(env!("BOXDD_SYS_BINDINGS_FILE"));

/// Default collision mask exported by Box2D's public C API.
pub const B2_DEFAULT_MASK_BITS: u64 = u64::MAX;

/// Whether the linked Box2D library was compiled with validation enabled.
#[cfg(feature = "validate")]
pub const B2_ENABLE_VALIDATION: u32 = 1;

/// Whether the linked Box2D library was compiled with validation enabled.
#[cfg(not(feature = "validate"))]
pub const B2_ENABLE_VALIDATION: u32 = 0;

#[cfg(feature = "double-precision")]
pub use b2CreateWorldDoublePrecision as b2CreateWorld;
