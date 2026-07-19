use super::*;

fn joint_base_from_world_points_impl<VA: Into<Position>, VB: Into<Position>>(
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: VA,
    anchor_b_world: VB,
) -> crate::joints::JointBase {
    crate::core::debug_checks::assert_body_valid(body_a);
    crate::core::debug_checks::assert_body_valid(body_b);
    let ta = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_a)) });
    let tb = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_b)) });
    let la =
        crate::joints::JointBaseBuilder::checked_world_to_local_point(ta, anchor_a_world.into())
            .expect("joint world anchor A must fit in its local f32 frame");
    let lb =
        crate::joints::JointBaseBuilder::checked_world_to_local_point(tb, anchor_b_world.into())
            .expect("joint world anchor B must fit in its local f32 frame");
    crate::joints::JointBaseBuilder::new()
        .bodies_by_id(body_a, body_b)
        .local_frames_raw(
            ffi::b2Transform {
                p: la.into_raw(),
                q: ffi::b2Rot { c: 1.0, s: 0.0 },
            },
            ffi::b2Transform {
                p: lb.into_raw(),
                q: ffi::b2Rot { c: 1.0, s: 0.0 },
            },
        )
        .build()
}

fn joint_base_from_world_with_axis_impl<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: VA,
    anchor_b_world: VB,
    axis_world: AX,
) -> crate::joints::JointBase {
    crate::core::debug_checks::assert_body_valid(body_a);
    crate::core::debug_checks::assert_body_valid(body_b);
    let ta = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_a)) });
    let tb = WorldTransform::from_raw(unsafe { ffi::b2Body_GetTransform(raw_body_id(body_b)) });
    let axis = axis_world.into();
    let la =
        crate::joints::JointBaseBuilder::checked_world_to_local_point(ta, anchor_a_world.into())
            .expect("joint world anchor A must fit in its local f32 frame");
    let lb =
        crate::joints::JointBaseBuilder::checked_world_to_local_point(tb, anchor_b_world.into())
            .expect("joint world anchor B must fit in its local f32 frame");
    let ra = crate::joints::JointBaseBuilder::checked_world_axis_to_local_rotation(ta, axis)
        .expect("joint world axis must define body A's local frame");
    let rb = crate::joints::JointBaseBuilder::checked_world_axis_to_local_rotation(tb, axis)
        .expect("joint world axis must define body B's local frame");
    crate::joints::JointBaseBuilder::new()
        .bodies_by_id(body_a, body_b)
        .local_frames_raw(
            ffi::b2Transform {
                p: la.into_raw(),
                q: ra.into_raw(),
            },
            ffi::b2Transform {
                p: lb.into_raw(),
                q: rb.into_raw(),
            },
        )
        .build()
}

impl World {
    // Convenience joints built from world anchors and axis using body ids
    pub fn create_revolute_joint_world<VA: Into<Position>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> crate::joints::Joint<'_> {
        let aw = anchor_world.into();
        let def = crate::joints::RevoluteJointDef::new(joint_base_from_world_points_impl(
            body_a, body_b, aw, aw,
        ));
        self.create_revolute_joint(&def)
    }

    pub fn create_revolute_joint_world_id<VA: Into<Position>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> JointId {
        let aw = anchor_world.into();
        let def = crate::joints::RevoluteJointDef::new(joint_base_from_world_points_impl(
            body_a, body_b, aw, aw,
        ));
        self.create_revolute_joint_id(&def)
    }

    pub fn create_prismatic_joint_world<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let def = crate::joints::PrismaticJointDef::new(joint_base_from_world_with_axis_impl(
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        ));
        self.create_prismatic_joint(&def)
    }

    pub fn create_prismatic_joint_world_id<
        VA: Into<Position>,
        VB: Into<Position>,
        AX: Into<Vec2>,
    >(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let def = crate::joints::PrismaticJointDef::new(joint_base_from_world_with_axis_impl(
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        ));
        self.create_prismatic_joint_id(&def)
    }

    pub fn create_wheel_joint_world<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let def = crate::joints::WheelJointDef::new(joint_base_from_world_with_axis_impl(
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        ));
        self.create_wheel_joint(&def)
    }

    pub fn create_wheel_joint_world_id<VA: Into<Position>, VB: Into<Position>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let def = crate::joints::WheelJointDef::new(joint_base_from_world_with_axis_impl(
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        ));
        self.create_wheel_joint_id(&def)
    }

    /// Helper: build a joint base from two world anchor points.
    /// Build `JointBase` from two world anchor points.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([-1.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([ 1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let base = world.joint_base_from_world_points(a, b, world.body_position(a), world.body_position(b));
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_points<VA: Into<Position>, VB: Into<Position>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
    ) -> crate::joints::JointBase {
        joint_base_from_world_points_impl(body_a, body_b, anchor_a_world, anchor_b_world)
    }

    /// Helper: build a joint base from two world anchors and a shared world axis (X-axis of joint frames).
    /// Build `JointBase` from world anchors and a shared world axis (X-axis of local frames).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([0.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let axis = Vec2::new(1.0, 0.0);
    /// let base = world.joint_base_from_world_with_axis(a, b, world.body_position(a), world.body_position(b), axis);
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
    ) -> crate::joints::JointBase {
        joint_base_from_world_with_axis_impl(
            body_a,
            body_b,
            anchor_a_world,
            anchor_b_world,
            axis_world,
        )
    }
}
