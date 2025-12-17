//! Fallible error types for `try_*` APIs.
//!
//! The default safe API surface prefers panics on misuse (invalid ids, calling from within a
//! Box2D callback) to prevent Rust-level UB. If you want recoverable errors (e.g. in production),
//! use the `try_*` APIs returning `ApiResult<T>`.

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
    #[error("invalid ChainId")]
    InvalidChainId,

    #[error("invalid ChainDef")]
    InvalidChainDef,

    #[error("string contains an interior NUL byte")]
    NulByteInString,

    #[error("user data type mismatch")]
    UserDataTypeMismatch,
}
