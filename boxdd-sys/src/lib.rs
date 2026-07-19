#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod precision;

#[cfg(test)]
mod build_support;

pub mod ffi;

pub use precision::{
    ABI_PRECISION, ActivePrecision, DoublePrecision, IS_DOUBLE_PRECISION, Precision,
    SinglePrecision,
};

/// Exact upstream Box2D revision described by the build manifest.
pub const UPSTREAM_SHA: &str = env!("BOXDD_SYS_UPSTREAM_SHA");

/// WASM import module selected for this build's precision and provider route.
pub const WASM_IMPORT_MODULE: &str = env!("BOXDD_SYS_WASM_IMPORT_MODULE");
