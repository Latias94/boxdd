//! Fallible error types for `try_*` APIs.
//!
//! The default safe API surface prefers panics on misuse (invalid ids, calling from within a
//! Box2D callback) to prevent Rust-level UB. If you want recoverable errors (e.g. in production),
//! use the `try_*` APIs returning `ApiResult<T>`.
//!
//! Common `ApiError` categories are:
//! - stale ids after an object was destroyed
//! - calling Box2D while the world is locked inside a callback
//! - invalid definitions or strings crossing the FFI boundary
//! - typed user-data mismatches
//! - callback resource exhaustion for advanced callback registration

pub type ApiResult<T> = core::result::Result<T, ApiError>;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ApiError {
    #[error("boxdd API called from a Box2D callback; Box2D world is locked")]
    InCallback,

    #[error("invalid BodyId")]
    InvalidBodyId,
    #[error("invalid ShapeId")]
    InvalidShapeId,
    #[error("invalid JointId")]
    InvalidJointId,
    #[error("invalid joint type for this API")]
    InvalidJointType,
    #[error("invalid ChainId")]
    InvalidChainId,

    #[error("invalid ChainDef")]
    InvalidChainDef,

    #[error("string contains an interior NUL byte")]
    NulByteInString,

    #[error("user data type mismatch")]
    UserDataTypeMismatch,

    #[error("no free callback slot is available for material mixing callbacks")]
    CallbackSlotsExhausted,
}
