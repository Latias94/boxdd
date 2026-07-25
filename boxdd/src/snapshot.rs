//! Transactional Box2D world snapshots.
//!
//! `Snapshot` is an in-process capability tied to its origin world. `SnapshotImage` is a
//! validated byte envelope which may only create a fresh world.

use core::fmt;
use core::marker::PhantomData;
use core::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use boxdd_sys::adapter::{AdapterIdentity, SnapshotValidation};
use boxdd_sys::ffi;

use crate::core::world_core::{ActivityState, LifecycleState, WorldCore};
use crate::error::{ApiError, ApiResult};
use crate::id::{IdBrand, WorldToken};
use crate::types::{BodyId, ChainId, JointId, ShapeId};
use crate::world::{WorkerCount, World};

const IMAGE_MAGIC: [u8; 8] = *b"BOXDDSNP";
const IMAGE_SCHEMA: u32 = 3;
const IMAGE_HEADER_LEN: usize = 425;
const IMAGE_CHECKSUM: Range<usize> = 24..56;
const IMAGE_EFFECTIVE_SOURCE_SHA256: Range<usize> = 295..360;
const REQUIRE_FRICTION_MIXER: u32 = 1 << 0;
const REQUIRE_RESTITUTION_MIXER: u32 = 1 << 1;
const REQUIRE_CUSTOM_FILTER: u32 = 1 << 2;
const REQUIRE_PRE_SOLVE: u32 = 1 << 3;
const KNOWN_HOST_REQUIREMENTS: u32 =
    REQUIRE_FRICTION_MIXER | REQUIRE_RESTITUTION_MIXER | REQUIRE_CUSTOM_FILTER | REQUIRE_PRE_SOLVE;

/// An unforgeable, process-local snapshot capability.
///
/// This type has no public constructor and is deliberately not serializable. Only a snapshot
/// taken from the same live `World` can authorize in-place restore.
pub struct Snapshot {
    origin: IdBrand,
    image: SnapshotImage,
    identities: crate::core::identity_registry::IdentityManifest,
    user_data: crate::core::user_data::UserDataManifest,
    _owner_thread: PhantomData<Rc<()>>,
}

impl Snapshot {
    /// Return the ABI-bound external image.
    ///
    /// It cannot authorize in-place restore on any world and is not a cross-target or
    /// cross-version persistence format.
    pub fn image(&self) -> &SnapshotImage {
        &self.image
    }
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("origin", &"World(..)")
            .field("native_bytes", &self.image.native_payload().len())
            .finish_non_exhaustive()
    }
}

/// A versioned, integrity-checked snapshot byte envelope.
///
/// Parsing bytes never grants authority to restore an existing world. Use `load` or
/// `World::from_snapshot_image` to create a world with a fresh Rust identity domain. This format
/// is bound to the exact target ABI, precision, upstream revision, effective C source identity,
/// and adapter layout; it is not a portable cross-version persistence format.
pub struct SnapshotImage {
    bytes: Vec<u8>,
    payload: Range<usize>,
    identity: AdapterIdentity,
    validation: SnapshotValidation,
    host_requirements: SnapshotHostRequirements,
}

impl SnapshotImage {
    /// Parse, checksum, ABI-check, and deeply validate an external image.
    pub fn from_bytes(bytes: &[u8]) -> ApiResult<Self> {
        let (parsed, validation) = validate_image(bytes)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| ApiError::SnapshotAllocationFailed)?;
        owned.extend_from_slice(bytes);
        Ok(Self::from_validated_bytes(owned, parsed, validation))
    }

    /// Return the complete envelope, including compatibility metadata and checksum.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the image and return its complete envelope.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Create a fresh world with a fresh token, nonces, and empty host registries.
    pub fn load(&self, worker_count: WorkerCount) -> ApiResult<SnapshotLoad> {
        World::from_snapshot_image(self, worker_count)
    }

    fn from_native(
        payload: Vec<u8>,
        identity: AdapterIdentity,
        host_requirements: SnapshotHostRequirements,
    ) -> ApiResult<Self> {
        Self::from_owned_bytes(encode_image(&payload, &identity, host_requirements)?)
    }

    fn from_owned_bytes(bytes: Vec<u8>) -> ApiResult<Self> {
        let (parsed, validation) = validate_image(&bytes)?;
        Ok(Self::from_validated_bytes(bytes, parsed, validation))
    }

    fn from_validated_bytes(
        bytes: Vec<u8>,
        parsed: ParsedEnvelope,
        validation: SnapshotValidation,
    ) -> Self {
        Self {
            bytes,
            payload: parsed.payload,
            identity: parsed.identity,
            validation,
            host_requirements: parsed.host_requirements,
        }
    }

    fn native_payload(&self) -> &[u8] {
        &self.bytes[self.payload.clone()]
    }
}

impl fmt::Debug for SnapshotImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnapshotImage")
            .field("schema", &IMAGE_SCHEMA)
            .field("envelope_bytes", &self.bytes.len())
            .field("native_bytes", &self.native_payload().len())
            .field("snapshot_layout_hash", &self.identity.snapshot_layout_hash)
            .field("host_requirements", &self.host_requirements)
            .finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct SnapshotHostRequirements(u32);

impl SnapshotHostRequirements {
    fn capture(core: &WorldCore) -> Self {
        let (friction, restitution) = core.mixer_presence();
        let (custom_filter, pre_solve) = core.snapshot_callback_presence();
        let mut flags = 0;
        if friction {
            flags |= REQUIRE_FRICTION_MIXER;
        }
        if restitution {
            flags |= REQUIRE_RESTITUTION_MIXER;
        }
        if custom_filter {
            flags |= REQUIRE_CUSTOM_FILTER;
        }
        if pre_solve {
            flags |= REQUIRE_PRE_SOLVE;
        }
        Self(flags)
    }

    fn parse(flags: u32, reserved: u32) -> ApiResult<Self> {
        if flags & !KNOWN_HOST_REQUIREMENTS != 0 || reserved != 0 {
            return Err(ApiError::InvalidSnapshotImage);
        }
        Ok(Self(flags))
    }

    fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn matches(self, core: &WorldCore) -> bool {
        self == Self::capture(core)
    }
}

/// Safe-ID mappings published by a successful in-place restore.
pub struct SnapshotRestore {
    bodies: Vec<(BodyId, BodyId)>,
    shapes: Vec<(ShapeId, ShapeId)>,
    joints: Vec<(JointId, JointId)>,
    chains: Vec<(ChainId, ChainId)>,
}

impl SnapshotRestore {
    pub fn body_id(&self, snapshot_id: BodyId) -> Option<BodyId> {
        map_id(&self.bodies, snapshot_id)
    }

    pub fn shape_id(&self, snapshot_id: ShapeId) -> Option<ShapeId> {
        map_id(&self.shapes, snapshot_id)
    }

    pub fn joint_id(&self, snapshot_id: JointId) -> Option<JointId> {
        map_id(&self.joints, snapshot_id)
    }

    pub fn chain_id(&self, snapshot_id: ChainId) -> Option<ChainId> {
        map_id(&self.chains, snapshot_id)
    }
}

impl fmt::Debug for SnapshotRestore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnapshotRestore")
            .field("bodies", &self.bodies.len())
            .field("shapes", &self.shapes.len())
            .field("joints", &self.joints.len())
            .field("chains", &self.chains.len())
            .finish()
    }
}

/// A freshly loaded world and the Safe IDs minted in its new identity domain.
pub struct SnapshotLoad {
    world: World,
    bodies: Vec<BodyId>,
    shapes: Vec<ShapeId>,
    joints: Vec<JointId>,
    chains: Vec<ChainId>,
}

impl SnapshotLoad {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn into_world(self) -> World {
        self.world
    }

    pub fn body_ids(&self) -> &[BodyId] {
        &self.bodies
    }

    pub fn shape_ids(&self) -> &[ShapeId] {
        &self.shapes
    }

    pub fn joint_ids(&self) -> &[JointId] {
        &self.joints
    }

    pub fn chain_ids(&self) -> &[ChainId] {
        &self.chains
    }

    fn new(world: World) -> ApiResult<Self> {
        let manifest = world.core().identity_manifest()?;
        Ok(Self {
            bodies: collect_ids(manifest.body_ids())?,
            shapes: collect_ids(manifest.shape_ids())?,
            joints: collect_ids(manifest.joint_ids())?,
            chains: collect_ids(manifest.chain_ids())?,
            world,
        })
    }
}

impl fmt::Debug for SnapshotLoad {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnapshotLoad")
            .field("bodies", &self.bodies.len())
            .field("shapes", &self.shapes.len())
            .field("joints", &self.joints.len())
            .field("chains", &self.chains.len())
            .finish_non_exhaustive()
    }
}

impl World {
    /// Capture an in-process snapshot capability.
    #[track_caller]
    pub fn snapshot(&self) -> Snapshot {
        self.try_snapshot().expect("world snapshot capture failed")
    }

    /// Capture an in-process snapshot capability.
    pub fn try_snapshot(&self) -> ApiResult<Snapshot> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_snapshot_preconditions()?;
        let mut activity = RestoreActivity::begin(self.core())?;
        let identity = runtime_identity()?;
        let identities = self.core().identity_manifest_while_restoring()?;
        let user_data = self.core().user_data_manifest_while_restoring()?;
        let host_requirements = SnapshotHostRequirements::capture(self.core());

        let native = capture_native(self.raw())?;
        let image = SnapshotImage::from_native(native, identity, host_requirements)?;
        identities.validate_snapshot_entries(&image.validation.entries)?;
        activity.finish()?;

        Ok(Snapshot {
            origin: self.brand(),
            image,
            identities,
            user_data,
            _owner_thread: PhantomData,
        })
    }

    /// Restore a snapshot captured by this exact world instance.
    #[track_caller]
    pub fn restore(&mut self, snapshot: &Snapshot) -> SnapshotRestore {
        self.try_restore(snapshot)
            .expect("world snapshot restore failed")
    }

    /// Restore a snapshot captured by this exact world instance.
    pub fn try_restore(&mut self, snapshot: &Snapshot) -> ApiResult<SnapshotRestore> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_snapshot_preconditions()?;
        if snapshot.origin != self.brand() {
            return Err(ApiError::ForeignSnapshot);
        }
        let current_identity = runtime_identity()?;
        if !same_identity(&snapshot.image.identity, &current_identity) {
            return Err(ApiError::SnapshotAbiMismatch);
        }
        let native_payload = snapshot.image.native_payload();
        let native_payload_len = checked_native_payload_len(native_payload.len())?;
        let mut activity = RestoreActivity::begin(self.core())?;
        let validation = validate_native(native_payload)?;
        snapshot
            .identities
            .validate_snapshot_entries(&validation.entries)?;
        if !snapshot.image.host_requirements.matches(self.core()) {
            return Err(ApiError::SnapshotHostWiringMismatch);
        }
        if !self.core().snapshot_callbacks_satisfy(
            validation.facts.requires_custom_filter != 0,
            validation.facts.requires_pre_solve != 0,
        ) {
            return Err(ApiError::SnapshotCallbacksUnavailable);
        }

        if restore_failpoint_is(RestoreFailpoint::Prepare) {
            return Err(ApiError::SnapshotManifestMismatch);
        }
        restore_allocation_failpoint(RestoreFailpoint::IdentityAllocation)?;
        let prepared_identities = self.core().prepare_identity_restore(&snapshot.identities)?;
        restore_allocation_failpoint(RestoreFailpoint::ReportAllocation)?;
        let report = build_restore_report(&snapshot.identities, &prepared_identities)?;
        restore_allocation_failpoint(RestoreFailpoint::UserDataAllocation)?;
        let prepared_user_data = self.core().prepare_user_data_restore(
            &snapshot.user_data,
            &snapshot.identities,
            &prepared_identities,
        )?;
        restore_allocation_failpoint(RestoreFailpoint::ContactEpochAllocation)?;
        let next_contact_epoch = self.core().prepare_contact_epoch()?;

        record_native_restore_call();
        let native_ok = unsafe {
            ffi::b2World_Restore(self.raw(), native_payload.as_ptr(), native_payload_len)
        };
        if !native_ok || restore_failpoint_is(RestoreFailpoint::Native) {
            activity.disarm();
            terminalize_after_restore(self.core());
            return Err(ApiError::SnapshotNativeFailed);
        }
        let mut host_panic = crate::core::callback_state::PanicSlot::default();
        let host_commit = catch_unwind(AssertUnwindSafe(|| -> ApiResult<()> {
            if restore_failpoint_is(RestoreFailpoint::Commit) {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            self.core().commit_identity_restore(prepared_identities)?;
            if restore_failpoint_is(RestoreFailpoint::AfterIdentityCommit) {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            let committed_user_data = self.core().commit_user_data_restore(prepared_user_data)?;
            if restore_failpoint_is(RestoreFailpoint::AfterUserDataCommit) {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            self.core().commit_contact_epoch(next_contact_epoch)?;
            if restore_failpoint_is(RestoreFailpoint::AfterContactEpochCommit) {
                return Err(ApiError::SnapshotManifestMismatch);
            }

            reattach_user_data(self.raw(), &committed_user_data.attachments);
            self.event_cache().invalidate();
            if let Err(payload) = drop_retired_user_data(committed_user_data.retired) {
                host_panic.capture(payload);
            }
            Ok(())
        }));
        match host_commit {
            Ok(Ok(())) if !host_panic.has_panicked() => {}
            Ok(Ok(())) => {
                activity.disarm();
                terminalize_after_panic(self.core(), host_panic);
                return Err(ApiError::WorldDestroyed);
            }
            Ok(Err(error)) => {
                activity.disarm();
                terminalize_after_restore(self.core());
                return Err(error);
            }
            Err(payload) => {
                activity.disarm();
                host_panic.capture(payload);
                terminalize_after_panic(self.core(), host_panic);
                return Err(ApiError::WorldDestroyed);
            }
        }
        if let Err(error) = activity.finish() {
            activity.disarm();
            terminalize_after_restore(self.core());
            return Err(error);
        }
        Ok(report)
    }

    /// Create a fresh world from a validated external image.
    pub fn from_snapshot_image(
        image: &SnapshotImage,
        worker_count: WorkerCount,
    ) -> ApiResult<SnapshotLoad> {
        crate::core::callback_state::check_not_in_callback()?;
        let current_identity = runtime_identity()?;
        if !same_identity(&image.identity, &current_identity) {
            return Err(ApiError::SnapshotAbiMismatch);
        }
        if !image.host_requirements.is_empty() {
            return Err(ApiError::SnapshotHostWiringMismatch);
        }
        let native_payload = image.native_payload();
        let native_payload_len = checked_native_payload_len(native_payload.len())?;
        let validation = validate_native(native_payload)?;
        if validation.facts.requires_custom_filter != 0 || validation.facts.requires_pre_solve != 0
        {
            return Err(ApiError::SnapshotCallbacksUnavailable);
        }

        let token = WorldToken::allocate()?;
        let foundation_lease = crate::core::foundation::acquire_ordinary_world_lease()?;
        let world_slot_guard = crate::core::foundation::lock_world_slot_mutation();
        let raw = unsafe {
            ffi::b2CreateWorldFromSnapshot(
                native_payload.as_ptr(),
                native_payload_len,
                worker_count.as_i32(),
            )
        };
        if !unsafe { ffi::b2World_IsValid(raw) } {
            drop(world_slot_guard);
            return Err(ApiError::SnapshotNativeFailed);
        }
        let brand = match IdBrand::new(raw, token) {
            Ok(brand) => brand,
            Err(error) => {
                unsafe { ffi::b2DestroyWorld(raw) };
                drop(world_slot_guard);
                return Err(error);
            }
        };
        let core =
            match WorldCore::new_from_snapshot(raw, brand, foundation_lease, &validation.entries) {
                Ok(core) => core,
                Err(error) => {
                    unsafe { ffi::b2DestroyWorld(raw) };
                    drop(world_slot_guard);
                    return Err(error);
                }
            };
        drop(world_slot_guard);
        SnapshotLoad::new(Self::from_restored_core(core))
    }
}

struct ParsedEnvelope {
    identity: AdapterIdentity,
    host_requirements: SnapshotHostRequirements,
    checksum: [u8; 32],
    payload: Range<usize>,
}

fn validate_image(bytes: &[u8]) -> ApiResult<(ParsedEnvelope, SnapshotValidation)> {
    checked_image_len(bytes.len())?;
    let parsed = parse_envelope(bytes)?;
    let payload = bytes
        .get(parsed.payload.clone())
        .ok_or(ApiError::InvalidSnapshotImage)?;
    checked_native_payload_len(payload.len())?;
    if image_checksum(bytes) != parsed.checksum {
        return Err(ApiError::SnapshotChecksumMismatch);
    }
    let current = runtime_identity()?;
    if !same_identity(&parsed.identity, &current) {
        return Err(ApiError::SnapshotAbiMismatch);
    }
    let validation = validate_native(payload)?;
    Ok((parsed, validation))
}

fn encode_image(
    payload: &[u8],
    identity: &AdapterIdentity,
    host_requirements: SnapshotHostRequirements,
) -> ApiResult<Vec<u8>> {
    let total = IMAGE_HEADER_LEN
        .checked_add(payload.len())
        .ok_or(ApiError::SnapshotAllocationFailed)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(total)
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    bytes.extend_from_slice(&IMAGE_MAGIC);
    push_u32(&mut bytes, IMAGE_SCHEMA);
    push_u32(&mut bytes, IMAGE_HEADER_LEN as u32);
    push_u64(
        &mut bytes,
        u64::try_from(payload.len()).map_err(|_| ApiError::SnapshotAllocationFailed)?,
    );
    bytes.extend_from_slice(&[0; 32]);
    push_u32(&mut bytes, host_requirements.0);
    push_u32(&mut bytes, 0);
    encode_identity(&mut bytes, identity);
    debug_assert_eq!(bytes.len(), IMAGE_HEADER_LEN);
    bytes.extend_from_slice(payload);
    let checksum = image_checksum(&bytes);
    bytes[IMAGE_CHECKSUM].copy_from_slice(&checksum);
    Ok(bytes)
}

fn image_checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&bytes[..IMAGE_CHECKSUM.start]);
    hasher.update(&bytes[IMAGE_CHECKSUM.end..]);
    *hasher.finalize().as_bytes()
}

fn parse_envelope(bytes: &[u8]) -> ApiResult<ParsedEnvelope> {
    if bytes.len() < IMAGE_HEADER_LEN {
        return Err(ApiError::InvalidSnapshotImage);
    }
    let mut cursor = Cursor::new(bytes);
    if cursor.take_array::<8>()? != IMAGE_MAGIC
        || cursor.u32()? != IMAGE_SCHEMA
        || cursor.u32()? as usize != IMAGE_HEADER_LEN
    {
        return Err(ApiError::InvalidSnapshotImage);
    }
    let payload_len = usize::try_from(cursor.u64()?).map_err(|_| ApiError::InvalidSnapshotImage)?;
    let checksum = cursor.take_array::<32>()?;
    let host_requirements = SnapshotHostRequirements::parse(cursor.u32()?, cursor.u32()?)?;
    let identity = decode_identity(&mut cursor)?;
    if cursor.position() != IMAGE_HEADER_LEN {
        return Err(ApiError::InvalidSnapshotImage);
    }
    let end = IMAGE_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(ApiError::InvalidSnapshotImage)?;
    if end != bytes.len() {
        return Err(ApiError::InvalidSnapshotImage);
    }
    Ok(ParsedEnvelope {
        identity,
        host_requirements,
        checksum,
        payload: IMAGE_HEADER_LEN..end,
    })
}

fn encode_identity(bytes: &mut Vec<u8>, identity: &AdapterIdentity) {
    push_u32(bytes, identity.struct_size);
    push_u32(bytes, identity.abi_version);
    push_u32(bytes, identity.snapshot_version);
    push_u32(bytes, identity.recording_version_major);
    push_u32(bytes, identity.recording_version_minor);
    push_u32(bytes, identity.snapshot_layout_hash);
    bytes.extend_from_slice(&[
        identity.pointer_width,
        identity.little_endian,
        identity.double_precision,
        identity.validation_enabled,
    ]);
    bytes.extend_from_slice(&identity.private_abi_hash);
    bytes.extend_from_slice(&identity.upstream_sha);
    bytes.extend_from_slice(&identity.target_abi);
    bytes.extend_from_slice(&identity.adapter_source_sha256);
    debug_assert_eq!(bytes.len(), IMAGE_EFFECTIVE_SOURCE_SHA256.start);
    bytes.extend_from_slice(&identity.effective_source_sha256);
    debug_assert_eq!(bytes.len(), IMAGE_EFFECTIVE_SOURCE_SHA256.end);
    bytes.extend_from_slice(&identity.recording_contract_blake3);
}

fn decode_identity(cursor: &mut Cursor<'_>) -> ApiResult<AdapterIdentity> {
    Ok(AdapterIdentity {
        struct_size: cursor.u32()?,
        abi_version: cursor.u32()?,
        snapshot_version: cursor.u32()?,
        recording_version_major: cursor.u32()?,
        recording_version_minor: cursor.u32()?,
        snapshot_layout_hash: cursor.u32()?,
        pointer_width: cursor.u8()?,
        little_endian: cursor.u8()?,
        double_precision: cursor.u8()?,
        validation_enabled: cursor.u8()?,
        private_abi_hash: cursor.take_array()?,
        upstream_sha: cursor.take_array()?,
        target_abi: cursor.take_array()?,
        adapter_source_sha256: cursor.take_array()?,
        effective_source_sha256: cursor.take_array()?,
        recording_contract_blake3: cursor.take_array()?,
    })
}

fn same_identity(left: &AdapterIdentity, right: &AdapterIdentity) -> bool {
    left == right
}

fn runtime_identity() -> ApiResult<AdapterIdentity> {
    boxdd_sys::adapter::verify_runtime_identity().map_err(|_| ApiError::SnapshotAbiMismatch)
}

fn validate_native(payload: &[u8]) -> ApiResult<SnapshotValidation> {
    #[cfg(test)]
    NATIVE_VALIDATION_CALLS.with(|calls| calls.set(calls.get() + 1));
    boxdd_sys::adapter::validate_snapshot(payload, &boxdd_sys::adapter::SnapshotLimits::default())
        .map_err(map_snapshot_validation_error)
}

fn map_snapshot_validation_error(error: boxdd_sys::adapter::SnapshotValidationError) -> ApiError {
    match error {
        boxdd_sys::adapter::SnapshotValidationError::AdapterIdentity(_) => {
            ApiError::SnapshotAbiMismatch
        }
        boxdd_sys::adapter::SnapshotValidationError::Status(status)
            if status == boxdd_sys::adapter::SNAPSHOT_ABI_MISMATCH =>
        {
            ApiError::SnapshotAbiMismatch
        }
        _ => ApiError::InvalidSnapshotImage,
    }
}

fn checked_native_payload_len(length: usize) -> ApiResult<i32> {
    if length > max_native_snapshot_bytes() {
        return Err(ApiError::InvalidSnapshotImage);
    }
    i32::try_from(length).map_err(|_| ApiError::InvalidSnapshotImage)
}

fn checked_image_len(length: usize) -> ApiResult<()> {
    let max = IMAGE_HEADER_LEN
        .checked_add(max_native_snapshot_bytes())
        .ok_or(ApiError::InvalidSnapshotImage)?;
    if length > max {
        return Err(ApiError::InvalidSnapshotImage);
    }
    Ok(())
}

fn max_native_snapshot_bytes() -> usize {
    usize::try_from(boxdd_sys::adapter::SnapshotLimits::default().max_image_bytes)
        .unwrap_or(usize::MAX)
}

fn capture_native(world: ffi::b2WorldId) -> ApiResult<Vec<u8>> {
    let required_i32 = unsafe { ffi::b2World_Snapshot(world, core::ptr::null_mut(), 0) };
    if required_i32 <= 0 {
        return Err(ApiError::SnapshotNativeFailed);
    }
    let required = usize::try_from(required_i32).map_err(|_| ApiError::SnapshotNativeFailed)?;
    checked_native_payload_len(required)?;
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(required)
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    let initialized = unsafe { ffi::b2World_Snapshot(world, payload.as_mut_ptr(), required_i32) };
    if initialized != required_i32 {
        return Err(ApiError::SnapshotNativeFailed);
    }
    // SAFETY: Box2D reported exactly the requested size after initializing the complete prefix.
    unsafe { payload.set_len(required) };
    Ok(payload)
}

fn build_restore_report(
    manifest: &crate::core::identity_registry::IdentityManifest,
    prepared: &crate::core::identity_registry::PreparedIdentityRestore,
) -> ApiResult<SnapshotRestore> {
    let mut bodies = Vec::new();
    let mut shapes = Vec::new();
    let mut joints = Vec::new();
    let mut chains = Vec::new();
    bodies
        .try_reserve_exact(manifest.body_ids().count())
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    shapes
        .try_reserve_exact(manifest.shape_ids().count())
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    joints
        .try_reserve_exact(manifest.joint_ids().count())
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    chains
        .try_reserve_exact(manifest.chain_ids().count())
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    for snapshot_id in manifest.body_ids() {
        bodies.push((
            snapshot_id,
            prepared
                .body_after_restore(manifest, snapshot_id)
                .ok_or(ApiError::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.shape_ids() {
        shapes.push((
            snapshot_id,
            prepared
                .shape_after_restore(manifest, snapshot_id)
                .ok_or(ApiError::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.joint_ids() {
        joints.push((
            snapshot_id,
            prepared
                .joint_after_restore(manifest, snapshot_id)
                .ok_or(ApiError::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.chain_ids() {
        chains.push((
            snapshot_id,
            prepared
                .chain_after_restore(manifest, snapshot_id)
                .ok_or(ApiError::SnapshotManifestMismatch)?,
        ));
    }
    Ok(SnapshotRestore {
        bodies,
        shapes,
        joints,
        chains,
    })
}

fn reattach_user_data(
    world: ffi::b2WorldId,
    attachments: &[crate::core::user_data::UserDataAttachment],
) {
    for attachment in attachments {
        match *attachment {
            crate::core::user_data::UserDataAttachment::World(pointer) => unsafe {
                ffi::b2World_SetUserData(world, pointer)
            },
            crate::core::user_data::UserDataAttachment::Body(id, pointer) => unsafe {
                ffi::b2Body_SetUserData(id.into_raw(), pointer)
            },
            crate::core::user_data::UserDataAttachment::Shape(id, pointer) => unsafe {
                ffi::b2Shape_SetUserData(id.into_raw(), pointer)
            },
            crate::core::user_data::UserDataAttachment::Joint(id, pointer) => unsafe {
                ffi::b2Joint_SetUserData(id.into_raw(), pointer)
            },
        }
    }
}

fn drop_retired_user_data(
    retired: Vec<crate::core::user_data::UserDataEntryRef>,
) -> std::thread::Result<()> {
    let mut panic = crate::core::callback_state::PanicSlot::default();
    for entry in retired {
        panic.run_cleanup(|| {
            let value = entry
                .take_erased()
                .expect("snapshot prepare checked user-data mutability");
            drop(value);
        });
    }
    panic.into_result(())
}

fn terminalize_after_restore(core: &WorldCore) {
    terminalize_after_panic(core, crate::core::callback_state::PanicSlot::default());
}

fn terminalize_after_panic(core: &WorldCore, mut panic: crate::core::callback_state::PanicSlot) {
    panic.run_cleanup(|| {
        core.poison();
        core.shutdown_native();
    });
    panic.resume_or_forget();
}

fn map_id<Id: Copy + PartialEq>(mappings: &[(Id, Id)], snapshot_id: Id) -> Option<Id> {
    mappings
        .iter()
        .find_map(|&(before, after)| (before == snapshot_id).then_some(after))
}

fn collect_ids<Id>(ids: impl ExactSizeIterator<Item = Id>) -> ApiResult<Vec<Id>> {
    let mut collected = Vec::new();
    collected
        .try_reserve_exact(ids.len())
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    collected.extend(ids);
    Ok(collected)
}

struct RestoreActivity<'a> {
    core: &'a WorldCore,
    armed: bool,
}

impl<'a> RestoreActivity<'a> {
    fn begin(core: &'a WorldCore) -> ApiResult<Self> {
        core.set_activity(ActivityState::Idle, ActivityState::Restoring)?;
        Ok(Self { core, armed: true })
    }

    fn finish(&mut self) -> ApiResult<()> {
        self.core.finish_restore_activity()?;
        self.armed = false;
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RestoreActivity<'_> {
    fn drop(&mut self) {
        if self.armed
            && self.core.lifecycle() == LifecycleState::Live
            && self.core.activity() == ActivityState::Restoring
        {
            let _ = self.core.finish_restore_activity();
        }
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn take_array<const N: usize>(&mut self) -> ApiResult<[u8; N]> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(ApiError::InvalidSnapshotImage)?;
        let source = self
            .bytes
            .get(self.position..end)
            .ok_or(ApiError::InvalidSnapshotImage)?;
        self.position = end;
        source
            .try_into()
            .map_err(|_| ApiError::InvalidSnapshotImage)
    }

    fn u8(&mut self) -> ApiResult<u8> {
        Ok(self.take_array::<1>()?[0])
    }

    fn u32(&mut self) -> ApiResult<u32> {
        Ok(u32::from_le_bytes(self.take_array()?))
    }

    fn u64(&mut self) -> ApiResult<u64> {
        Ok(u64::from_le_bytes(self.take_array()?))
    }
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum RestoreFailpoint {
    Prepare,
    IdentityAllocation,
    ReportAllocation,
    UserDataAllocation,
    ContactEpochAllocation,
    Native,
    Commit,
    AfterIdentityCommit,
    AfterUserDataCommit,
    AfterContactEpochCommit,
}

#[cfg(test)]
thread_local! {
    static RESTORE_FAILPOINT: core::cell::Cell<Option<RestoreFailpoint>> = const { core::cell::Cell::new(None) };
    static NATIVE_RESTORE_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static NATIVE_VALIDATION_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

fn restore_failpoint_is(point: RestoreFailpoint) -> bool {
    #[cfg(test)]
    {
        RESTORE_FAILPOINT.with(|current| current.get() == Some(point))
    }
    #[cfg(not(test))]
    {
        let _ = point;
        false
    }
}

fn restore_allocation_failpoint(point: RestoreFailpoint) -> ApiResult<()> {
    if restore_failpoint_is(point) {
        Err(ApiError::SnapshotAllocationFailed)
    } else {
        Ok(())
    }
}

fn record_native_restore_call() {
    #[cfg(test)]
    NATIVE_RESTORE_CALLS.with(|calls| calls.set(calls.get() + 1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyDef, DistanceJointDef, JointBase, ShapeDef, Vec2, WorldDef, shapes};

    fn set_failpoint(point: Option<RestoreFailpoint>) {
        RESTORE_FAILPOINT.with(|current| current.set(point));
    }

    fn reset_native_calls() {
        NATIVE_RESTORE_CALLS.with(|calls| calls.set(0));
    }

    fn native_calls() -> usize {
        NATIVE_RESTORE_CALLS.with(core::cell::Cell::get)
    }

    fn reset_native_validation_calls() {
        NATIVE_VALIDATION_CALLS.with(|calls| calls.set(0));
    }

    fn native_validation_calls() -> usize {
        NATIVE_VALIDATION_CALLS.with(core::cell::Cell::get)
    }

    #[test]
    fn native_payload_length_is_bounded_before_ffi_conversion() {
        let max = max_native_snapshot_bytes();
        let max_envelope = IMAGE_HEADER_LEN.checked_add(max).unwrap();

        assert_eq!(
            checked_native_payload_len(max),
            Ok(i32::try_from(max).unwrap())
        );
        assert_eq!(
            checked_native_payload_len(max + 1),
            Err(ApiError::InvalidSnapshotImage)
        );
        assert_eq!(
            checked_native_payload_len(i32::MAX as usize + 1),
            Err(ApiError::InvalidSnapshotImage)
        );
        assert_eq!(checked_image_len(max_envelope), Ok(()));
        assert_eq!(
            checked_image_len(max_envelope + 1),
            Err(ApiError::InvalidSnapshotImage)
        );
    }

    #[test]
    fn adapter_identity_and_native_status_errors_map_precisely() {
        assert_eq!(
            map_snapshot_validation_error(
                boxdd_sys::adapter::SnapshotValidationError::AdapterIdentity(
                    boxdd_sys::adapter::AdapterIdentityError::Unavailable,
                ),
            ),
            ApiError::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(
                boxdd_sys::adapter::SnapshotValidationError::AdapterIdentity(
                    boxdd_sys::adapter::AdapterIdentityError::Mismatch(
                        boxdd_sys::adapter::AdapterIdentityField::Precision,
                    ),
                ),
            ),
            ApiError::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(boxdd_sys::adapter::SnapshotValidationError::Status(
                boxdd_sys::adapter::SNAPSHOT_ABI_MISMATCH,
            )),
            ApiError::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(boxdd_sys::adapter::SnapshotValidationError::Status(
                boxdd_sys::adapter::SNAPSHOT_BAD_HEADER,
            )),
            ApiError::InvalidSnapshotImage
        );
    }

    #[test]
    fn effective_source_digest_mismatch_rejects_before_native_validation() {
        let world = World::new(WorldDef::default()).unwrap();
        let mut bytes = world.snapshot().image().as_bytes().to_vec();
        bytes[IMAGE_EFFECTIVE_SOURCE_SHA256.start] ^= 1;
        let checksum = image_checksum(&bytes);
        bytes[IMAGE_CHECKSUM].copy_from_slice(&checksum);

        reset_native_validation_calls();
        assert!(matches!(
            SnapshotImage::from_bytes(&bytes),
            Err(ApiError::SnapshotAbiMismatch)
        ));
        assert_eq!(native_validation_calls(), 0);
    }

    #[test]
    fn native_identity_metadata_must_exactly_match_the_host_manifest() {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_SHAPE,
        };

        let mut world = World::new(WorldDef::default()).unwrap();
        let body_a = world.create_body_id(BodyDef::default());
        let body_b = world.create_body_id(BodyDef::default());
        world.create_circle_shape_for(
            body_a,
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5),
        );
        world.create_distance_joint_id(
            &DistanceJointDef::new(JointBase::new(body_a, body_b)).length(1.0),
        );
        world.create_chain_for_id(
            body_a,
            &shapes::chain::ChainDef::builder()
                .points([
                    Vec2::new(-2.0, 0.0),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(2.0, 0.0),
                ])
                .build(),
        );
        let snapshot = world.snapshot();
        let entries = &snapshot.image.validation.entries;
        assert!(
            snapshot
                .identities
                .validate_snapshot_entries(entries)
                .is_ok()
        );

        let mut tampered = entries.clone();
        let body = tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_BODY)
            .unwrap();
        body.generation = body.generation.wrapping_add(1);
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.owner_b >= 0)
            .unwrap()
            .owner_b_order = i32::MAX;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE)
            .unwrap()
            .owner_a = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.owner_b >= 0)
            .unwrap()
            .owner_b = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_JOINT)
            .unwrap()
            .subtype = u32::MAX;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_CHAIN)
            .unwrap()
            .owner_a = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(ApiError::SnapshotManifestMismatch)
        );
    }

    #[test]
    fn prepare_failure_preserves_the_live_world_without_native_restore() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyDef::default());
        let handle = world.handle();
        let snapshot = world.snapshot();
        reset_native_calls();
        set_failpoint(Some(RestoreFailpoint::Prepare));

        assert_eq!(
            world.try_restore(&snapshot).unwrap_err(),
            ApiError::SnapshotManifestMismatch
        );

        set_failpoint(None);
        assert_eq!(native_calls(), 0);
        assert!(handle.try_body_position(body).is_ok());
        assert_eq!(world.core().lifecycle(), LifecycleState::Live);
        assert_eq!(world.core().activity(), ActivityState::Idle);
    }

    #[test]
    fn native_failure_terminalizes_after_exactly_one_restore_call() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyDef::default());
        let handle = world.handle();
        let snapshot = world.snapshot();
        reset_native_calls();
        set_failpoint(Some(RestoreFailpoint::Native));

        assert_eq!(
            world.try_restore(&snapshot).unwrap_err(),
            ApiError::SnapshotNativeFailed
        );

        set_failpoint(None);
        assert_eq!(native_calls(), 1);
        assert_eq!(
            handle.try_body_position(body).unwrap_err(),
            ApiError::WorldDestroyed
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
    }

    #[test]
    fn host_commit_failure_terminalizes_after_exactly_one_restore_call() {
        for point in [
            RestoreFailpoint::Commit,
            RestoreFailpoint::AfterIdentityCommit,
            RestoreFailpoint::AfterUserDataCommit,
            RestoreFailpoint::AfterContactEpochCommit,
        ] {
            let mut world = World::new(WorldDef::default()).unwrap();
            let body = world.create_body_id(BodyDef::default());
            let handle = world.handle();
            let snapshot = world.snapshot();
            reset_native_calls();
            set_failpoint(Some(point));

            assert_eq!(
                world.try_restore(&snapshot).unwrap_err(),
                ApiError::SnapshotManifestMismatch
            );

            set_failpoint(None);
            assert_eq!(native_calls(), 1);
            assert_eq!(
                handle.try_body_position(body).unwrap_err(),
                ApiError::WorldDestroyed
            );
            assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
        }
    }

    #[test]
    fn allocation_failures_preserve_the_live_world_without_native_restore() {
        for point in [
            RestoreFailpoint::IdentityAllocation,
            RestoreFailpoint::ReportAllocation,
            RestoreFailpoint::UserDataAllocation,
            RestoreFailpoint::ContactEpochAllocation,
        ] {
            let mut world = World::new(WorldDef::default()).unwrap();
            let body = world.create_body_id(BodyDef::default());
            let handle = world.handle();
            let snapshot = world.snapshot();
            reset_native_calls();
            set_failpoint(Some(point));

            assert_eq!(
                world.try_restore(&snapshot).unwrap_err(),
                ApiError::SnapshotAllocationFailed
            );

            set_failpoint(None);
            assert_eq!(native_calls(), 0);
            assert!(handle.try_body_position(body).is_ok());
            assert_eq!(world.core().lifecycle(), LifecycleState::Live);
            assert_eq!(world.core().activity(), ActivityState::Idle);
        }
    }

    #[test]
    fn panic_while_retiring_user_data_terminalizes_and_resumes_unwind() {
        struct PanicOnDrop;

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("intentional user-data destructor panic");
            }
        }

        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyDef::default());
        let handle = world.handle();
        let snapshot = world.snapshot();
        world.body(body).unwrap().set_user_data(PanicOnDrop);
        reset_native_calls();

        let panic = catch_unwind(AssertUnwindSafe(|| world.try_restore(&snapshot)));

        assert!(panic.is_err());
        assert_eq!(native_calls(), 1);
        assert_eq!(
            handle.try_body_position(body).unwrap_err(),
            ApiError::WorldDestroyed
        );
        assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn shutdown_panic_during_outer_restore_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_SNAPSHOT_SHUTDOWN";
        const TEST_NAME: &str =
            "snapshot::tests::shutdown_panic_during_outer_restore_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer snapshot unwind remains primary";

        struct PanicOnDrop;

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("secondary snapshot shutdown panic");
            }
        }

        struct InvokeOnDrop(Option<Box<dyn FnOnce()>>);

        impl Drop for InvokeOnDrop {
            fn drop(&mut self) {
                if let Some(invoke) = self.0.take() {
                    invoke();
                }
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut world = World::new(WorldDef::default()).unwrap();
                let snapshot = world.snapshot();
                world.set_user_data(PanicOnDrop);
                set_failpoint(Some(RestoreFailpoint::Native));
                let _restore = InvokeOnDrop(Some(Box::new(move || {
                    assert!(matches!(
                        world.try_restore(&snapshot),
                        Err(ApiError::SnapshotNativeFailed)
                    ));
                })));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            eprintln!("boxdd-outer-unwind-snapshot-shutdown: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind snapshot shutdown child process must start");
        assert!(
            output.status.success(),
            "outer-unwind snapshot shutdown child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-snapshot-shutdown: completed"),
            "outer-unwind snapshot shutdown child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
