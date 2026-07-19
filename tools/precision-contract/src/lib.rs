//! Compile-only fixture for the Box2D precision identity contract.
//!
//! This crate intentionally contains no native calls. Enabling `mixed-dependency` asks `boxdd`
//! for its single-precision API while a second dependency edge enables the double-precision
//! `boxdd-sys` ABI. Cargo unifies the sys feature, and `boxdd` must reject that graph while it is
//! still compiling.

#[cfg(any(feature = "single", feature = "double"))]
pub const SYS_IS_DOUBLE_PRECISION: bool = boxdd_sys::IS_DOUBLE_PRECISION;

#[cfg(feature = "mixed-dependency")]
pub const WRAPPER_WORLD_SCALAR_BYTES: usize = core::mem::size_of::<boxdd::WorldScalar>();
