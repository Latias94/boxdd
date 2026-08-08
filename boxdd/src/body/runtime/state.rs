use boxdd_sys::ffi;

use crate::error::{Error, Result};
use crate::types::{BodyId, MotionLocks};
use crate::world::BodyCall;

use super::super::{
    definition::{BodyType, check_non_negative_finite_body_scalar},
    scoped::Body,
};
use super::{check_native_body_finite, check_native_body_non_negative, raw_body_id};

#[inline]
fn body_type_raw_impl(id: BodyId) -> ffi::b2BodyType {
    #[cfg(test)]
    {
        BODY_GET_TYPE_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        if let Some(raw) = BODY_GET_TYPE_OVERRIDE.with(core::cell::Cell::get) {
            return raw;
        }
    }
    unsafe { ffi::b2Body_GetType(raw_body_id(id)) }
}

#[cfg(test)]
thread_local! {
    static BODY_GET_TYPE_OVERRIDE: core::cell::Cell<Option<ffi::b2BodyType>> = const {
        core::cell::Cell::new(None)
    };
    static BODY_GET_TYPE_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

#[inline]
pub(crate) fn resolve_body_type_output(
    body: BodyCall<'_>,
    raw: ffi::b2BodyType,
) -> Result<BodyType> {
    BodyType::decode_native(raw).inspect_err(|_| body.poison())
}

#[inline]
pub(crate) fn try_body_type_impl(body: BodyCall<'_>) -> Result<BodyType> {
    resolve_body_type_output(body, body_type_raw_impl(body.id()))
}

#[inline]
fn body_set_type_impl(id: BodyId, body_type: BodyType) {
    unsafe { ffi::b2Body_SetType(raw_body_id(id), body_type.into_raw()) }
}

#[inline]
pub(crate) fn body_gravity_scale_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetGravityScale(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_gravity_scale_impl(id: BodyId, gravity_scale: f32) {
    unsafe { ffi::b2Body_SetGravityScale(raw_body_id(id), gravity_scale) }
}

#[inline]
pub(crate) fn body_linear_damping_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetLinearDamping(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_linear_damping_impl(id: BodyId, linear_damping: f32) {
    unsafe { ffi::b2Body_SetLinearDamping(raw_body_id(id), linear_damping) }
}

#[inline]
pub(crate) fn body_angular_damping_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetAngularDamping(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_angular_damping_impl(id: BodyId, angular_damping: f32) {
    unsafe { ffi::b2Body_SetAngularDamping(raw_body_id(id), angular_damping) }
}

#[inline]
pub(crate) fn body_enable_sleep_impl(id: BodyId, enable_sleep: bool) {
    unsafe { ffi::b2Body_EnableSleep(raw_body_id(id), enable_sleep) }
}

#[inline]
pub(crate) fn body_is_sleep_enabled_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsSleepEnabled(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_sleep_threshold_impl(id: BodyId, sleep_threshold: f32) {
    unsafe { ffi::b2Body_SetSleepThreshold(raw_body_id(id), sleep_threshold) }
}

#[inline]
pub(crate) fn body_sleep_threshold_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetSleepThreshold(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_is_awake_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsAwake(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_awake_impl(id: BodyId, awake: bool) {
    unsafe { ffi::b2Body_SetAwake(raw_body_id(id), awake) }
}

#[inline]
pub(crate) fn body_is_enabled_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsEnabled(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_enable_impl(id: BodyId) {
    unsafe { ffi::b2Body_Enable(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_disable_impl(id: BodyId) {
    unsafe { ffi::b2Body_Disable(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_is_bullet_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsBullet(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_set_bullet_impl(id: BodyId, bullet: bool) {
    unsafe { ffi::b2Body_SetBullet(raw_body_id(id), bullet) }
}

#[inline]
pub(crate) fn body_enable_contact_recycling_impl(id: BodyId, flag: bool) {
    unsafe { ffi::b2Body_EnableContactRecycling(raw_body_id(id), flag) }
}

#[inline]
pub(crate) fn body_is_contact_recycling_enabled_impl(id: BodyId) -> bool {
    unsafe { ffi::b2Body_IsContactRecyclingEnabled(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_enable_contact_events_impl(id: BodyId, flag: bool) {
    unsafe { ffi::b2Body_EnableContactEvents(raw_body_id(id), flag) }
}

#[inline]
pub(crate) fn body_enable_hit_events_impl(id: BodyId, flag: bool) {
    unsafe { ffi::b2Body_EnableHitEvents(raw_body_id(id), flag) }
}

#[inline]
pub(crate) fn body_motion_locks_impl(id: BodyId) -> MotionLocks {
    MotionLocks::from_raw(unsafe { ffi::b2Body_GetMotionLocks(raw_body_id(id)) })
}

impl Body<'_> {
    /// Try to return the body's simulation type.
    ///
    /// An unknown native discriminant returns
    /// [`Error::InvalidNativeBodyType`](crate::Error::InvalidNativeBodyType) and poisons the
    /// world.
    pub fn body_type(&self) -> Result<BodyType> {
        self.body_access().call(try_body_type_impl)
    }

    pub fn set_body_type(&mut self, body_type: BodyType) -> Result<()> {
        self.body_access().call(|_| {
            body_set_type_impl(self.body_id(), body_type);
            Ok(())
        })
    }

    pub fn gravity_scale(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_finite(
                "Body::gravity_scale",
                "gravity_scale",
                body_gravity_scale_impl(self.body_id()),
            )
        })
    }

    pub fn set_gravity_scale(&mut self, gravity_scale: f32) -> Result<()> {
        self.body_access().call(|_| {
            if !crate::is_valid_float(gravity_scale) {
                return Err(Error::invalid_argument(
                    "Body::set_gravity_scale",
                    "gravity_scale",
                    "a finite value",
                ));
            }
            body_set_gravity_scale_impl(self.body_id(), gravity_scale);
            Ok(())
        })
    }

    pub fn linear_damping(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_non_negative(
                "Body::linear_damping",
                "linear_damping",
                body_linear_damping_impl(self.body_id()),
            )
        })
    }

    pub fn set_linear_damping(&mut self, linear_damping: f32) -> Result<()> {
        self.body_access().call(|_| {
            check_non_negative_finite_body_scalar(
                "Body::set_linear_damping",
                "linear_damping",
                linear_damping,
            )?;
            body_set_linear_damping_impl(self.body_id(), linear_damping);
            Ok(())
        })
    }

    pub fn angular_damping(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_non_negative(
                "Body::angular_damping",
                "angular_damping",
                body_angular_damping_impl(self.body_id()),
            )
        })
    }

    pub fn set_angular_damping(&mut self, angular_damping: f32) -> Result<()> {
        self.body_access().call(|_| {
            check_non_negative_finite_body_scalar(
                "Body::set_angular_damping",
                "angular_damping",
                angular_damping,
            )?;
            body_set_angular_damping_impl(self.body_id(), angular_damping);
            Ok(())
        })
    }

    pub fn enable_sleep(&mut self, flag: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_enable_sleep_impl(self.body_id(), flag);
            Ok(())
        })
    }

    pub fn is_sleep_enabled(&self) -> Result<bool> {
        self.body_access()
            .call(|_| Ok(body_is_sleep_enabled_impl(self.body_id())))
    }

    pub fn set_sleep_threshold(&mut self, sleep_threshold: f32) -> Result<()> {
        self.body_access().call(|_| {
            check_non_negative_finite_body_scalar(
                "Body::set_sleep_threshold",
                "sleep_threshold",
                sleep_threshold,
            )?;
            body_set_sleep_threshold_impl(self.body_id(), sleep_threshold);
            Ok(())
        })
    }

    pub fn sleep_threshold(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_non_negative(
                "Body::sleep_threshold",
                "sleep_threshold",
                body_sleep_threshold_impl(self.body_id()),
            )
        })
    }

    pub fn is_awake(&self) -> Result<bool> {
        self.body_access()
            .call(|_| Ok(body_is_awake_impl(self.body_id())))
    }

    pub fn set_awake(&mut self, awake: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_set_awake_impl(self.body_id(), awake);
            Ok(())
        })
    }

    pub fn is_enabled(&self) -> Result<bool> {
        self.body_access()
            .call(|_| Ok(body_is_enabled_impl(self.body_id())))
    }

    pub fn enable(&mut self) -> Result<()> {
        self.body_access().call(|_| {
            body_enable_impl(self.body_id());
            Ok(())
        })
    }

    pub fn disable(&mut self) -> Result<()> {
        self.body_access().call(|_| {
            body_disable_impl(self.body_id());
            Ok(())
        })
    }

    pub fn is_bullet(&self) -> Result<bool> {
        self.body_access()
            .call(|_| Ok(body_is_bullet_impl(self.body_id())))
    }

    pub fn set_bullet(&mut self, flag: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_set_bullet_impl(self.body_id(), flag);
            Ok(())
        })
    }

    pub fn enable_contact_recycling(&mut self, flag: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_enable_contact_recycling_impl(self.body_id(), flag);
            Ok(())
        })
    }

    pub fn is_contact_recycling_enabled(&self) -> Result<bool> {
        self.body_access()
            .call(|_| Ok(body_is_contact_recycling_enabled_impl(self.body_id())))
    }

    pub fn enable_contact_events(&mut self, flag: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_enable_contact_events_impl(self.body_id(), flag);
            Ok(())
        })
    }

    pub fn enable_hit_events(&mut self, flag: bool) -> Result<()> {
        self.body_access().call(|_| {
            body_enable_hit_events_impl(self.body_id(), flag);
            Ok(())
        })
    }

    /// Return the body's translation and rotation locks.
    pub fn motion_locks(&self) -> Result<MotionLocks> {
        self.body_access()
            .call(|_| Ok(body_motion_locks_impl(self.body_id())))
    }

    /// Replace the body's translation and rotation locks.
    pub fn set_motion_locks(&mut self, locks: MotionLocks) -> Result<()> {
        self.body_access().call(|_| {
            unsafe { ffi::b2Body_SetMotionLocks(raw_body_id(self.body_id()), locks.into_raw()) };
            Ok(())
        })
    }

    /// Wake every body currently touching this body.
    pub fn wake_touching(&mut self) -> Result<()> {
        self.body_access().call(|_| {
            unsafe { ffi::b2Body_WakeTouching(raw_body_id(self.body_id())) };
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    struct BodyGetTypeOverride;

    impl BodyGetTypeOverride {
        fn install(raw: ffi::b2BodyType) -> Self {
            BODY_GET_TYPE_OVERRIDE.with(|current| {
                assert_eq!(current.replace(Some(raw)), None);
            });
            BODY_GET_TYPE_CALLS.with(|calls| calls.set(0));
            Self
        }

        fn calls(&self) -> usize {
            BODY_GET_TYPE_CALLS.with(core::cell::Cell::get)
        }
    }

    impl Drop for BodyGetTypeOverride {
        fn drop(&mut self) {
            BODY_GET_TYPE_OVERRIDE.with(|current| current.set(None));
            BODY_GET_TYPE_CALLS.with(|calls| calls.set(0));
        }
    }

    #[test]
    fn body_type_reports_unknown_once_then_stops_before_native_get_type() {
        let raw = ffi::b2BodyType_b2_bodyTypeCount;
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
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body = world.body(body).unwrap();
        let get_type = BodyGetTypeOverride::install(raw);

        assert_eq!(body.body_type(), Err(Error::InvalidNativeBodyType { raw }));
        assert_eq!(get_type.calls(), 1);
        assert_eq!(body.body_type(), Err(Error::WorldPoisoned));
        assert_eq!(get_type.calls(), 1);
    }
}
