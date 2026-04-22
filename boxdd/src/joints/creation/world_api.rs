use super::validation::*;
use super::*;

type JointCreateFn<D> = unsafe extern "C" fn(ffi::b2WorldId, *const D) -> ffi::b2JointId;

fn create_joint_id_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> JointId {
    crate::core::callback_state::assert_not_in_callback();
    assert_joint_def_targets_world(world, base);
    assert_def_valid(raw_def);
    JointId::from_raw(unsafe { create(world.raw(), raw_def) })
}

fn try_create_joint_id_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<JointId> {
    crate::core::callback_state::check_not_in_callback()?;
    check_joint_def_targets_world(world, base)?;
    check_def_valid(raw_def)?;
    Ok(JointId::from_raw(unsafe { create(world.raw(), raw_def) }))
}

fn create_joint_scoped_checked_impl<'w, D>(
    world: &'w mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> Joint<'w> {
    let id = create_joint_id_checked_impl(world, base, raw_def, create, assert_def_valid);
    Joint::new(world.core_arc(), id)
}

fn try_create_joint_scoped_checked_impl<'w, D>(
    world: &'w mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<Joint<'w>> {
    let id = try_create_joint_id_checked_impl(world, base, raw_def, create, check_def_valid)?;
    Ok(Joint::new(world.core_arc(), id))
}

fn create_joint_owned_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> OwnedJoint {
    let id = create_joint_id_checked_impl(world, base, raw_def, create, assert_def_valid);
    OwnedJoint::new(world.core_arc(), id)
}

fn try_create_joint_owned_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<OwnedJoint> {
    let id = try_create_joint_id_checked_impl(world, base, raw_def, create, check_def_valid)?;
    Ok(OwnedJoint::new(world.core_arc(), id))
}

impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn create_distance_joint_owned(&mut self, def: &DistanceJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint<'w>(
        &'w mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint_id(&mut self, def: &DistanceJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint_owned(
        &mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint_owned(&mut self, def: &RevoluteJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint<'w>(
        &'w mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint_owned(
        &mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint_owned(&mut self, def: &PrismaticJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint<'w>(
        &'w mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint_owned(
        &mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint_id(&mut self, def: &WheelJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint_owned(&mut self, def: &WeldJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint_id(&mut self, def: &WeldJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint_owned(&mut self, def: &WeldJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint_owned(&mut self, def: &MotorJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint_id(&mut self, def: &MotorJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint_owned(&mut self, def: &MotorJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint_owned(&mut self, def: &FilterJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint_id(&mut self, def: &FilterJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint_owned(&mut self, def: &FilterJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) } {
            unsafe { ffi::b2DestroyJoint(raw_joint_id(id), wake_bodies) };
            let _ = self.core_arc().clear_joint_user_data(id);
        }
    }

    pub fn try_destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DestroyJoint(raw_joint_id(id), wake_bodies) };
        let _ = self.core_arc().clear_joint_user_data(id);
        Ok(())
    }
}
