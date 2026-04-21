use crate::types::JointId;
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: JointId,
}

/// Zero-copy view wrapper for a joint event.
/// Borrowed data is valid only within the closure passed to
/// `with_joint_events_view`.
#[derive(Copy, Clone)]
pub struct JointEventView<'a>(&'a ffi::b2JointEvent);
impl<'a> JointEventView<'a> {
    pub fn joint_id(&self) -> JointId {
        self.0.jointId
    }
}

pub struct JointEventIter<'a>(core::slice::Iter<'a, ffi::b2JointEvent>);
impl<'a> Iterator for JointEventIter<'a> {
    type Item = JointEventView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(JointEventView)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

fn joint_events_into_impl(world: ffi::b2WorldId, out: &mut Vec<JointEvent>) {
    let raw = unsafe { ffi::b2World_GetJointEvents(world) };
    let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
        unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
    } else {
        &[][..]
    };
    super::map_snapshot_into(out, slice, |e| JointEvent {
        joint_id: e.jointId,
    });
}

fn joint_events_snapshot_impl(world: ffi::b2WorldId) -> Vec<JointEvent> {
    let mut out = Vec::new();
    joint_events_into_impl(world, &mut out);
    out
}

fn joint_events_checked_impl(world: ffi::b2WorldId) -> Vec<JointEvent> {
    crate::core::callback_state::assert_not_in_callback();
    joint_events_snapshot_impl(world)
}

fn joint_events_into_checked_impl(world: ffi::b2WorldId, out: &mut Vec<JointEvent>) {
    crate::core::callback_state::assert_not_in_callback();
    joint_events_into_impl(world, out);
}

fn try_joint_events_impl(world: ffi::b2WorldId) -> crate::error::ApiResult<Vec<JointEvent>> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(joint_events_snapshot_impl(world))
}

fn try_joint_events_into_impl(
    world: ffi::b2WorldId,
    out: &mut Vec<JointEvent>,
) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    joint_events_into_impl(world, out);
    Ok(())
}

impl World {
    pub fn joint_events(&self) -> Vec<JointEvent> {
        joint_events_checked_impl(self.raw())
    }

    pub fn joint_events_into(&self, out: &mut Vec<JointEvent>) {
        joint_events_into_checked_impl(self.raw(), out);
    }

    pub fn try_joint_events(&self) -> crate::error::ApiResult<Vec<JointEvent>> {
        try_joint_events_impl(self.raw())
    }

    pub fn try_joint_events_into(&self, out: &mut Vec<JointEvent>) -> crate::error::ApiResult<()> {
        try_joint_events_into_impl(self.raw(), out)
    }
}

impl WorldHandle {
    pub fn joint_events(&self) -> Vec<JointEvent> {
        joint_events_checked_impl(self.raw())
    }

    pub fn joint_events_into(&self, out: &mut Vec<JointEvent>) {
        joint_events_into_checked_impl(self.raw(), out);
    }

    pub fn try_joint_events(&self) -> crate::error::ApiResult<Vec<JointEvent>> {
        try_joint_events_impl(self.raw())
    }

    pub fn try_joint_events_into(&self, out: &mut Vec<JointEvent>) -> crate::error::ApiResult<()> {
        try_joint_events_into_impl(self.raw(), out)
    }
}

impl World {
    /// Low-level raw view over joint events (borrows Box2D's internal buffers).
    ///
    /// # Safety
    /// The returned slice borrows internal Box2D buffers. While `f` runs, you must not perform
    /// any operation that can mutate those buffers (e.g. stepping the world or destroying joints).
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

    /// Zero-copy view over joint events without exposing raw FFI types.
    ///
    /// While `f` runs, dropping `Owned*` handles does not destroy bodies/shapes/joints immediately;
    /// the destruction is deferred until after the view ends to keep the borrowed buffers valid.
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
            let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
            let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
            } else {
                &[][..]
            };
            f(JointEventIter(slice.iter()))
        })
    }

    /// Zero-copy view over joint events with recoverable callback-lock checking.
    pub fn try_with_joint_events_view<T>(
        &self,
        f: impl FnOnce(JointEventIter<'_>) -> T,
    ) -> crate::error::ApiResult<T> {
        self.try_with_borrowed_event_buffers(|| {
            let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
            let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
                unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
            } else {
                &[][..]
            };
            f(JointEventIter(slice.iter()))
        })
    }
}
