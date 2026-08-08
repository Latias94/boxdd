//! Repository-owned safety adapter for snapshot and replay internals.

use core::{fmt, mem};

use crate::ffi::b2RecPlayer;

pub use crate::adapter_contract::ADAPTER_ABI_VERSION;
pub const SNAPSHOT_VERSION: u32 = 3;
pub const RECORDING_VERSION_MAJOR: u32 = 3;
pub const RECORDING_VERSION_MINOR: u32 = 2;
pub const SNAPSHOT_FACTS_VERSION: u32 = 1;
pub const SNAPSHOT_ENTRY_VERSION: u32 = 1;

pub type SnapshotStatus = u32;
pub const SNAPSHOT_OK: SnapshotStatus = 0;
pub const SNAPSHOT_NULL_INPUT: SnapshotStatus = 1;
pub const SNAPSHOT_TRUNCATED: SnapshotStatus = 2;
pub const SNAPSHOT_BAD_HEADER: SnapshotStatus = 3;
pub const SNAPSHOT_ABI_MISMATCH: SnapshotStatus = 4;
pub const SNAPSHOT_OVERFLOW: SnapshotStatus = 5;
pub const SNAPSHOT_LIMIT_EXCEEDED: SnapshotStatus = 6;
pub const SNAPSHOT_INVALID_VALUE: SnapshotStatus = 7;
pub const SNAPSHOT_INVALID_REFERENCE: SnapshotStatus = 8;
pub const SNAPSHOT_DUPLICATE: SnapshotStatus = 9;
pub const SNAPSHOT_TRAILING_BYTES: SnapshotStatus = 10;
pub const SNAPSHOT_BUFFER_TOO_SMALL: SnapshotStatus = 11;

pub const SNAPSHOT_ENTRY_BODY: u32 = 1;
pub const SNAPSHOT_ENTRY_SHAPE: u32 = 2;
pub const SNAPSHOT_ENTRY_CHAIN: u32 = 3;
pub const SNAPSHOT_ENTRY_CONTACT: u32 = 4;
pub const SNAPSHOT_ENTRY_JOINT: u32 = 5;
pub const SNAPSHOT_ENTRY_ISLAND: u32 = 6;
pub const SNAPSHOT_ENTRY_SOLVER_SET: u32 = 7;

pub const SNAPSHOT_ENTRY_LIVE: u32 = 0x0000_0001;
pub const SNAPSHOT_ENTRY_REQUIRES_CUSTOM_FILTER: u32 = 0x0000_0002;
pub const SNAPSHOT_ENTRY_REQUIRES_PRE_SOLVE: u32 = 0x0000_0004;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterIdentity {
    pub struct_size: u32,
    pub abi_version: u32,
    pub snapshot_version: u32,
    pub recording_version_major: u32,
    pub recording_version_minor: u32,
    pub snapshot_layout_hash: u32,
    pub pointer_width: u8,
    pub little_endian: u8,
    pub double_precision: u8,
    pub validation_enabled: u8,
    pub private_abi_hash: [u8; 32],
    pub upstream_sha: [u8; 41],
    pub target_abi: [u8; 65],
    pub adapter_source_sha256: [u8; 65],
    pub effective_source_sha256: [u8; 65],
    pub recording_contract_blake3: [u8; 65],
}

impl Default for AdapterIdentity {
    fn default() -> Self {
        Self {
            struct_size: 0,
            abi_version: 0,
            snapshot_version: 0,
            recording_version_major: 0,
            recording_version_minor: 0,
            snapshot_layout_hash: 0,
            pointer_width: 0,
            little_endian: 0,
            double_precision: 0,
            validation_enabled: 0,
            private_abi_hash: [0; 32],
            upstream_sha: [0; 41],
            target_abi: [0; 65],
            adapter_source_sha256: [0; 65],
            effective_source_sha256: [0; 65],
            recording_contract_blake3: [0; 65],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SnapshotLimits {
    pub struct_size: u32,
    pub version: u32,
    pub max_image_bytes: u64,
    pub max_validation_work: u64,
    pub max_entries: u32,
    pub max_array_elements: u32,
    pub max_tree_nodes: u32,
    pub max_hash_capacity: u32,
    pub max_bitset_blocks: u32,
    pub reserved: u32,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            struct_size: size_u32::<Self>(),
            version: SNAPSHOT_FACTS_VERSION,
            max_image_bytes: 256 * 1024 * 1024,
            max_validation_work: 16_000_000,
            max_entries: 1_000_000,
            max_array_elements: 1_000_000,
            max_tree_nodes: 1_000_000,
            max_hash_capacity: 1_000_000,
            max_bitset_blocks: 1_000_000,
            reserved: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SnapshotFacts {
    pub struct_size: u32,
    pub version: u32,
    pub image_bytes: u64,
    pub consumed_bytes: u64,
    pub required_entries: u64,
    pub validation_work: u64,
    pub snapshot_flags: u32,
    pub world_flags: u32,
    pub pool_next: [u32; 7],
    pub pool_free: [u32; 7],
    pub entry_counts: [u32; 7],
    pub requires_custom_filter: u32,
    pub requires_pre_solve: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SnapshotEntry {
    pub struct_size: u32,
    pub version: u32,
    pub kind: u32,
    pub flags: u32,
    pub index: i32,
    pub owner_a: i32,
    pub owner_b: i32,
    pub set_index: i32,
    pub local_index: i32,
    pub color_index: i32,
    pub free_order: i32,
    pub generation: u32,
    pub subtype: u32,
    pub owner_a_prev: i32,
    pub owner_a_next: i32,
    pub owner_b_prev: i32,
    pub owner_b_next: i32,
    pub owner_b_order: i32,
}

#[derive(Debug)]
pub struct SnapshotValidation {
    pub facts: SnapshotFacts,
    pub entries: Vec<SnapshotEntry>,
}

/// Why a snapshot could not be safely validated by the linked adapter.
///
/// The identity variant is returned before any validator call receives Rust-owned output
/// pointers. A native status is returned only after the linked adapter has been authorized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SnapshotValidationError {
    AdapterIdentity(AdapterIdentityError),
    Status(SnapshotStatus),
}

impl fmt::Display for SnapshotValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterIdentity(error) => write!(formatter, "native adapter identity: {error}"),
            Self::Status(status) => write!(formatter, "native snapshot validator status {status}"),
        }
    }
}

impl std::error::Error for SnapshotValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AdapterIdentity(error) => Some(error),
            Self::Status(_) => None,
        }
    }
}

/// One field of the linked native adapter that does not match this Rust crate instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AdapterIdentityField {
    StructSize,
    AbiVersion,
    SnapshotVersion,
    RecordingVersion,
    PointerWidth,
    Endianness,
    Precision,
    Validation,
    UpstreamSha,
    TargetAbi,
    AdapterSource,
    EffectiveSource,
    RecordingContract,
    SnapshotLayout,
    PrivateAbi,
}

impl fmt::Display for AdapterIdentityField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::StructSize => "identity struct size",
            Self::AbiVersion => "adapter ABI version",
            Self::SnapshotVersion => "snapshot version",
            Self::RecordingVersion => "recording version",
            Self::PointerWidth => "pointer width",
            Self::Endianness => "endianness",
            Self::Precision => "precision",
            Self::Validation => "validation mode",
            Self::UpstreamSha => "upstream revision",
            Self::TargetAbi => "target ABI",
            Self::AdapterSource => "adapter source digest",
            Self::EffectiveSource => "effective source digest",
            Self::RecordingContract => "recording contract digest",
            Self::SnapshotLayout => "snapshot layout identity",
            Self::PrivateAbi => "private ABI identity",
        };
        formatter.write_str(name)
    }
}

/// Failure to authorize the linked native adapter before using any Box2D API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AdapterIdentityError {
    Unavailable,
    Mismatch(AdapterIdentityField),
}

impl fmt::Display for AdapterIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("native adapter identity is unavailable"),
            Self::Mismatch(field) => write!(formatter, "native adapter {field} does not match"),
        }
    }
}

impl std::error::Error for AdapterIdentityError {}

const fn size_u32<T>() -> u32 {
    assert!(mem::size_of::<T>() <= u32::MAX as usize);
    mem::size_of::<T>() as u32
}

#[cfg_attr(
    all(target_arch = "wasm32", not(feature = "double-precision")),
    link(wasm_import_module = "box2d-sys-v2-single")
)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "double-precision"),
    link(wasm_import_module = "box2d-sys-v2-double")
)]
unsafe extern "C" {
    pub fn boxddAdapter_AbiVersion() -> u32;
    pub fn boxddAdapter_GetIdentity(out: *mut AdapterIdentity, out_size: usize) -> bool;
    pub fn boxddAdapter_GetSnapshotLayoutHash() -> u32;
    pub static boxddEffectiveSourceSha256: [u8; 65];
    pub fn boxddSnapshot_Validate(
        image: *const u8,
        size: usize,
        limits: *const SnapshotLimits,
        facts: *mut SnapshotFacts,
        entries: *mut SnapshotEntry,
        entry_capacity: usize,
        required_entries: *mut usize,
    ) -> SnapshotStatus;
    pub fn boxddRecPlayer_IsHealthy(player: *const b2RecPlayer) -> bool;
}

/// Returns the runtime identity compiled into the linked adapter.
pub fn runtime_identity() -> Option<AdapterIdentity> {
    let mut identity = AdapterIdentity::default();
    // SAFETY: identity is a writable value of the exact versioned C ABI type.
    let ok = unsafe { boxddAdapter_GetIdentity(&mut identity, mem::size_of_val(&identity)) };
    ok.then_some(identity)
}

/// Verifies a captured identity against this exact Rust crate build.
///
/// `reported_abi_version` and `reported_layout_hash` are separate adapter exports. Comparing them
/// prevents a stale or partially implemented adapter from satisfying the contract with a single
/// fabricated struct. The private C layout cannot be recomputed in Rust, so its non-zero identity
/// is authorized by the independently matched repository adapter-source digest.
pub fn verify_identity(
    identity: &AdapterIdentity,
    reported_abi_version: u32,
    reported_layout_hash: u32,
) -> Result<(), AdapterIdentityError> {
    use AdapterIdentityField as Field;

    let mismatch = |field| Err(AdapterIdentityError::Mismatch(field));
    if identity.struct_size as usize != mem::size_of::<AdapterIdentity>() {
        return mismatch(Field::StructSize);
    }
    if reported_abi_version != ADAPTER_ABI_VERSION || identity.abi_version != ADAPTER_ABI_VERSION {
        return mismatch(Field::AbiVersion);
    }
    if identity.snapshot_version != SNAPSHOT_VERSION {
        return mismatch(Field::SnapshotVersion);
    }
    if identity.recording_version_major != RECORDING_VERSION_MAJOR
        || identity.recording_version_minor != RECORDING_VERSION_MINOR
    {
        return mismatch(Field::RecordingVersion);
    }
    if identity.pointer_width as usize != mem::size_of::<usize>() {
        return mismatch(Field::PointerWidth);
    }
    if identity.little_endian != u8::from(cfg!(target_endian = "little")) {
        return mismatch(Field::Endianness);
    }
    if identity.double_precision != u8::from(cfg!(feature = "double-precision")) {
        return mismatch(Field::Precision);
    }
    if identity.validation_enabled != u8::from(cfg!(feature = "validate")) {
        return mismatch(Field::Validation);
    }
    if !canonical_identity_string(&identity.upstream_sha, crate::UPSTREAM_SHA) {
        return mismatch(Field::UpstreamSha);
    }
    if !canonical_identity_string(&identity.target_abi, crate::TARGET_ABI) {
        return mismatch(Field::TargetAbi);
    }
    if !canonical_identity_string(
        &identity.adapter_source_sha256,
        crate::ADAPTER_SOURCE_SHA256,
    ) {
        return mismatch(Field::AdapterSource);
    }
    if !canonical_identity_string(
        &identity.effective_source_sha256,
        crate::EFFECTIVE_SOURCE_SHA256,
    ) {
        return mismatch(Field::EffectiveSource);
    }
    if !canonical_identity_string(
        &identity.recording_contract_blake3,
        crate::RECORDING_CONTRACT_BLAKE3,
    ) {
        return mismatch(Field::RecordingContract);
    }
    if identity.snapshot_layout_hash != crate::SNAPSHOT_LAYOUT_HASH
        || reported_layout_hash != crate::SNAPSHOT_LAYOUT_HASH
    {
        return mismatch(Field::SnapshotLayout);
    }
    if identity.private_abi_hash != crate::PRIVATE_ABI_HASH {
        return mismatch(Field::PrivateAbi);
    }
    Ok(())
}

/// Reads and authorizes the linked adapter before any non-adapter Box2D FFI call.
pub fn verify_runtime_identity() -> Result<AdapterIdentity, AdapterIdentityError> {
    // SAFETY: these functions take no caller pointers and are the adapter's identity handshake.
    let reported_abi_version = unsafe { boxddAdapter_AbiVersion() };
    let identity = runtime_identity().ok_or(AdapterIdentityError::Unavailable)?;
    // SAFETY: this function takes no caller pointers and returns a fixed-width identity value.
    let reported_layout_hash = unsafe { boxddAdapter_GetSnapshotLayoutHash() };
    verify_identity(&identity, reported_abi_version, reported_layout_hash)?;
    Ok(identity)
}

fn canonical_identity_string<const N: usize>(value: &[u8; N], expected: &str) -> bool {
    let Some(nul) = value.iter().position(|byte| *byte == 0) else {
        return false;
    };
    value[..nul] == *expected.as_bytes() && value[nul..].iter().all(|byte| *byte == 0)
}

/// Fully validates a snapshot and returns its canonical slot facts.
///
/// Before passing Rust-owned output storage to the native validator, this function verifies that
/// the linked adapter matches this exact crate build. Callers therefore receive identity failures
/// separately from untrusted snapshot-content failures.
pub fn validate_snapshot(
    image: &[u8],
    limits: &SnapshotLimits,
) -> Result<SnapshotValidation, SnapshotValidationError> {
    validate_snapshot_with(
        image,
        limits,
        verify_runtime_identity,
        validate_snapshot_native,
    )
}

fn validate_snapshot_with(
    image: &[u8],
    limits: &SnapshotLimits,
    identity: impl FnOnce() -> Result<AdapterIdentity, AdapterIdentityError>,
    validate: impl FnOnce(&[u8], &SnapshotLimits) -> Result<SnapshotValidation, SnapshotStatus>,
) -> Result<SnapshotValidation, SnapshotValidationError> {
    identity().map_err(SnapshotValidationError::AdapterIdentity)?;
    validate(image, limits).map_err(SnapshotValidationError::Status)
}

fn validate_snapshot_native(
    image: &[u8],
    limits: &SnapshotLimits,
) -> Result<SnapshotValidation, SnapshotStatus> {
    let mut facts = SnapshotFacts::default();
    let mut required = 0usize;
    // SAFETY: the adapter accepts arbitrary byte slices and performs no unchecked input access.
    let sizing_status = unsafe {
        boxddSnapshot_Validate(
            image.as_ptr(),
            image.len(),
            limits,
            &mut facts,
            core::ptr::null_mut(),
            0,
            &mut required,
        )
    };
    if sizing_status != SNAPSHOT_BUFFER_TOO_SMALL && sizing_status != SNAPSHOT_OK {
        return Err(sizing_status);
    }
    if required > limits.max_entries as usize || required != facts.required_entries as usize {
        return Err(SNAPSHOT_LIMIT_EXCEEDED);
    }

    let mut entries = Vec::new();
    entries
        .try_reserve_exact(required)
        .map_err(|_| SNAPSHOT_LIMIT_EXCEEDED)?;
    entries.resize(required, SnapshotEntry::default());
    // SAFETY: entries has exactly `required` initialized, writable elements and image remains alive.
    let status = unsafe {
        boxddSnapshot_Validate(
            image.as_ptr(),
            image.len(),
            limits,
            &mut facts,
            entries.as_mut_ptr(),
            entries.len(),
            &mut required,
        )
    };
    if status != SNAPSHOT_OK {
        return Err(status);
    }
    if required != entries.len() || facts.required_entries != entries.len() as u64 {
        return Err(SNAPSHOT_INVALID_VALUE);
    }
    Ok(SnapshotValidation { facts, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum IdentityMutation {
        StructSize,
        ReportedAbiVersion,
        EmbeddedAbiVersion,
        SnapshotVersion,
        RecordingVersionMajor,
        RecordingVersionMinor,
        PointerWidth,
        Endianness,
        Precision,
        Validation,
        UpstreamSha,
        TargetAbi,
        AdapterSource,
        EffectiveSource,
        RecordingContract,
        EmbeddedSnapshotLayout,
        ReportedSnapshotLayout,
        PrivateAbi,
    }

    impl IdentityMutation {
        const ALL: [Self; 18] = [
            Self::StructSize,
            Self::ReportedAbiVersion,
            Self::EmbeddedAbiVersion,
            Self::SnapshotVersion,
            Self::RecordingVersionMajor,
            Self::RecordingVersionMinor,
            Self::PointerWidth,
            Self::Endianness,
            Self::Precision,
            Self::Validation,
            Self::UpstreamSha,
            Self::TargetAbi,
            Self::AdapterSource,
            Self::EffectiveSource,
            Self::RecordingContract,
            Self::EmbeddedSnapshotLayout,
            Self::ReportedSnapshotLayout,
            Self::PrivateAbi,
        ];

        fn expected_field(self) -> AdapterIdentityField {
            match self {
                Self::StructSize => AdapterIdentityField::StructSize,
                Self::ReportedAbiVersion | Self::EmbeddedAbiVersion => {
                    AdapterIdentityField::AbiVersion
                }
                Self::SnapshotVersion => AdapterIdentityField::SnapshotVersion,
                Self::RecordingVersionMajor | Self::RecordingVersionMinor => {
                    AdapterIdentityField::RecordingVersion
                }
                Self::PointerWidth => AdapterIdentityField::PointerWidth,
                Self::Endianness => AdapterIdentityField::Endianness,
                Self::Precision => AdapterIdentityField::Precision,
                Self::Validation => AdapterIdentityField::Validation,
                Self::UpstreamSha => AdapterIdentityField::UpstreamSha,
                Self::TargetAbi => AdapterIdentityField::TargetAbi,
                Self::AdapterSource => AdapterIdentityField::AdapterSource,
                Self::EffectiveSource => AdapterIdentityField::EffectiveSource,
                Self::RecordingContract => AdapterIdentityField::RecordingContract,
                Self::EmbeddedSnapshotLayout | Self::ReportedSnapshotLayout => {
                    AdapterIdentityField::SnapshotLayout
                }
                Self::PrivateAbi => AdapterIdentityField::PrivateAbi,
            }
        }

        fn apply(
            self,
            identity: &mut AdapterIdentity,
            reported_abi_version: &mut u32,
            reported_layout_hash: &mut u32,
        ) {
            match self {
                Self::StructSize => identity.struct_size ^= 1,
                Self::ReportedAbiVersion => *reported_abi_version ^= 1,
                Self::EmbeddedAbiVersion => identity.abi_version ^= 1,
                Self::SnapshotVersion => identity.snapshot_version ^= 1,
                Self::RecordingVersionMajor => identity.recording_version_major ^= 1,
                Self::RecordingVersionMinor => identity.recording_version_minor ^= 1,
                Self::PointerWidth => identity.pointer_width ^= 1,
                Self::Endianness => identity.little_endian ^= 1,
                Self::Precision => identity.double_precision ^= 1,
                Self::Validation => identity.validation_enabled ^= 1,
                Self::UpstreamSha => identity.upstream_sha[0] ^= 1,
                Self::TargetAbi => identity.target_abi[0] ^= 1,
                Self::AdapterSource => identity.adapter_source_sha256[0] ^= 1,
                Self::EffectiveSource => identity.effective_source_sha256[0] ^= 1,
                Self::RecordingContract => identity.recording_contract_blake3[0] ^= 1,
                Self::EmbeddedSnapshotLayout => identity.snapshot_layout_hash ^= 1,
                Self::ReportedSnapshotLayout => *reported_layout_hash ^= 1,
                Self::PrivateAbi => identity.private_abi_hash[0] ^= 1,
            }
        }
    }

    #[test]
    fn rust_layouts_match_the_versioned_header_contract() {
        assert_eq!(mem::size_of::<SnapshotLimits>(), 48);
        assert_eq!(mem::size_of::<SnapshotEntry>(), 72);
        assert_eq!(mem::size_of::<SnapshotFacts>(), 144);
        assert_eq!(mem::size_of::<AdapterIdentity>(), 364);
    }

    #[test]
    fn runtime_adapter_identity_matches_this_crate_instance() {
        verify_runtime_identity().expect("linked adapter must match the Rust crate build");
    }

    #[test]
    fn forged_adapter_identity_is_rejected_field_by_field() {
        let identity = verify_runtime_identity().expect("test adapter identity");

        for mutation in IdentityMutation::ALL {
            let mut forged = identity;
            let mut reported_abi_version = ADAPTER_ABI_VERSION;
            let mut reported_layout_hash = identity.snapshot_layout_hash;
            mutation.apply(
                &mut forged,
                &mut reported_abi_version,
                &mut reported_layout_hash,
            );

            assert_eq!(
                verify_identity(&forged, reported_abi_version, reported_layout_hash),
                Err(AdapterIdentityError::Mismatch(mutation.expected_field())),
                "identity mutation {mutation:?} was not rejected precisely"
            );
        }
    }

    #[test]
    fn canonical_identity_strings_require_exact_bytes_and_zero_padding() {
        assert!(canonical_identity_string(b"abc\0\0", "abc"));
        assert!(!canonical_identity_string(b"abcde", "abc"));
        assert!(!canonical_identity_string(b"ab\0\0\0", "abc"));
        assert!(!canonical_identity_string(b"abc\0x", "abc"));
    }

    #[test]
    fn snapshot_identity_gate_short_circuits_the_native_validator() {
        let identity_calls = core::cell::Cell::new(0);
        let validator_calls = core::cell::Cell::new(0);
        let limits = SnapshotLimits::default();
        let error = validate_snapshot_with(
            b"untrusted snapshot",
            &limits,
            || {
                identity_calls.set(identity_calls.get() + 1);
                Err(AdapterIdentityError::Unavailable)
            },
            |_, _| {
                validator_calls.set(validator_calls.get() + 1);
                Err(SNAPSHOT_BAD_HEADER)
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            SnapshotValidationError::AdapterIdentity(AdapterIdentityError::Unavailable)
        );
        assert_eq!(identity_calls.get(), 1);
        assert_eq!(validator_calls.get(), 0);
    }

    #[test]
    fn snapshot_identity_gate_runs_the_native_validator_after_authorization() {
        let phase = core::cell::Cell::new(0);
        let limits = SnapshotLimits::default();
        let image = b"untrusted snapshot";
        let error = validate_snapshot_with(
            image,
            &limits,
            || {
                assert_eq!(phase.replace(1), 0);
                Ok(AdapterIdentity::default())
            },
            |observed_image, observed_limits| {
                assert_eq!(phase.replace(2), 1);
                assert_eq!(observed_image, image);
                assert!(core::ptr::eq(observed_limits, &limits));
                Err(SNAPSHOT_BAD_HEADER)
            },
        )
        .unwrap_err();

        assert_eq!(error, SnapshotValidationError::Status(SNAPSHOT_BAD_HEADER));
        assert_eq!(phase.get(), 2);
    }
}
