//! Transactional Box2D world snapshots.
//!
//! `Snapshot` is an opaque in-process capability tied to its origin world. Native snapshot bytes
//! never cross the Safe Rust API boundary.

use core::fmt;
use core::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use boxdd_sys::adapter::{AdapterIdentity, SnapshotValidation};
use boxdd_sys::ffi;

#[cfg(test)]
use crate::core::world_core::{ActivityState, LifecycleState};
use crate::core::world_core::{CallbackRegistrationGenerations, RestoreActivityLease, WorldCore};
use crate::error::{Error, Result};
use crate::id::{ContactEpoch, IdBrand};
use crate::types::{BodyId, ChainId, JointId, ShapeId};
use crate::world::World;

const REQUIRE_FRICTION_MIXER: u32 = 1 << 0;
const REQUIRE_RESTITUTION_MIXER: u32 = 1 << 1;
const REQUIRE_CUSTOM_FILTER: u32 = 1 << 2;
const REQUIRE_PRE_SOLVE: u32 = 1 << 3;

/// An unforgeable, process-local snapshot capability.
///
/// This type has no public constructor and is deliberately not serializable. Only a snapshot
/// taken from the same live `World` can authorize in-place restore. Every host callback captured
/// by the snapshot must still be the same registration; clearing or replacing one invalidates
/// that snapshot for in-place restore.
pub struct Snapshot {
    origin: IdBrand,
    native_payload: Box<[u8]>,
    identity: AdapterIdentity,
    validation: SnapshotValidation,
    host_requirements: SnapshotHostRequirements,
    identities: crate::core::identity_registry::IdentityManifest,
    user_data: crate::core::user_data::UserDataManifest,
    mixer_identities: crate::MixerIdentities,
    callback_registration_generations: CallbackRegistrationGenerations,
    _owner_thread: PhantomData<Rc<()>>,
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("origin", &"World(..)")
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
    /// Iterates over every body ID mapping from the captured snapshot to the restored world.
    pub fn body_mappings(&self) -> impl ExactSizeIterator<Item = (BodyId, BodyId)> + '_ {
        self.bodies.iter().copied()
    }

    /// Iterates over every shape ID mapping from the captured snapshot to the restored world.
    pub fn shape_mappings(&self) -> impl ExactSizeIterator<Item = (ShapeId, ShapeId)> + '_ {
        self.shapes.iter().copied()
    }

    /// Iterates over every joint ID mapping from the captured snapshot to the restored world.
    pub fn joint_mappings(&self) -> impl ExactSizeIterator<Item = (JointId, JointId)> + '_ {
        self.joints.iter().copied()
    }

    /// Iterates over every chain ID mapping from the captured snapshot to the restored world.
    pub fn chain_mappings(&self) -> impl ExactSizeIterator<Item = (ChainId, ChainId)> + '_ {
        self.chains.iter().copied()
    }

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

/// A fully validated in-place restore which has not crossed the native commit boundary.
///
/// The mapping report is final and may be used to allocate and validate host-side state before
/// [`Self::commit`] invokes Box2D. Dropping this value cancels the restore without changing the
/// world. Once commit starts, any native or host-commit failure terminalizes the world because
/// Box2D cannot roll a partially restored world back.
#[must_use = "dropping a prepared restore cancels it before the native commit boundary"]
pub struct PreparedSnapshotRestore<'world, 'snapshot> {
    world: &'world mut World,
    activity: RestoreActivityLease,
    native_payload: &'snapshot [u8],
    native_payload_len: i32,
    identities: crate::core::identity_registry::PreparedIdentityRestore,
    user_data: crate::core::user_data::PreparedUserDataRestore,
    next_contact_epoch: ContactEpoch,
    report: SnapshotRestore,
}

impl PreparedSnapshotRestore<'_, '_> {
    /// Returns the final process-local ID mappings without mutating the world.
    pub fn mappings(&self) -> &SnapshotRestore {
        &self.report
    }

    /// Commits the native and Rust restore state.
    pub fn commit(self) -> Result<SnapshotRestore> {
        self.commit_with(|_| Ok(()))
    }

    /// Commits the restore and runs one fallible host-state commit before publishing success.
    ///
    /// Callers must perform all validation and allocation before invoking this method. A panic in
    /// `host_commit` or retired user-data cleanup is caught long enough to terminalize the native
    /// world. It resumes at an ordinary Rust call boundary; if the thread is already unwinding, the
    /// secondary panic is suppressed and this method returns [`Error::SnapshotCommitPanicked`] so
    /// the original panic remains primary. A returned error also terminalizes the world because the
    /// native restore has already committed.
    pub fn commit_with(
        self,
        host_commit: impl FnOnce(&SnapshotRestore) -> Result<()>,
    ) -> Result<SnapshotRestore> {
        let host_commit = crate::core::callback_state::PendingUserValue::new(host_commit);
        crate::core::callback_state::check_not_in_callback()?;
        let Self {
            world,
            mut activity,
            native_payload,
            native_payload_len,
            identities,
            user_data,
            next_contact_epoch,
            report,
        } = self;

        record_native_restore_call();
        let native_ok = unsafe {
            ffi::b2World_Restore(world.raw(), native_payload.as_ptr(), native_payload_len)
        };
        if !native_ok || restore_failpoint_is(RestoreFailpoint::Native) {
            activity.disarm();
            terminalize_after_restore(world.core());
            return Err(Error::SnapshotNativeFailed);
        }

        let mut host_panic = crate::core::callback_state::PanicSlot::default();
        let rust_commit = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
            if restore_failpoint_is(RestoreFailpoint::Commit) {
                return Err(Error::SnapshotManifestMismatch);
            }
            world.core().commit_identity_restore(identities)?;
            if restore_failpoint_is(RestoreFailpoint::AfterIdentityCommit) {
                return Err(Error::SnapshotManifestMismatch);
            }
            let mut committed_user_data = world.core().commit_user_data_restore(user_data)?;
            if restore_failpoint_is(RestoreFailpoint::AfterUserDataCommit) {
                return Err(Error::SnapshotManifestMismatch);
            }
            world.core().commit_contact_epoch(next_contact_epoch)?;
            if restore_failpoint_is(RestoreFailpoint::AfterContactEpochCommit) {
                return Err(Error::SnapshotManifestMismatch);
            }

            reattach_user_data(world.raw(), committed_user_data.attachments());
            world.invalidate_completed_step();
            if let Err(payload) = committed_user_data.drop_retired() {
                host_panic.capture(payload);
            }
            if !host_panic.has_panicked() {
                let host_commit = host_commit.into_inner();
                host_commit(&report)?;
            }
            Ok(())
        }));
        match rust_commit {
            Ok(Ok(())) if !host_panic.has_panicked() => {}
            Ok(Ok(())) => {
                activity.disarm();
                terminalize_after_panic(world.core(), host_panic);
                return Err(Error::SnapshotCommitPanicked);
            }
            Ok(Err(error)) => {
                activity.disarm();
                terminalize_after_restore(world.core());
                return Err(error);
            }
            Err(payload) => {
                activity.disarm();
                host_panic.capture(payload);
                terminalize_after_panic(world.core(), host_panic);
                return Err(Error::SnapshotCommitPanicked);
            }
        }
        if let Err(error) = activity.finish() {
            activity.disarm();
            terminalize_after_restore(world.core());
            return Err(error);
        }
        Ok(report)
    }
}

impl fmt::Debug for PreparedSnapshotRestore<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSnapshotRestore")
            .field("mappings", &self.report)
            .finish_non_exhaustive()
    }
}

impl World {
    /// Capture an in-process snapshot capability.
    pub fn snapshot(&self) -> Result<Snapshot> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::world::check_world_available(self)?;
        let mut activity = self.core().begin_restore_activity()?;
        let identity = runtime_identity()?;
        let identities = self.core().identity_manifest_while_restoring()?;
        let user_data = self.core().user_data_manifest_while_restoring()?;
        let host_requirements = SnapshotHostRequirements::capture(self.core());
        let mixer_identities = self.core().mixer_identities();
        let callback_registration_generations = self.core().callback_registration_generations();

        let native_payload = capture_native(self.raw())?.into_boxed_slice();
        let authorization = NativeValidationAuthorization::restoring(&identity, &activity);
        let validation = validate_native(&native_payload, &authorization)?;
        identities.validate_snapshot_entries(&validation.entries)?;
        activity.finish()?;

        Ok(Snapshot {
            origin: self.brand(),
            native_payload,
            identity,
            validation,
            host_requirements,
            identities,
            user_data,
            mixer_identities,
            callback_registration_generations,
            _owner_thread: PhantomData,
        })
    }

    /// Restore a snapshot captured by this exact world instance.
    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<SnapshotRestore> {
        self.prepare_restore(snapshot)?.commit()
    }

    /// Validates and allocates every Rust-side restore artifact before native mutation.
    pub fn prepare_restore<'world, 'snapshot>(
        &'world mut self,
        snapshot: &'snapshot Snapshot,
    ) -> Result<PreparedSnapshotRestore<'world, 'snapshot>> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::world::check_world_available(self)?;
        if snapshot.origin != self.brand() {
            return Err(Error::ForeignSnapshot);
        }
        let current_identity = runtime_identity()?;
        if snapshot.identity != current_identity {
            return Err(Error::SnapshotAbiMismatch);
        }
        let native_payload = &snapshot.native_payload;
        let native_payload_len = checked_native_payload_len(native_payload.len())?;
        let activity = self.core().begin_restore_activity()?;
        let validation = &snapshot.validation;
        snapshot
            .identities
            .validate_snapshot_entries(&validation.entries)?;
        if !snapshot.host_requirements.matches(self.core())
            || snapshot.mixer_identities != self.core().mixer_identities()
            || !snapshot
                .callback_registration_generations
                .matches(&self.core().callback_registration_generations())
        {
            return Err(Error::SnapshotHostWiringMismatch);
        }
        if !self.core().snapshot_callbacks_satisfy(
            validation.facts.requires_custom_filter != 0,
            validation.facts.requires_pre_solve != 0,
        ) {
            return Err(Error::SnapshotCallbacksUnavailable);
        }

        if restore_failpoint_is(RestoreFailpoint::Prepare) {
            return Err(Error::SnapshotManifestMismatch);
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

        Ok(PreparedSnapshotRestore {
            world: self,
            activity,
            native_payload,
            native_payload_len,
            identities: prepared_identities,
            user_data: prepared_user_data,
            next_contact_epoch,
            report,
        })
    }
}

/// A private proof that native snapshot validation is covered by foundation activity.
struct NativeValidationAuthorization<'a> {
    _current_identity: &'a AdapterIdentity,
    _lease: PhantomData<&'a ()>,
}

impl<'a> NativeValidationAuthorization<'a> {
    fn restoring(
        current_identity: &'a AdapterIdentity,
        activity: &'a RestoreActivityLease,
    ) -> Self {
        debug_assert!(activity.is_armed());
        Self {
            _current_identity: current_identity,
            _lease: PhantomData,
        }
    }
}

fn runtime_identity() -> Result<AdapterIdentity> {
    boxdd_sys::adapter::verify_runtime_identity().map_err(|_| Error::SnapshotAbiMismatch)
}

fn validate_native(
    payload: &[u8],
    _authorization: &NativeValidationAuthorization<'_>,
) -> Result<SnapshotValidation> {
    #[cfg(test)]
    NATIVE_VALIDATION_CALLS.with(|calls| calls.set(calls.get() + 1));
    boxdd_sys::adapter::validate_snapshot(payload, &boxdd_sys::adapter::SnapshotLimits::default())
        .map_err(map_snapshot_validation_error)
}

fn map_snapshot_validation_error(error: boxdd_sys::adapter::SnapshotValidationError) -> Error {
    match error {
        boxdd_sys::adapter::SnapshotValidationError::AdapterIdentity(_) => {
            Error::SnapshotAbiMismatch
        }
        boxdd_sys::adapter::SnapshotValidationError::Status(status)
            if status == boxdd_sys::adapter::SNAPSHOT_ABI_MISMATCH =>
        {
            Error::SnapshotAbiMismatch
        }
        _ => Error::InvalidNativeSnapshot,
    }
}

fn checked_native_payload_len(length: usize) -> Result<i32> {
    if length > max_native_snapshot_bytes() {
        return Err(Error::InvalidNativeSnapshot);
    }
    i32::try_from(length).map_err(|_| Error::InvalidNativeSnapshot)
}

fn max_native_snapshot_bytes() -> usize {
    usize::try_from(boxdd_sys::adapter::SnapshotLimits::default().max_image_bytes)
        .unwrap_or(usize::MAX)
}

fn capture_native(world: ffi::b2WorldId) -> Result<Vec<u8>> {
    // SAFETY: `b2World_Snapshot` initializes the reported prefix when the supplied capacity is
    // sufficient, and the helper publishes bytes only when the fill count matches that capacity.
    unsafe {
        capture_native_with(|output, capacity| ffi::b2World_Snapshot(world, output, capacity))
    }
}

/// # Safety
///
/// When `snapshot` returns the supplied positive capacity, it must have initialized exactly that
/// many bytes at `output` without writing beyond the supplied capacity.
unsafe fn capture_native_with(mut snapshot: impl FnMut(*mut u8, i32) -> i32) -> Result<Vec<u8>> {
    let required_i32 = snapshot(core::ptr::null_mut(), 0);
    if required_i32 <= 0 {
        return Err(Error::SnapshotNativeFailed);
    }
    let required = usize::try_from(required_i32).map_err(|_| Error::SnapshotNativeFailed)?;
    checked_native_payload_len(required)?;
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(required)
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    let output = payload.spare_capacity_mut().as_mut_ptr().cast::<u8>();
    let initialized = snapshot(output, required_i32);
    if initialized != required_i32 {
        return Err(Error::SnapshotNativeFailed);
    }
    // SAFETY: Box2D reported exactly the requested size after initializing the complete prefix.
    unsafe { payload.set_len(required) };
    Ok(payload)
}

fn build_restore_report(
    manifest: &crate::core::identity_registry::IdentityManifest,
    prepared: &crate::core::identity_registry::PreparedIdentityRestore,
) -> Result<SnapshotRestore> {
    let mut bodies = Vec::new();
    let mut shapes = Vec::new();
    let mut joints = Vec::new();
    let mut chains = Vec::new();
    bodies
        .try_reserve_exact(manifest.body_ids().count())
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    shapes
        .try_reserve_exact(manifest.shape_ids().count())
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    joints
        .try_reserve_exact(manifest.joint_ids().count())
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    chains
        .try_reserve_exact(manifest.chain_ids().count())
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    for snapshot_id in manifest.body_ids() {
        bodies.push((
            snapshot_id,
            prepared
                .body_after_restore(manifest, snapshot_id)
                .ok_or(Error::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.shape_ids() {
        shapes.push((
            snapshot_id,
            prepared
                .shape_after_restore(manifest, snapshot_id)
                .ok_or(Error::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.joint_ids() {
        joints.push((
            snapshot_id,
            prepared
                .joint_after_restore(manifest, snapshot_id)
                .ok_or(Error::SnapshotManifestMismatch)?,
        ));
    }
    for snapshot_id in manifest.chain_ids() {
        chains.push((
            snapshot_id,
            prepared
                .chain_after_restore(manifest, snapshot_id)
                .ok_or(Error::SnapshotManifestMismatch)?,
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

fn restore_allocation_failpoint(point: RestoreFailpoint) -> Result<()> {
    if restore_failpoint_is(point) {
        Err(Error::SnapshotAllocationFailed)
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
    use crate::{DistanceJointDef, ShapeDef, Vec2, shapes};

    #[cfg(not(target_arch = "wasm32"))]
    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    #[cfg(not(target_arch = "wasm32"))]
    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

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

        assert_eq!(
            checked_native_payload_len(max),
            Ok(i32::try_from(max).unwrap())
        );
        assert_eq!(
            checked_native_payload_len(max + 1),
            Err(Error::InvalidNativeSnapshot)
        );
        assert_eq!(
            checked_native_payload_len(i32::MAX as usize + 1),
            Err(Error::InvalidNativeSnapshot)
        );
    }

    #[test]
    fn native_payload_growth_between_query_and_fill_is_rejected() {
        let mut calls = 0;
        // SAFETY: The injected fill never reports the supplied capacity, so the helper cannot
        // publish or read the deliberately uninitialized output buffer.
        let error = unsafe {
            capture_native_with(|output, capacity| {
                calls += 1;
                match calls {
                    1 => {
                        assert!(output.is_null());
                        assert_eq!(capacity, 0);
                        4
                    }
                    2 => {
                        assert!(!output.is_null());
                        assert_eq!(capacity, 4);
                        5
                    }
                    _ => panic!("snapshot capture must call native exactly twice"),
                }
            })
        }
        .unwrap_err();

        assert_eq!(error, Error::SnapshotNativeFailed);
        assert_eq!(calls, 2);
    }

    #[test]
    fn prepared_restore_rejects_callback_before_native_commit_and_releases_world() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        let prepared = world.prepare_restore(&snapshot).unwrap();
        reset_native_calls();

        let callback_guard = crate::core::callback_state::CallbackGuard::enter();
        assert!(matches!(prepared.commit(), Err(Error::InCallback)));
        assert_eq!(native_calls(), 0);
        drop(callback_guard);

        assert!(
            world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def()
                )
                .is_ok()
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
            Error::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(
                boxdd_sys::adapter::SnapshotValidationError::AdapterIdentity(
                    boxdd_sys::adapter::AdapterIdentityError::Mismatch(
                        boxdd_sys::adapter::AdapterIdentityField::Precision,
                    ),
                ),
            ),
            Error::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(boxdd_sys::adapter::SnapshotValidationError::Status(
                boxdd_sys::adapter::SNAPSHOT_ABI_MISMATCH,
            )),
            Error::SnapshotAbiMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(boxdd_sys::adapter::SnapshotValidationError::Status(
                boxdd_sys::adapter::SNAPSHOT_BAD_HEADER,
            )),
            Error::InvalidNativeSnapshot
        );
    }

    #[test]
    fn native_identity_metadata_must_exactly_match_the_host_manifest() {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_SHAPE,
        };

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        {
            let mut body = world.body(body_a).unwrap();
            body.create_circle(
                &ShapeDef::default(),
                &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
            )
            .unwrap();
            body.create_chain(
                &shapes::chain::ChainDef::builder()
                    .points([
                        Vec2::new(-2.0, 0.0),
                        Vec2::new(-1.0, 0.0),
                        Vec2::new(1.0, 0.0),
                        Vec2::new(2.0, 0.0),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        }
        world
            .create_distance_joint(
                &DistanceJointDef::new(world.joint_base(body_a, body_b).unwrap()).length(1.0),
            )
            .unwrap();
        reset_native_validation_calls();
        let snapshot = world.snapshot().unwrap();
        assert_eq!(native_validation_calls(), 1);
        let entries = &snapshot.validation.entries;
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
            Err(Error::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.owner_b >= 0)
            .unwrap()
            .owner_b_order = i32::MAX;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(Error::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE)
            .unwrap()
            .owner_a = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(Error::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.owner_b >= 0)
            .unwrap()
            .owner_b = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(Error::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_JOINT)
            .unwrap()
            .subtype = u32::MAX;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(Error::SnapshotManifestMismatch)
        );

        let mut tampered = entries.clone();
        tampered
            .iter_mut()
            .find(|entry| entry.kind == SNAPSHOT_ENTRY_CHAIN)
            .unwrap()
            .owner_a = -1;
        assert_eq!(
            snapshot.identities.validate_snapshot_entries(&tampered),
            Err(Error::SnapshotManifestMismatch)
        );
    }

    #[test]
    fn prepare_failure_preserves_the_live_world_without_native_restore() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        reset_native_calls();
        set_failpoint(Some(RestoreFailpoint::Prepare));

        assert_eq!(
            world.restore(&snapshot).unwrap_err(),
            Error::SnapshotManifestMismatch
        );

        set_failpoint(None);
        assert_eq!(native_calls(), 0);
        assert!(world.body(body).and_then(|body| body.position()).is_ok());
        assert_eq!(world.core().lifecycle(), LifecycleState::Live);
        assert_eq!(world.core().activity(), ActivityState::Idle);
    }

    #[test]
    fn native_failure_terminalizes_after_exactly_one_restore_call() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        reset_native_calls();
        set_failpoint(Some(RestoreFailpoint::Native));

        assert_eq!(
            world.restore(&snapshot).unwrap_err(),
            Error::SnapshotNativeFailed
        );

        set_failpoint(None);
        assert_eq!(native_calls(), 1);
        assert!(matches!(world.body(body), Err(Error::WorldDestroyed)));
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
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def(),
                )
                .unwrap();
            let snapshot = world.snapshot().unwrap();
            reset_native_calls();
            set_failpoint(Some(point));

            assert_eq!(
                world.restore(&snapshot).unwrap_err(),
                Error::SnapshotManifestMismatch
            );

            set_failpoint(None);
            assert_eq!(native_calls(), 1);
            assert!(matches!(world.body(body), Err(Error::WorldDestroyed)));
            assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
        }
    }

    #[test]
    fn fallible_host_commit_error_terminalizes_after_native_restore() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        reset_native_calls();

        let prepared = world.prepare_restore(&snapshot).unwrap();
        assert_eq!(
            prepared
                .commit_with(|_| Err(Error::SnapshotManifestMismatch))
                .unwrap_err(),
            Error::SnapshotManifestMismatch
        );

        assert_eq!(native_calls(), 1);
        assert!(matches!(world.body(body), Err(Error::WorldDestroyed)));
        assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
    }

    #[test]
    fn allocation_failures_preserve_the_live_world_without_native_restore() {
        for point in [
            RestoreFailpoint::IdentityAllocation,
            RestoreFailpoint::ReportAllocation,
            RestoreFailpoint::UserDataAllocation,
            RestoreFailpoint::ContactEpochAllocation,
        ] {
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def(),
                )
                .unwrap();
            let snapshot = world.snapshot().unwrap();
            reset_native_calls();
            set_failpoint(Some(point));

            assert_eq!(
                world.restore(&snapshot).unwrap_err(),
                Error::SnapshotAllocationFailed
            );

            set_failpoint(None);
            assert_eq!(native_calls(), 0);
            assert!(world.body(body).and_then(|body| body.position()).is_ok());
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

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        world
            .body(body)
            .unwrap()
            .set_user_data(PanicOnDrop)
            .unwrap();
        reset_native_calls();

        let panic = catch_unwind(AssertUnwindSafe(|| world.restore(&snapshot)));

        assert!(panic.is_err());
        assert_eq!(native_calls(), 1);
        assert!(matches!(world.body(body), Err(Error::WorldDestroyed)));
        assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn multiple_retired_user_data_panics_after_native_commit_do_not_abort() {
        const CHILD: &str = "BOXDD_SNAPSHOT_MULTI_RETIRED_PANIC";
        const TEST_NAME: &str =
            "snapshot::tests::multiple_retired_user_data_panics_after_native_commit_do_not_abort";

        struct PanicOnDrop(std::sync::Arc<std::sync::atomic::AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                panic!("intentional retired user-data panic");
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let mut world = crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            let body_a = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def(),
                )
                .unwrap();
            let body_b = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def(),
                )
                .unwrap();
            let snapshot = world.snapshot().unwrap();
            world
                .body(body_a)
                .unwrap()
                .set_user_data(PanicOnDrop(std::sync::Arc::clone(&dropped)))
                .unwrap();
            world
                .body(body_b)
                .unwrap()
                .set_user_data(PanicOnDrop(std::sync::Arc::clone(&dropped)))
                .unwrap();
            set_failpoint(Some(RestoreFailpoint::AfterUserDataCommit));

            let panic = catch_unwind(AssertUnwindSafe(|| world.restore(&snapshot)));

            assert!(panic.is_err());
            assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 2);
            assert_eq!(world.core().lifecycle(), LifecycleState::Destroyed);
            eprintln!("boxdd-snapshot-multi-retired-panic: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("snapshot multi-panic child process must start");
        assert!(
            output.status.success(),
            "snapshot multi-panic child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-snapshot-multi-retired-panic: completed")
        );
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

        if std::env::var_os(CHILD).is_some() {
            let restore_result = std::rc::Rc::new(core::cell::Cell::new(None));
            let restore_result_from_drop = std::rc::Rc::clone(&restore_result);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut world = crate::Foundation::initialize_default()
                    .unwrap()
                    .create_world(
                        crate::Foundation::get()
                            .expect("Foundation must be initialized before constructing a WorldDef")
                            .world_def(),
                    )
                    .unwrap();
                let snapshot = world.snapshot().unwrap();
                world.set_user_data(PanicOnDrop).unwrap();
                set_failpoint(Some(RestoreFailpoint::Native));
                let _restore = InvokeOnDrop(Some(move || {
                    restore_result_from_drop.set(Some(world.restore(&snapshot).map(|_| ())));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert_eq!(restore_result.get(), Some(Err(Error::SnapshotNativeFailed)));
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn host_commit_panic_during_outer_unwind_returns_typed_fallback_without_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_SNAPSHOT_HOST_COMMIT";
        const TEST_NAME: &str = "snapshot::tests::host_commit_panic_during_outer_unwind_returns_typed_fallback_without_abort";
        const PRIMARY_PANIC: &str = "outer snapshot host-commit unwind remains primary";

        if std::env::var_os(CHILD).is_some() {
            let restore_result = std::rc::Rc::new(core::cell::Cell::new(None));
            let restore_result_from_drop = std::rc::Rc::clone(&restore_result);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut world = crate::Foundation::initialize_default()
                    .unwrap()
                    .create_world(
                        crate::Foundation::get()
                            .expect("Foundation must be initialized before constructing a WorldDef")
                            .world_def(),
                    )
                    .unwrap();
                let snapshot = world.snapshot().unwrap();
                let _restore = InvokeOnDrop(Some(move || {
                    let prepared = world.prepare_restore(&snapshot).unwrap();
                    restore_result_from_drop.set(Some(
                        prepared
                            .commit_with(|_| -> Result<()> {
                                panic!("secondary snapshot host-commit panic")
                            })
                            .map(|_| ()),
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert_eq!(
                restore_result.get(),
                Some(Err(Error::SnapshotCommitPanicked))
            );
            eprintln!("boxdd-outer-unwind-snapshot-host-commit: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind snapshot host-commit child process must start");
        assert!(
            output.status.success(),
            "outer-unwind snapshot host-commit child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-snapshot-host-commit: completed"),
            "outer-unwind snapshot host-commit child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_host_commit_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_SNAPSHOT_HOST_COMMIT";
        const TEST_NAME: &str =
            "snapshot::tests::rejected_host_commit_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer rejected snapshot host-commit unwind remains primary";

        struct PanicOnDrop(std::sync::Arc<std::sync::atomic::AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                if self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    panic!("secondary rejected snapshot host-commit cleanup panic");
                }
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let rejected = std::rc::Rc::new(core::cell::Cell::new(false));
            let rejected_from_drop = std::rc::Rc::clone(&rejected);
            let dropped_from_drop = std::sync::Arc::clone(&dropped);
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut world = crate::Foundation::initialize_default()
                    .unwrap()
                    .create_world(
                        crate::Foundation::get()
                            .expect("Foundation must be initialized before constructing a WorldDef")
                            .world_def(),
                    )
                    .unwrap();
                let snapshot = world.snapshot().unwrap();
                let prepared = world.prepare_restore(&snapshot).unwrap();
                let _restore = InvokeOnDrop(Some(move || {
                    let _callback = crate::core::callback_state::CallbackGuard::enter();
                    let marker = PanicOnDrop(dropped_from_drop);
                    rejected_from_drop.set(matches!(
                        prepared.commit_with(move |_| {
                            let _ = &marker;
                            Ok(())
                        }),
                        Err(Error::InCallback)
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 1);
            eprintln!("boxdd-outer-unwind-rejected-snapshot-host-commit: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected snapshot host-commit child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected snapshot host-commit child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-snapshot-host-commit: completed"),
            "outer-unwind rejected snapshot host-commit child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
