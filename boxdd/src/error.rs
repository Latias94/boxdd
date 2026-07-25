//! Fallible error types for `try_*` APIs.
//!
//! The default safe API surface prefers panics on misuse (invalid ids, calling from within a
//! Box2D callback) to prevent Rust-level UB. If you want recoverable errors (e.g. in production),
//! use the `try_*` APIs returning `ApiResult<T>`.
//!
//! Common `ApiError` categories are:
//! - stale ids after an object was destroyed or a native slot was recycled
//! - identifiers used with a different world or standalone dynamic tree
//! - calling Box2D while the world is locked inside a callback
//! - out-of-range runtime indices on validated handles
//! - invalid numeric values or argument ranges that would otherwise trip Box2D asserts
//! - invalid definitions or strings crossing the FFI boundary
//! - typed user-data mismatches
//! - callback resource exhaustion for advanced callback registration
//! - invalid native output capacities/counts or output-buffer allocation failure
//! - unknown enum discriminants returned by an incompatible or corrupted native provider

pub type ApiResult<T> = core::result::Result<T, ApiError>;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum ApiError {
    #[error("boxdd API called from a Box2D callback; Box2D world is locked")]
    InCallback,

    #[error("identifier belongs to a different Rust world")]
    WrongWorld,

    #[error("identifier belongs to a different dynamic tree")]
    WrongTree,

    #[error("invalid or stale TreeProxyId")]
    InvalidTreeProxyId,

    #[error("raw identifier has the wrong object kind")]
    WrongIdKind,

    #[error("raw identifier uses an unsupported representation")]
    InvalidRawId,

    #[error("the process exhausted its Rust world identity space")]
    WorldIdentityExhausted,

    #[error("the process exhausted its Rust dynamic-tree identity space")]
    TreeIdentityExhausted,

    #[error("an owner exhausted its native object identity space")]
    ObjectIdentityExhausted,

    #[error("failed to allocate native object identity tracking storage")]
    IdentityTrackingAllocationFailed,

    #[error("the world exhausted its user-data value-version space")]
    UserDataVersionExhausted,

    #[error("failed to allocate user-data tracking storage")]
    UserDataAllocationFailed,

    #[error("failed to allocate snapshot transaction storage")]
    SnapshotAllocationFailed,

    #[error("the native snapshot operation failed")]
    SnapshotNativeFailed,

    #[error("snapshot bytes are malformed")]
    InvalidSnapshotImage,

    #[error("snapshot envelope checksum mismatch")]
    SnapshotChecksumMismatch,

    #[error("snapshot ABI identity is incompatible with this build")]
    SnapshotAbiMismatch,

    #[error("snapshot capability belongs to a different world")]
    ForeignSnapshot,

    #[error("snapshot host manifest does not match its native identity manifest")]
    SnapshotManifestMismatch,

    #[error("snapshot requires host callbacks which are not installed")]
    SnapshotCallbacksUnavailable,

    #[error("snapshot host callback or material-mixer wiring does not match the captured world")]
    SnapshotHostWiringMismatch,

    #[error("user data is already borrowed incompatibly by this call stack")]
    ReentrantAccess,

    #[error("world is poisoned")]
    WorldPoisoned,

    #[error("world has been destroyed")]
    WorldDestroyed,

    #[error("world is busy recording or restoring state")]
    WorldBusy,

    #[error(transparent)]
    FoundationActivity(#[from] crate::FoundationActivityError),

    #[error("invalid BodyId")]
    InvalidBodyId,
    #[error("invalid ShapeId")]
    InvalidShapeId,
    #[error("invalid JointId")]
    InvalidJointId,
    #[error("invalid joint type for this API")]
    InvalidJointType,
    #[error("Box2D returned unknown native body type discriminant {raw}")]
    InvalidNativeBodyType { raw: u32 },
    #[error("Box2D returned unknown native shape type discriminant {raw}")]
    InvalidNativeShapeType { raw: u32 },
    #[error("Box2D returned unknown native joint type discriminant {raw}")]
    InvalidNativeJointType { raw: u32 },
    #[error("invalid ChainId")]
    InvalidChainId,

    #[error("shape is owned by a chain and cannot be mutated or destroyed independently")]
    ChainOwnedShape,
    #[error("invalid ContactId")]
    InvalidContactId,

    #[error("invalid ChainDef")]
    InvalidChainDef,

    #[error("index out of range for this API")]
    IndexOutOfRange,

    #[error("invalid argument for this API")]
    InvalidArgument,

    #[error("worker count {requested} is not supported on this target")]
    UnsupportedWorkerCount { requested: u32 },

    #[error("worker count is fixed by the world's raw task-system contract")]
    RawTaskSystemWorkerCountFixed,

    #[error("custom-filter callbacks are not supported by Box2D recording")]
    RecordingCustomFilterUnsupported,

    #[error("pre-solve callbacks are not supported by Box2D recording")]
    RecordingPreSolveUnsupported,

    #[error("Box2D failed to allocate a recording buffer")]
    RecordingAllocationFailed,

    #[error("Box2D returned an invalid recording buffer")]
    InvalidNativeRecording,

    #[error("string contains an interior NUL byte")]
    NulByteInString,

    #[error("user data type mismatch")]
    UserDataTypeMismatch,

    #[error("no free callback slot is available for material mixing callbacks")]
    CallbackSlotsExhausted,

    #[error("Box2D requested a negative FFI output capacity: {capacity}")]
    NegativeFfiOutputCapacity { capacity: i32 },

    #[error("Box2D returned a negative FFI output count: {count}")]
    NegativeFfiOutputCount { count: i32 },

    #[error("Box2D returned {count} FFI output values for capacity {capacity}")]
    FfiOutputCountExceedsCapacity { count: i32, capacity: i32 },

    #[error("failed to allocate an FFI output buffer")]
    FfiOutputAllocationFailed,
}
