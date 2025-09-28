use crate::world::World;
use boxdd_sys::ffi;

#[derive(Clone, Debug)]
pub struct JointEvent {
    pub joint_id: ffi::b2JointId,
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
}
