use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: ffi::b2JointId,
}

/// Zero-copy view wrapper for a joint event.
/// Borrowed data is valid only within the closure passed to
/// `with_joint_events_view`.
#[derive(Copy, Clone)]
pub struct JointEventView<'a>(&'a ffi::b2JointEvent);
impl<'a> JointEventView<'a> {
    pub fn joint_id(&self) -> ffi::b2JointId {
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

impl World {
    pub fn joint_events(&self) -> Vec<JointEvent> {
        let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
        if raw.count <= 0 || raw.jointEvents.is_null() {
            return Vec::new();
        }
        let s = unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) };
        s.iter()
            .map(|e| JointEvent {
                joint_id: e.jointId,
            })
            .collect()
    }

    pub fn with_joint_events<T>(&self, f: impl FnOnce(&[ffi::b2JointEvent]) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
        let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
        } else {
            &[][..]
        };
        f(slice)
    }

    /// Zero-copy safe view over joint events without exposing raw FFI types.
    /// Borrowed data is valid only within the closure; do not store references.
    ///
    /// Example
    /// ```rust
    /// use boxdd::prelude::*;
    /// let mut world = World::new(WorldDef::default()).unwrap();
    /// world.with_joint_events_view(|it| { let _ = it.count(); });
    /// ```
    pub fn with_joint_events_view<T>(&self, f: impl FnOnce(JointEventIter<'_>) -> T) -> T {
        let raw = unsafe { ffi::b2World_GetJointEvents(self.raw()) };
        let slice = if raw.count > 0 && !raw.jointEvents.is_null() {
            unsafe { core::slice::from_raw_parts(raw.jointEvents, raw.count as usize) }
        } else {
            &[][..]
        };
        f(JointEventIter(slice.iter()))
    }
}
