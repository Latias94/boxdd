use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::types::{BodyId, JointId, ShapeId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct UserDataVersion(u64);

pub(crate) struct ErasedUserData {
    type_id: TypeId,
    ptr: NonNull<u8>,
    drop_fn: unsafe fn(*mut c_void),
}

impl ErasedUserData {
    #[inline]
    pub(crate) fn new<T: 'static>(value: T) -> Self {
        unsafe fn drop_boxed<T: 'static>(p: *mut c_void) {
            drop(unsafe { Box::from_raw(p as *mut T) });
        }

        let boxed = Box::new(value);
        let ptr =
            NonNull::new(Box::into_raw(boxed) as *mut u8).expect("Box::into_raw returned null");
        Self {
            type_id: TypeId::of::<T>(),
            ptr,
            drop_fn: drop_boxed::<T>,
        }
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr() as *mut c_void
    }

    #[inline]
    pub(crate) fn with_ref<T: 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        if self.type_id != TypeId::of::<T>() {
            return None;
        }
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        let r = unsafe { &*(self.ptr.as_ptr() as *const T) };
        Some(f(r))
    }

    #[inline]
    pub(crate) fn matches<T: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    #[inline]
    pub(crate) fn with_mut<T: 'static, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        if self.type_id != TypeId::of::<T>() {
            return None;
        }
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        let r = unsafe { &mut *(self.ptr.as_ptr() as *mut T) };
        Some(f(r))
    }

    pub(crate) fn try_into_value<T: 'static>(self) -> std::result::Result<T, Self> {
        if self.type_id != TypeId::of::<T>() {
            return Err(self);
        }
        let p = self.ptr;
        core::mem::forget(self);
        // SAFETY: `type_id` guarantees the allocation is a `T`.
        Ok(*unsafe { Box::from_raw(p.as_ptr() as *mut T) })
    }
}

impl Drop for ErasedUserData {
    fn drop(&mut self) {
        unsafe { (self.drop_fn)(self.ptr.as_ptr() as *mut c_void) }
    }
}

#[must_use = "user-data updates must install the native pointer and retire the previous value"]
pub(crate) struct UserDataUpdate {
    pointer: *mut c_void,
    retired: RetiredUserData,
}

impl UserDataUpdate {
    #[inline]
    pub(crate) fn inserted(pointer: *mut c_void) -> Self {
        Self {
            pointer,
            retired: RetiredUserData::default(),
        }
    }

    #[inline]
    pub(crate) fn into_parts(self) -> (*mut c_void, RetiredUserData) {
        (self.pointer, self.retired)
    }
}

#[derive(Default)]
#[must_use = "retired user data must be released after updating the native pointer"]
pub(crate) struct RetiredUserData(Option<ErasedUserData>);

impl RetiredUserData {
    #[inline]
    pub(crate) fn new(value: Option<ErasedUserData>) -> Self {
        Self(value)
    }

    #[inline]
    pub(crate) fn is_some(&self) -> bool {
        self.0.is_some()
    }

    #[inline]
    pub(crate) fn into_erased(mut self) -> Option<ErasedUserData> {
        self.0.take()
    }

    fn drain_panic(&mut self, panic: &mut crate::core::callback_state::PanicSlot) {
        if let Some(value) = self.0.take() {
            panic.run_cleanup(|| drop(value));
        }
    }

    pub(crate) fn resume_drop_panic(mut self) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        self.drain_panic(&mut panic);
        panic.resume_or_forget();
    }
}

impl Drop for RetiredUserData {
    fn drop(&mut self) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        self.drain_panic(&mut panic);
        panic.resume_or_forget();
    }
}

pub(crate) struct UserDataEntry {
    value: RefCell<Option<ErasedUserData>>,
    version: Cell<UserDataVersion>,
}

pub(crate) type UserDataEntryRef = Rc<UserDataEntry>;

impl UserDataEntry {
    pub(crate) fn new<T: 'static>(
        value: T,
        version: UserDataVersion,
    ) -> (UserDataEntryRef, *mut c_void) {
        let value = ErasedUserData::new(value);
        let pointer = value.as_ptr();
        (
            Rc::new(Self {
                value: RefCell::new(Some(value)),
                version: Cell::new(version),
            }),
            pointer,
        )
    }

    pub(crate) fn replace<T: 'static>(
        &self,
        value: crate::core::callback_state::PendingUserValue<T>,
        version: UserDataVersion,
    ) -> Result<UserDataUpdate> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let value = ErasedUserData::new(value.into_inner());
        let pointer = value.as_ptr();
        let retired = slot.replace(value);
        drop(slot);
        self.version.set(version);
        Ok(UserDataUpdate {
            pointer,
            retired: RetiredUserData::new(retired),
        })
    }

    pub(crate) fn version_if_present(&self) -> Result<Option<UserDataVersion>> {
        let slot = self
            .value
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?;
        Ok(slot.as_ref().map(|_| self.version.get()))
    }

    pub(crate) fn pointer_if_version(
        &self,
        version: UserDataVersion,
    ) -> Result<Option<*mut c_void>> {
        let slot = self
            .value
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?;
        Ok(match slot.as_ref() {
            Some(value) if self.version.get() == version => Some(value.as_ptr()),
            _ => None,
        })
    }

    pub(crate) fn check_mutable(&self) -> Result<()> {
        let borrow = self
            .value
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        drop(borrow);
        Ok(())
    }

    pub(crate) fn take_erased(&self) -> Result<Option<ErasedUserData>> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        Ok(slot.take())
    }

    pub(crate) fn try_with<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        let slot = self
            .value
            .try_borrow()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(value) = slot.as_ref() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(Error::UserDataTypeMismatch);
        }
        Ok(Some(value.with_ref(f.into_inner()).expect("type checked")))
    }

    pub(crate) fn try_with_mut<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(value) = slot.as_mut() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(Error::UserDataTypeMismatch);
        }
        Ok(Some(value.with_mut(f.into_inner()).expect("type checked")))
    }

    pub(crate) fn take<T: 'static>(&self) -> Result<Option<T>> {
        let mut slot = self
            .value
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantAccess)?;
        let Some(value) = slot.as_ref() else {
            return Ok(None);
        };
        if !value.matches::<T>() {
            return Err(Error::UserDataTypeMismatch);
        }
        let value = slot.take().expect("value checked");
        drop(slot);
        match value.try_into_value::<T>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => unreachable!("type checked"),
        }
    }
}

#[derive(Default)]
pub(crate) struct UserDataStore {
    pub(crate) world: Option<UserDataEntryRef>,
    pub(crate) bodies: HashMap<BodyId, UserDataEntryRef>,
    pub(crate) shapes: HashMap<ShapeId, UserDataEntryRef>,
    pub(crate) joints: HashMap<JointId, UserDataEntryRef>,
    last_version: u64,
    revision: u128,
}

impl UserDataStore {
    pub(crate) fn next_version(&mut self) -> Result<UserDataVersion> {
        self.last_version = self
            .last_version
            .checked_add(1)
            .ok_or(Error::UserDataVersionExhausted)?;
        self.revision = self.revision.wrapping_add(1);
        Ok(UserDataVersion(self.last_version))
    }

    pub(crate) fn mark_changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn snapshot_manifest(&self) -> Result<UserDataManifest> {
        let world = match self.world.as_ref() {
            Some(entry) => entry.version_if_present()?,
            None => None,
        };
        let bodies = snapshot_entries(&self.bodies)?;
        let shapes = snapshot_entries(&self.shapes)?;
        let joints = snapshot_entries(&self.joints)?;
        Ok(UserDataManifest {
            world,
            bodies,
            shapes,
            joints,
        })
    }

    pub(crate) fn prepare_restore(
        &self,
        manifest: &UserDataManifest,
        identity_manifest: &crate::core::identity_registry::IdentityManifest,
        identities: &crate::core::identity_registry::PreparedIdentityRestore,
    ) -> Result<PreparedUserDataRestore> {
        let mut target = Self {
            last_version: self.last_version,
            revision: self.revision.wrapping_add(1),
            ..Self::default()
        };
        target
            .bodies
            .try_reserve(manifest.bodies.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;
        target
            .shapes
            .try_reserve(manifest.shapes.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;
        target
            .joints
            .try_reserve(manifest.joints.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;

        let mut attachments = Vec::new();
        let attachment_capacity = usize::from(manifest.world.is_some())
            .checked_add(manifest.bodies.len())
            .and_then(|count| count.checked_add(manifest.shapes.len()))
            .and_then(|count| count.checked_add(manifest.joints.len()))
            .ok_or(Error::SnapshotAllocationFailed)?;
        attachments
            .try_reserve_exact(attachment_capacity)
            .map_err(|_| Error::SnapshotAllocationFailed)?;

        if let (Some(version), Some(entry)) = (manifest.world, self.world.as_ref())
            && let Some(pointer) = entry.pointer_if_version(version)?
        {
            target.world = Some(Rc::clone(entry));
            attachments.push(UserDataAttachment::World(pointer));
        }

        for &(snapshot_id, version) in &manifest.bodies {
            let Some(restored_id) = identities.body_after_restore(identity_manifest, snapshot_id)
            else {
                continue;
            };
            let Some(entry) = self.bodies.get(&snapshot_id) else {
                continue;
            };
            let Some(pointer) = entry.pointer_if_version(version)? else {
                continue;
            };
            target.bodies.insert(restored_id, Rc::clone(entry));
            attachments.push(UserDataAttachment::Body(restored_id, pointer));
        }
        for &(snapshot_id, version) in &manifest.shapes {
            let Some(restored_id) = identities.shape_after_restore(identity_manifest, snapshot_id)
            else {
                continue;
            };
            let Some(entry) = self.shapes.get(&snapshot_id) else {
                continue;
            };
            let Some(pointer) = entry.pointer_if_version(version)? else {
                continue;
            };
            target.shapes.insert(restored_id, Rc::clone(entry));
            attachments.push(UserDataAttachment::Shape(restored_id, pointer));
        }
        for &(snapshot_id, version) in &manifest.joints {
            let Some(restored_id) = identities.joint_after_restore(identity_manifest, snapshot_id)
            else {
                continue;
            };
            let Some(entry) = self.joints.get(&snapshot_id) else {
                continue;
            };
            let Some(pointer) = entry.pointer_if_version(version)? else {
                continue;
            };
            target.joints.insert(restored_id, Rc::clone(entry));
            attachments.push(UserDataAttachment::Joint(restored_id, pointer));
        }

        let retired_capacity = usize::from(self.world.is_some())
            .checked_add(self.bodies.len())
            .and_then(|count| count.checked_add(self.shapes.len()))
            .and_then(|count| count.checked_add(self.joints.len()))
            .ok_or(Error::SnapshotAllocationFailed)?;
        let mut retired = Vec::new();
        retired
            .try_reserve_exact(retired_capacity)
            .map_err(|_| Error::SnapshotAllocationFailed)?;

        Ok(PreparedUserDataRestore {
            base_revision: self.revision,
            target,
            attachments,
            retired,
        })
    }

    /// Remove every entry without dropping its arbitrary payload.
    ///
    /// Callers must take and destroy each entry's erased value behind an individual panic
    /// boundary. Dropping the store as one aggregate could otherwise abort if two payload
    /// destructors panic while Rust is unwinding the same aggregate drop.
    pub(crate) fn drain_entries(&mut self) -> Vec<UserDataEntryRef> {
        let mut entries = Vec::with_capacity(
            usize::from(self.world.is_some())
                + self.bodies.len()
                + self.shapes.len()
                + self.joints.len(),
        );
        entries.extend(self.world.take());
        entries.extend(self.bodies.drain().map(|(_, entry)| entry));
        entries.extend(self.shapes.drain().map(|(_, entry)| entry));
        entries.extend(self.joints.drain().map(|(_, entry)| entry));
        entries
    }
}

#[derive(Clone)]
pub(crate) struct UserDataManifest {
    world: Option<UserDataVersion>,
    bodies: Vec<(BodyId, UserDataVersion)>,
    shapes: Vec<(ShapeId, UserDataVersion)>,
    joints: Vec<(JointId, UserDataVersion)>,
}

pub(crate) enum UserDataAttachment {
    World(*mut c_void),
    Body(BodyId, *mut c_void),
    Shape(ShapeId, *mut c_void),
    Joint(JointId, *mut c_void),
}

pub(crate) struct PreparedUserDataRestore {
    base_revision: u128,
    target: UserDataStore,
    attachments: Vec<UserDataAttachment>,
    retired: Vec<UserDataEntryRef>,
}

impl PreparedUserDataRestore {
    pub(crate) fn commit(mut self, store: &mut UserDataStore) -> Result<CommittedUserDataRestore> {
        if store.revision != self.base_revision {
            return Err(Error::WorldBusy);
        }
        let mut old = core::mem::replace(store, self.target);

        if let Some(entry) = old.world.take()
            && store
                .world
                .as_ref()
                .is_none_or(|survivor| !Rc::ptr_eq(survivor, &entry))
        {
            self.retired.push(entry);
        }
        drain_retired_map(&mut old.bodies, &store.bodies, &mut self.retired);
        drain_retired_map(&mut old.shapes, &store.shapes, &mut self.retired);
        drain_retired_map(&mut old.joints, &store.joints, &mut self.retired);

        Ok(CommittedUserDataRestore {
            attachments: self.attachments,
            retired: self.retired,
        })
    }
}

pub(crate) struct CommittedUserDataRestore {
    attachments: Vec<UserDataAttachment>,
    retired: Vec<UserDataEntryRef>,
}

impl CommittedUserDataRestore {
    pub(crate) fn attachments(&self) -> &[UserDataAttachment] {
        &self.attachments
    }

    pub(crate) fn drop_retired(&mut self) -> std::thread::Result<()> {
        cleanup_retired_user_data(core::mem::take(&mut self.retired)).into_result(())
    }
}

impl Drop for CommittedUserDataRestore {
    fn drop(&mut self) {
        cleanup_retired_user_data(core::mem::take(&mut self.retired)).resume_or_forget();
    }
}

fn cleanup_retired_user_data(
    retired: Vec<UserDataEntryRef>,
) -> crate::core::callback_state::PanicSlot {
    let mut panic = crate::core::callback_state::PanicSlot::default();
    for entry in retired {
        panic.run_cleanup(|| {
            let value = entry
                .take_erased()
                .expect("snapshot prepare checked user-data mutability");
            drop(value);
        });
    }
    panic
}

fn snapshot_entries<Id: Copy + Eq + std::hash::Hash>(
    entries: &HashMap<Id, UserDataEntryRef>,
) -> Result<Vec<(Id, UserDataVersion)>> {
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(entries.len())
        .map_err(|_| Error::SnapshotAllocationFailed)?;
    for (&id, entry) in entries {
        if let Some(version) = entry.version_if_present()? {
            snapshot.push((id, version));
        }
    }
    Ok(snapshot)
}

fn drain_retired_map<Id: Eq + std::hash::Hash>(
    old: &mut HashMap<Id, UserDataEntryRef>,
    target: &HashMap<Id, UserDataEntryRef>,
    retired: &mut Vec<UserDataEntryRef>,
) {
    for (id, entry) in old.drain() {
        if target
            .get(&id)
            .is_none_or(|survivor| !Rc::ptr_eq(survivor, &entry))
        {
            retired.push(entry);
        }
    }
}
