use crate::id::IdBrand;
use crate::types::JointId;
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: JointId,
}

impl JointEvent {
    /// Copy a native joint event into an owned Rust value.
    pub(crate) fn from_raw_in(brand: IdBrand, raw: ffi::b2JointEvent) -> Self {
        Self {
            joint_id: brand
                .try_joint(raw.jointId)
                .expect("Box2D joint event contained an invalid joint id"),
        }
    }
}

/// Zero-copy view wrapper for a joint event.
/// Borrowed data is valid only within the closure passed to
/// `with_joint_events_view`.
#[derive(Copy, Clone)]
pub struct JointEventView<'a> {
    event: &'a JointEvent,
}
impl<'a> JointEventView<'a> {
    pub fn joint_id(&self) -> JointId {
        self.event.joint_id
    }
}

pub struct JointEventIter<'a> {
    iter: core::slice::Iter<'a, JointEvent>,
}
impl<'a> Iterator for JointEventIter<'a> {
    type Item = JointEventView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|event| JointEventView { event })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub(super) fn capture_native_events_into(
    world: ffi::b2WorldId,
    brand: IdBrand,
    out: &mut Vec<JointEvent>,
) {
    let raw = unsafe { ffi::b2World_GetJointEvents(world) };
    let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
    } else {
        &[][..]
    };
    super::map_snapshot_into(out, slice, |event| JointEvent::from_raw_in(brand, *event));
}

fn joint_events_snapshot_impl(cache: &super::EventCache) -> Vec<JointEvent> {
    cache.snapshot().joint.clone()
}

fn joint_events_into_impl(cache: &super::EventCache, out: &mut Vec<JointEvent>) {
    let snapshot = cache.snapshot();
    super::map_snapshot_into(out, &snapshot.joint, Clone::clone);
}

impl World {
    pub fn joint_events(&self) -> Vec<JointEvent> {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        joint_events_snapshot_impl(self.event_cache())
    }

    pub fn joint_events_into(&self, out: &mut Vec<JointEvent>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        joint_events_into_impl(self.event_cache(), out);
    }

    pub fn try_joint_events(&self) -> crate::error::ApiResult<Vec<JointEvent>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(joint_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_joint_events_into(&self, out: &mut Vec<JointEvent>) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        joint_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl WorldHandle {
    pub fn joint_events(&self) -> Vec<JointEvent> {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        joint_events_snapshot_impl(self.event_cache())
    }

    pub fn joint_events_into(&self, out: &mut Vec<JointEvent>) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .check_available()
            .expect("world is not available for event access");
        joint_events_into_impl(self.event_cache(), out);
    }

    pub fn try_joint_events(&self) -> crate::error::ApiResult<Vec<JointEvent>> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        Ok(joint_events_snapshot_impl(self.event_cache()))
    }

    pub fn try_joint_events_into(&self, out: &mut Vec<JointEvent>) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().check_available()?;
        joint_events_into_impl(self.event_cache(), out);
        Ok(())
    }
}

impl World {
    /// Low-level raw view over joint events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// Call this immediately after the latest world step and before any operation that may mutate
    /// the world. The returned slice borrows transient Box2D storage. While `f` runs, you must not
    /// perform any operation that can mutate that storage.
    ///
    /// Dropping `Owned*` handles inside `f` is OK; destruction is deferred until after this call.
    pub unsafe fn with_joint_events_raw<T>(&self, f: impl FnOnce(&[ffi::b2JointEvent]) -> T) -> T {
        self.with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
            let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
            } else {
                &[][..]
            };
            f(slice)
        })
    }

    /// Low-level raw view over joint events with recoverable callback-lock checking.
    ///
    /// # Safety
    /// Same safety contract as `with_joint_events_raw`.
    pub unsafe fn try_with_joint_events_raw<T>(
        &self,
        f: impl FnOnce(&[ffi::b2JointEvent]) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
            let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
            } else {
                &[][..]
            };
            f(slice)
        })
    }

    /// Zero-copy view over the Rust-owned completed-step joint events.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes/joints immediately;
    /// the destruction is deferred until after the view ends to preserve existing event-view
    /// ordering semantics.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_joint_events_view(|it| { let _ = it.count(); });
    /// ```
    ///
    pub fn with_joint_events_view<T>(&self, f: impl FnOnce(JointEventIter<'_>) -> T) -> T {
        self.with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(JointEventIter {
                iter: snapshot.joint.iter(),
            })
        })
    }

    /// Zero-copy view over joint events with recoverable callback-lock checking.
    pub fn try_with_joint_events_view<T>(
        &self,
        f: impl FnOnce(JointEventIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let snapshot = self.event_cache().snapshot();
            f(JointEventIter {
                iter: snapshot.joint.iter(),
            })
        })
    }
}
