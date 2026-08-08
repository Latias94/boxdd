//! Common errors for the safe API.
//!
//! Every fallible safe operation uses its canonical name and returns [`Result`].
//!
//! Common `Error` categories are:
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

pub type Result<T> = core::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    #[error("boxdd API called from a Box2D callback; Box2D world is locked")]
    InCallback,

    #[error("identifier belongs to a different Rust world")]
    WrongWorld,

    #[error("identifier belongs to a different dynamic tree")]
    WrongTree,

    #[error("invalid or stale TreeProxyId")]
    InvalidTreeProxyId,

    #[error("the process exhausted its Rust world identity space")]
    WorldIdentityExhausted,

    #[error("Box2D failed to create a world")]
    WorldCreationFailed,

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

    #[error("snapshot Rust-side commit panicked after native restore")]
    SnapshotCommitPanicked,

    #[error("the native snapshot payload is malformed")]
    InvalidNativeSnapshot,

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
    FoundationInitialization(#[from] crate::FoundationInitError),

    #[error(transparent)]
    FoundationActivity(#[from] crate::FoundationActivityError),

    #[error("invalid BodyId")]
    InvalidBodyId,
    #[error("invalid ShapeId")]
    InvalidShapeId,
    #[error("invalid JointId")]
    InvalidJointId,
    #[error("wrong joint type: expected {expected:?}, got {actual:?}")]
    WrongJointType {
        expected: crate::JointType,
        actual: crate::JointType,
    },
    #[error("wrong shape type: expected {expected:?}, got {actual:?}")]
    WrongShapeType {
        expected: crate::ShapeType,
        actual: crate::ShapeType,
    },
    #[error("Box2D returned unknown native body type discriminant {raw}")]
    InvalidNativeBodyType { raw: u32 },
    #[error("Box2D returned unknown native shape type discriminant {raw}")]
    InvalidNativeShapeType { raw: u32 },
    #[error("Box2D returned a negative allocated byte count: {count}")]
    NegativeAllocatedByteCount { count: i64 },
    #[error("Box2D returned invalid elapsed milliseconds")]
    InvalidNativeElapsedMilliseconds,
    #[error("Box2D returned an invalid angle")]
    InvalidNativeAngle,
    #[error("Box2D returned an invalid rotation")]
    InvalidNativeRotation,
    #[error("invalid ChainId")]
    InvalidChainId,

    #[error("shape is owned by a chain and cannot be mutated or destroyed independently")]
    ChainOwnedShape,
    #[error("invalid ContactId")]
    InvalidContactId,

    #[error("invalid ChainDef")]
    InvalidChainDef,

    #[error(
        "length-scale provenance mismatch during `{operation}` (expected bits {expected:#010x}, actual bits {actual:#010x})"
    )]
    LengthScaleMismatch {
        operation: &'static str,
        expected: u32,
        actual: u32,
    },

    #[error("invalid argument `{argument}` for `{operation}`: expected {constraint}")]
    InvalidArgument {
        operation: &'static str,
        argument: &'static str,
        constraint: &'static str,
    },

    #[error("index {index} is out of range for `{operation}`; expected 0..{bound}")]
    IndexOutOfRange {
        operation: &'static str,
        index: i64,
        bound: usize,
    },

    #[error("Box2D returned invalid worker count {value}")]
    InvalidNativeWorkerCount { value: i32 },

    #[error("Box2D returned invalid world capacity `{field}`={value}: expected {constraint}")]
    InvalidNativeWorldCapacity {
        field: &'static str,
        value: i64,
        constraint: &'static str,
    },

    #[error(
        "Box2D dynamic-tree state is invalid during `{operation}`: `{field}`={value}, expected {constraint}"
    )]
    InvalidNativeDynamicTreeState {
        operation: &'static str,
        field: &'static str,
        value: i64,
        constraint: &'static str,
    },

    #[error("Box2D returned invalid `{output}` from `{operation}`: expected {constraint}")]
    InvalidNativeOutput {
        operation: &'static str,
        output: &'static str,
        constraint: &'static str,
    },

    #[error("worker count {requested} is not supported on this target")]
    UnsupportedWorkerCount { requested: u32 },

    #[error("custom-filter callbacks are not supported by Box2D recording")]
    RecordingCustomFilterUnsupported,

    #[error("pre-solve callbacks are not supported by Box2D recording")]
    RecordingPreSolveUnsupported,

    #[error("Box2D failed to allocate a recording buffer")]
    RecordingAllocationFailed,

    #[error("Box2D returned an invalid recording buffer")]
    InvalidNativeRecording,

    #[error("recording exceeded its configured byte limit")]
    RecordingLimitExceeded,

    #[error("one recorded operation exceeded the native 24-bit payload limit")]
    RecordingOperationTooLarge,

    #[error("native Box2D recording failed validation")]
    RecordingOutputValidationFailed,

    #[error("failed to allocate recording storage")]
    RecordingStorageAllocationFailed,

    #[error("replay mixer identities do not match the recording")]
    ReplayMixerIdentityMismatch,

    #[error("Box2D failed to create a replay player after successful preflight")]
    ReplayNativeCreateFailed,

    #[error("Box2D replay entered a terminal native failure state")]
    ReplayNativeFailure,

    #[error("Box2D returned invalid replay metadata")]
    InvalidNativeReplayMetadata,

    #[error("replay frame is outside the supported native range")]
    ReplayFrameOutOfRange,

    #[error("replay query index is outside the current frame")]
    ReplayQueryOutOfRange,

    #[error("invalid replay keyframe policy")]
    InvalidReplayKeyframePolicy,

    #[error("replay observation epoch exhausted")]
    ReplayEpochExhausted,

    #[error(
        "Box2D replay did not restore length units per meter (expected bits {expected:#010x}, observed {observed:#010x})"
    )]
    ReplayLengthScaleNotRestored { expected: u32, observed: u32 },

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

    #[error("Box2D returned an invalid transient event buffer")]
    InvalidNativeEventBuffer,
}

impl Error {
    #[inline]
    pub const fn invalid_argument(
        operation: &'static str,
        argument: &'static str,
        constraint: &'static str,
    ) -> Self {
        Self::InvalidArgument {
            operation,
            argument,
            constraint,
        }
    }

    #[inline]
    pub const fn index_out_of_range(operation: &'static str, index: i64, bound: usize) -> Self {
        Self::IndexOutOfRange {
            operation,
            index,
            bound,
        }
    }
}
