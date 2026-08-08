use super::*;

fn run_joint_world_coordinate_gate<R>(
    operation: &'static str,
    world: &World,
    body_a: BodyId,
    body_b: BodyId,
    build: impl FnOnce(crate::WorldTransform, crate::WorldTransform) -> crate::error::Result<R>,
) -> crate::error::Result<R> {
    let build = crate::core::callback_state::PendingUserValue::new(build);
    crate::core::callback_state::check_not_in_callback()?;
    world.core().check_body(body_a)?;
    world.core().check_body(body_b)?;
    let transform_a =
        crate::joints::read_native_body_world_transform(operation, "body_a_transform", body_a)?;
    let transform_b =
        crate::joints::read_native_body_world_transform(operation, "body_b_transform", body_b)?;
    build.into_inner()(transform_a, transform_b)
}

fn joint_base_from_world_points_impl<VA: Into<Position>, VB: Into<Position>>(
    operation: &'static str,
    world: &World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: VA,
    anchor_b_world: VB,
) -> crate::error::Result<crate::joints::JointBase> {
    let anchor_a_world = crate::core::callback_state::PendingUserValue::new(anchor_a_world);
    let anchor_b_world = crate::core::callback_state::PendingUserValue::new(anchor_b_world);
    run_joint_world_coordinate_gate(operation, world, body_a, body_b, move |ta, tb| {
        let anchor_a_world = anchor_a_world.into_inner().into();
        let anchor_b_world = anchor_b_world.into_inner().into();
        let la = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_a_world",
            ta,
            anchor_a_world,
        )?;
        let lb = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_b_world",
            tb,
            anchor_b_world,
        )?;
        Ok(world.core().joint_base(body_a, body_b).with_local_frames(
            crate::Transform::from_pos_angle(la, 0.0)?,
            crate::Transform::from_pos_angle(lb, 0.0)?,
        ))
    })
}

fn joint_base_from_shared_world_point_impl<VA: Into<Position>>(
    operation: &'static str,
    world: &World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_world: VA,
) -> crate::error::Result<crate::joints::JointBase> {
    let anchor_world = crate::core::callback_state::PendingUserValue::new(anchor_world);
    run_joint_world_coordinate_gate(operation, world, body_a, body_b, move |ta, tb| {
        let anchor_world = anchor_world.into_inner().into();
        let local_a = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_world",
            ta,
            anchor_world,
        )?;
        let local_b = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_world",
            tb,
            anchor_world,
        )?;
        Ok(world.core().joint_base(body_a, body_b).with_local_frames(
            crate::Transform::from_pos_angle(local_a, 0.0)?,
            crate::Transform::from_pos_angle(local_b, 0.0)?,
        ))
    })
}

fn joint_base_from_world_with_axis_impl<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
    operation: &'static str,
    world: &World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: VA,
    anchor_b_world: VB,
    axis_world: AX,
) -> crate::error::Result<crate::joints::JointBase> {
    let anchor_a_world = crate::core::callback_state::PendingUserValue::new(anchor_a_world);
    let anchor_b_world = crate::core::callback_state::PendingUserValue::new(anchor_b_world);
    let axis_world = crate::core::callback_state::PendingUserValue::new(axis_world);
    run_joint_world_coordinate_gate(operation, world, body_a, body_b, move |ta, tb| {
        let anchor_a_world = anchor_a_world.into_inner().into();
        let anchor_b_world = anchor_b_world.into_inner().into();
        let axis = axis_world.into_inner().into();
        let la = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_a_world",
            ta,
            anchor_a_world,
        )?;
        let lb = crate::joints::checked_world_to_local_point(
            operation,
            "anchor_b_world",
            tb,
            anchor_b_world,
        )?;
        let ra =
            crate::joints::checked_world_axis_to_local_rotation(operation, "axis_world", ta, axis)?;
        let rb =
            crate::joints::checked_world_axis_to_local_rotation(operation, "axis_world", tb, axis)?;
        Ok(world.core().joint_base(body_a, body_b).with_local_frames(
            crate::Transform::from_raw_unvalidated(ffi::b2Transform {
                p: la.into_raw(),
                q: ra.into_raw(),
            }),
            crate::Transform::from_raw_unvalidated(ffi::b2Transform {
                p: lb.into_raw(),
                q: rb.into_raw(),
            }),
        ))
    })
}

impl World {
    // Convenience joints built from world anchors and axis using body ids
    pub fn create_revolute_joint_world<VA: Into<Position>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> crate::error::Result<JointId> {
        let def = crate::joints::RevoluteJointDef::new(joint_base_from_shared_world_point_impl(
            "World::create_revolute_joint_world",
            self,
            body_a,
            body_b,
            anchor_world,
        )?);
        self.create_revolute_joint(&def)
    }

    pub fn create_prismatic_joint_world<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::error::Result<JointId> {
        let def = crate::joints::PrismaticJointDef::new(joint_base_from_world_with_axis_impl(
            "World::create_prismatic_joint_world",
            self,
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        )?);
        self.create_prismatic_joint(&def)
    }

    pub fn create_wheel_joint_world<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::error::Result<JointId> {
        let def = crate::joints::WheelJointDef::new(joint_base_from_world_with_axis_impl(
            "World::create_wheel_joint_world",
            self,
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        )?);
        self.create_wheel_joint(&def)
    }

    /// Build `JointBase` from two world anchor points.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{Foundation, World};
    /// let foundation = Foundation::initialize_default().unwrap();
    /// let mut world = foundation.create_world(foundation.world_builder().gravity([0.0, -9.8]).build().unwrap()).unwrap();
    /// let a = world.create_body(world.body_builder().position([-1.0, 2.0]).build().unwrap()).unwrap();
    /// let b = world.create_body(world.body_builder().position([1.0, 2.0]).build().unwrap()).unwrap();
    /// let base = world
    ///     .joint_base_from_world_points(a, b, [-1.0, 2.0], [1.0, 2.0])
    ///     .unwrap();
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
    ) -> crate::error::Result<crate::joints::JointBase> {
        joint_base_from_world_points_impl(
            "World::joint_base_from_world_points",
            self,
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
        )
    }

    /// Build `JointBase` from world anchors and a shared world axis (X-axis of local frames).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{Foundation, Vec2, World};
    /// let foundation = Foundation::initialize_default().unwrap();
    /// let mut world = foundation.create_world(foundation.world_builder().gravity([0.0, -9.8]).build().unwrap()).unwrap();
    /// let a = world.create_body(world.body_builder().position([0.0, 2.0]).build().unwrap()).unwrap();
    /// let b = world.create_body(world.body_builder().position([1.0, 2.0]).build().unwrap()).unwrap();
    /// let axis = Vec2::new(1.0, 0.0);
    /// let base = world
    ///     .joint_base_from_world_with_axis(a, b, [0.0, 2.0], [1.0, 2.0], axis)
    ///     .unwrap();
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_with_axis<
        VA: Into<Position>,
        VB: Into<Position>,
        AX: Into<Vec2>,
    >(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::error::Result<crate::joints::JointBase> {
        joint_base_from_world_with_axis_impl(
            "World::joint_base_from_world_with_axis",
            self,
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    struct PositionConversionProbe {
        converted: Rc<Cell<bool>>,
    }

    impl From<PositionConversionProbe> for Position {
        fn from(probe: PositionConversionProbe) -> Self {
            probe.converted.set(true);
            Position::new(0.0, 0.0)
        }
    }

    struct PanickingDropPositionProbe {
        converted: Rc<Cell<bool>>,
        drops: Rc<Cell<usize>>,
    }

    impl From<PanickingDropPositionProbe> for Position {
        fn from(probe: PanickingDropPositionProbe) -> Self {
            probe.converted.set(true);
            Position::ZERO
        }
    }

    impl Drop for PanickingDropPositionProbe {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
            panic!("secondary rejected joint input cleanup panic");
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

    fn world_with_two_bodies() -> (World, BodyId, BodyId) {
        let foundation = crate::Foundation::initialize_default().unwrap();
        let mut world = foundation.create_world(foundation.world_def()).unwrap();
        let body_a = world.create_body(world.body_def()).unwrap();
        let body_b = world.create_body(world.body_def()).unwrap();
        (world, body_a, body_b)
    }

    #[test]
    fn revolute_world_anchor_conversion_runs_after_callback_gate() {
        let (mut world, body_a, body_b) = world_with_two_bodies();
        let converted = Rc::new(Cell::new(false));
        let _callback = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world
                .create_revolute_joint_world(
                    body_a,
                    body_b,
                    PositionConversionProbe {
                        converted: Rc::clone(&converted),
                    },
                )
                .unwrap_err(),
            crate::Error::InCallback
        );
        assert!(!converted.get());
    }

    #[test]
    fn revolute_world_anchor_conversion_runs_after_owner_validation() {
        let (mut world, body_a, _) = world_with_two_bodies();
        let (foreign_world, foreign_body, _) = world_with_two_bodies();
        let converted = Rc::new(Cell::new(false));

        assert_eq!(
            world
                .create_revolute_joint_world(
                    body_a,
                    foreign_body,
                    PositionConversionProbe {
                        converted: Rc::clone(&converted),
                    },
                )
                .unwrap_err(),
            crate::Error::WrongWorld
        );
        assert!(!converted.get());
        drop(foreign_world);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_joint_world_input_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_JOINT_WORLD_INPUT";
        const TEST_NAME: &str = "world::creation::joint_builders::tests::rejected_joint_world_input_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer rejected joint input unwind remains primary";

        if std::env::var_os(CHILD).is_some() {
            let (world, body_a, body_b) = world_with_two_bodies();
            let converted = Rc::new(Cell::new(false));
            let drops = Rc::new(Cell::new(0));
            let rejected = Rc::new(Cell::new(false));
            let converted_from_drop = Rc::clone(&converted);
            let drops_from_drop = Rc::clone(&drops);
            let rejected_from_drop = Rc::clone(&rejected);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _callback = crate::core::callback_state::CallbackGuard::enter();
                let _invoke = InvokeOnDrop(Some(|| {
                    rejected_from_drop.set(matches!(
                        world.joint_base_from_world_points(
                            body_a,
                            body_b,
                            PanickingDropPositionProbe {
                                converted: converted_from_drop,
                                drops: drops_from_drop,
                            },
                            Position::ZERO,
                        ),
                        Err(crate::Error::InCallback)
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));

            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert!(!converted.get());
            assert_eq!(drops.get(), 1);
            eprintln!("boxdd-outer-unwind-rejected-joint-world-input: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected joint input child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected joint input child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-joint-world-input: completed"),
            "outer-unwind rejected joint input child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
