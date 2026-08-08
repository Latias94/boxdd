use boxdd_sys::ffi;

use crate::error::Result;
use crate::types::{BodyId, Position, Vec2};

use super::super::{
    scoped::Body,
    validation::{
        check_body_world_point_in_local_range, check_valid_body_float, check_valid_body_position,
        check_valid_body_vec2, check_valid_native_body_position,
    },
};
use super::{mass::body_world_center_of_mass_impl, raw_body_id};

#[inline]
fn body_apply_force_impl<F: Into<Vec2>, P: Into<Position>>(
    id: BodyId,
    force: F,
    point: P,
    wake: bool,
) {
    let force: ffi::b2Vec2 = force.into().into_raw();
    let point: ffi::b2Pos = point.into().into_raw();
    unsafe { ffi::b2Body_ApplyForce(raw_body_id(id), force, point, wake) };
}

#[inline]
fn body_apply_force_to_center_impl<V: Into<Vec2>>(id: BodyId, force: V, wake: bool) {
    let force: ffi::b2Vec2 = force.into().into_raw();
    unsafe { ffi::b2Body_ApplyForceToCenter(raw_body_id(id), force, wake) };
}

#[inline]
fn body_apply_torque_impl(id: BodyId, torque: f32, wake: bool) {
    unsafe { ffi::b2Body_ApplyTorque(raw_body_id(id), torque, wake) }
}

#[inline]
fn body_clear_forces_impl(id: BodyId) {
    unsafe { ffi::b2Body_ClearForces(raw_body_id(id)) };
}

#[inline]
fn body_apply_linear_impulse_impl<F: Into<Vec2>, P: Into<Position>>(
    id: BodyId,
    impulse: F,
    point: P,
    wake: bool,
) {
    let impulse: ffi::b2Vec2 = impulse.into().into_raw();
    let point: ffi::b2Pos = point.into().into_raw();
    unsafe { ffi::b2Body_ApplyLinearImpulse(raw_body_id(id), impulse, point, wake) };
}

#[inline]
fn body_apply_linear_impulse_to_center_impl<V: Into<Vec2>>(id: BodyId, impulse: V, wake: bool) {
    let impulse: ffi::b2Vec2 = impulse.into().into_raw();
    unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(raw_body_id(id), impulse, wake) };
}

#[inline]
fn body_apply_angular_impulse_impl(id: BodyId, impulse: f32, wake: bool) {
    unsafe { ffi::b2Body_ApplyAngularImpulse(raw_body_id(id), impulse, wake) }
}

impl Body<'_> {
    pub fn apply_force<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        force: F,
        point: P,
        wake: bool,
    ) -> Result<()> {
        let id = self.body_id();
        let force = crate::core::callback_state::PendingUserValue::new(force);
        let point = crate::core::callback_state::PendingUserValue::new(point);
        self.body_access().call(move |_| {
            let force =
                check_valid_body_vec2("Body::apply_force", "force", force.into_inner().into())?;
            let point =
                check_valid_body_position("Body::apply_force", "point", point.into_inner().into())?;
            let center = check_valid_native_body_position(
                "Body::apply_force",
                "world_center_of_mass",
                body_world_center_of_mass_impl(id),
            )?;
            let point =
                check_body_world_point_in_local_range("Body::apply_force", "point", point, center)?;
            body_apply_force_impl(id, force, point, wake);
            Ok(())
        })
    }

    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) -> Result<()> {
        self.body_access().call(|_| {
            let force =
                check_valid_body_vec2("Body::apply_force_to_center", "force", force.into())?;
            body_apply_force_to_center_impl(self.body_id(), force, wake);
            Ok(())
        })
    }

    pub fn apply_torque(&mut self, torque: f32, wake: bool) -> Result<()> {
        self.body_access().call(|_| {
            let torque = check_valid_body_float("Body::apply_torque", "torque", torque)?;
            body_apply_torque_impl(self.body_id(), torque, wake);
            Ok(())
        })
    }

    pub fn clear_forces(&mut self) -> Result<()> {
        self.body_access().call(|_| {
            body_clear_forces_impl(self.body_id());
            Ok(())
        })
    }

    pub fn apply_linear_impulse<F: Into<Vec2>, P: Into<Position>>(
        &mut self,
        impulse: F,
        point: P,
        wake: bool,
    ) -> Result<()> {
        let id = self.body_id();
        let impulse = crate::core::callback_state::PendingUserValue::new(impulse);
        let point = crate::core::callback_state::PendingUserValue::new(point);
        self.body_access().call(move |_| {
            let impulse = check_valid_body_vec2(
                "Body::apply_linear_impulse",
                "impulse",
                impulse.into_inner().into(),
            )?;
            let point = check_valid_body_position(
                "Body::apply_linear_impulse",
                "point",
                point.into_inner().into(),
            )?;
            let center = check_valid_native_body_position(
                "Body::apply_linear_impulse",
                "world_center_of_mass",
                body_world_center_of_mass_impl(id),
            )?;
            let point = check_body_world_point_in_local_range(
                "Body::apply_linear_impulse",
                "point",
                point,
                center,
            )?;
            body_apply_linear_impulse_impl(id, impulse, point, wake);
            Ok(())
        })
    }

    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        impulse: V,
        wake: bool,
    ) -> Result<()> {
        self.body_access().call(|_| {
            let impulse = check_valid_body_vec2(
                "Body::apply_linear_impulse_to_center",
                "impulse",
                impulse.into(),
            )?;
            body_apply_linear_impulse_to_center_impl(self.body_id(), impulse, wake);
            Ok(())
        })
    }

    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) -> Result<()> {
        self.body_access().call(|_| {
            let impulse =
                check_valid_body_float("Body::apply_angular_impulse", "impulse", impulse)?;
            body_apply_angular_impulse_impl(self.body_id(), impulse, wake);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    struct PositionConversionProbe {
        converted: Rc<Cell<bool>>,
        drops: Rc<Cell<usize>>,
    }

    impl From<PositionConversionProbe> for Position {
        fn from(probe: PositionConversionProbe) -> Self {
            probe.converted.set(true);
            Position::ZERO
        }
    }

    impl Drop for PositionConversionProbe {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
            panic!("secondary unmaterialized body input cleanup panic");
        }
    }

    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_later_apply_force_input_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_APPLY_FORCE_INPUT_ORDER";
        const TEST_NAME: &str = "body::runtime::dynamics::tests::rejected_later_apply_force_input_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer apply-force input unwind remains primary";

        if std::env::var_os(CHILD).is_some() {
            let foundation = crate::Foundation::initialize_default().unwrap();
            let mut world = foundation.create_world(foundation.world_def()).unwrap();
            let body_id = world.create_body(world.body_def()).unwrap();
            let mut body = world.body(body_id).unwrap();
            let converted = Rc::new(Cell::new(false));
            let drops = Rc::new(Cell::new(0));
            let rejected = Rc::new(Cell::new(false));
            let converted_from_drop = Rc::clone(&converted);
            let drops_from_drop = Rc::clone(&drops);
            let rejected_from_drop = Rc::clone(&rejected);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _invoke = InvokeOnDrop(Some(|| {
                    rejected_from_drop.set(
                        body.apply_force(
                            Vec2::new(f32::NAN, 0.0),
                            PositionConversionProbe {
                                converted: converted_from_drop,
                                drops: drops_from_drop,
                            },
                            true,
                        )
                        .is_err(),
                    );
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));

            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert!(!converted.get());
            assert_eq!(drops.get(), 1);
            eprintln!("boxdd-outer-unwind-apply-force-input-order: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind apply-force input-order child process must start");
        assert!(
            output.status.success(),
            "outer-unwind apply-force input-order child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-apply-force-input-order: completed"),
            "outer-unwind apply-force input-order child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
