use super::*;

trait NativeJointDef {
    type Raw;
    const KIND: JointType;

    fn base(&self) -> &JointBase;
    fn validate(&self) -> ApiResult<()>;
    fn to_raw(&self) -> Self::Raw;
}

impl NativeJointDef for DistanceJointDef {
    type Raw = ffi::b2DistanceJointDef;
    const KIND: JointType = JointType::Distance;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_distance_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for RevoluteJointDef {
    type Raw = ffi::b2RevoluteJointDef;
    const KIND: JointType = JointType::Revolute;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_revolute_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for PrismaticJointDef {
    type Raw = ffi::b2PrismaticJointDef;
    const KIND: JointType = JointType::Prismatic;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_prismatic_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for WheelJointDef {
    type Raw = ffi::b2WheelJointDef;
    const KIND: JointType = JointType::Wheel;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_wheel_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for WeldJointDef {
    type Raw = ffi::b2WeldJointDef;
    const KIND: JointType = JointType::Weld;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_weld_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for MotorJointDef {
    type Raw = ffi::b2MotorJointDef;
    const KIND: JointType = JointType::Motor;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_motor_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for FilterJointDef {
    type Raw = ffi::b2FilterJointDef;
    const KIND: JointType = JointType::Filter;

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> ApiResult<()> {
        check_filter_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

pub(crate) fn check_joint_target_identity(world: &World, base: &JointBase) -> ApiResult<()> {
    check_joint_target_identity_with_access(world, base, crate::core::world_core::WorldAccess::Idle)
}

fn check_joint_target_identity_with_access(
    world: &World,
    base: &JointBase,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    world
        .core()
        .check_body_identity_with_access(base.body_a_id(), access)?;
    world
        .core()
        .check_body_identity_with_access(base.body_b_id(), access)
}

pub(crate) fn check_joint_target_native(world: &World, base: &JointBase) -> ApiResult<()> {
    world
        .core()
        .check_body_native_after_identity(base.body_a_id())?;
    world
        .core()
        .check_body_native_after_identity(base.body_b_id())
}

fn check_joint_target_with_access<D: NativeJointDef>(
    world: &World,
    def: &D,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    let base = def.base();
    check_joint_target_identity_with_access(world, base, access)?;
    def.validate()?;
    check_joint_target_native(world, base)
}

fn try_create_joint_id_impl<D: NativeJointDef>(
    world: &mut World,
    def: &D,
    create: impl FnOnce(ffi::b2WorldId, &D::Raw) -> ffi::b2JointId,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        create,
        crate::core::world_core::WorldAccess::Idle,
    )
}

fn try_create_joint_id_impl_with_access<D: NativeJointDef>(
    world: &mut World,
    def: &D,
    create: impl FnOnce(ffi::b2WorldId, &D::Raw) -> ffi::b2JointId,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    crate::core::callback_state::check_not_in_callback()?;
    check_joint_target_with_access(world, def, access)?;

    let raw_def = def.to_raw();
    let raw_id = create(world.raw(), &raw_def);
    let base = def.base();
    world.core().finish_created_joint_with_access(
        raw_id,
        base.body_a_id(),
        base.body_b_id(),
        D::KIND,
        access,
    )
}

pub(crate) fn try_create_distance_joint_id_with_access(
    world: &mut World,
    def: &DistanceJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateDistanceJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_motor_joint_id_with_access(
    world: &mut World,
    def: &MotorJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateMotorJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_filter_joint_id_with_access(
    world: &mut World,
    def: &FilterJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateFilterJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_prismatic_joint_id_with_access(
    world: &mut World,
    def: &PrismaticJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreatePrismaticJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_revolute_joint_id_with_access(
    world: &mut World,
    def: &RevoluteJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateRevoluteJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_weld_joint_id_with_access(
    world: &mut World,
    def: &WeldJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateWeldJoint(world, raw) },
        access,
    )
}

pub(crate) fn try_create_wheel_joint_id_with_access(
    world: &mut World,
    def: &WheelJointDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointId> {
    try_create_joint_id_impl_with_access(
        world,
        def,
        |world, raw| unsafe { ffi::b2CreateWheelJoint(world, raw) },
        access,
    )
}

impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        let id = self.create_distance_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        self.try_create_distance_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_distance_joint_owned(&mut self, def: &DistanceJointDef) -> OwnedJoint {
        let id = self.create_distance_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_distance_joint<'w>(
        &'w mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<Joint<'w>> {
        let id = self.try_create_distance_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_distance_joint_id(&mut self, def: &DistanceJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateDistanceJoint(world, raw)
        })
    }

    pub fn try_create_distance_joint_owned(
        &mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_distance_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        let id = self.create_revolute_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        self.try_create_revolute_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_revolute_joint_owned(&mut self, def: &RevoluteJointDef) -> OwnedJoint {
        let id = self.create_revolute_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_revolute_joint<'w>(
        &'w mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<Joint<'w>> {
        let id = self.try_create_revolute_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateRevoluteJoint(world, raw)
        })
    }

    pub fn try_create_revolute_joint_owned(
        &mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_revolute_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        let id = self.create_prismatic_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        self.try_create_prismatic_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_prismatic_joint_owned(&mut self, def: &PrismaticJointDef) -> OwnedJoint {
        let id = self.create_prismatic_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_prismatic_joint<'w>(
        &'w mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<Joint<'w>> {
        let id = self.try_create_prismatic_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreatePrismaticJoint(world, raw)
        })
    }

    pub fn try_create_prismatic_joint_owned(
        &mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<OwnedJoint> {
        let id = self.try_create_prismatic_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        let id = self.create_wheel_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        self.try_create_wheel_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> OwnedJoint {
        let id = self.create_wheel_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> ApiResult<Joint<'w>> {
        let id = self.try_create_wheel_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_wheel_joint_id(&mut self, def: &WheelJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateWheelJoint(world, raw)
        })
    }

    pub fn try_create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_wheel_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        let id = self.create_weld_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        self.try_create_weld_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_weld_joint_owned(&mut self, def: &WeldJointDef) -> OwnedJoint {
        let id = self.create_weld_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> ApiResult<Joint<'w>> {
        let id = self.try_create_weld_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_weld_joint_id(&mut self, def: &WeldJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateWeldJoint(world, raw)
        })
    }

    pub fn try_create_weld_joint_owned(&mut self, def: &WeldJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_weld_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        let id = self.create_motor_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        self.try_create_motor_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_motor_joint_owned(&mut self, def: &MotorJointDef) -> OwnedJoint {
        let id = self.create_motor_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> ApiResult<Joint<'w>> {
        let id = self.try_create_motor_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_motor_joint_id(&mut self, def: &MotorJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateMotorJoint(world, raw)
        })
    }

    pub fn try_create_motor_joint_owned(&mut self, def: &MotorJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_motor_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        let id = self.create_filter_joint_id(def);
        Joint::new(self.core_rc(), id)
    }

    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        self.try_create_filter_joint_id(def)
            .expect("invalid joint definition or target world")
    }

    pub fn create_filter_joint_owned(&mut self, def: &FilterJointDef) -> OwnedJoint {
        let id = self.create_filter_joint_id(def);
        OwnedJoint::new(self.core_rc(), id)
    }

    pub fn try_create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> ApiResult<Joint<'w>> {
        let id = self.try_create_filter_joint_id(def)?;
        Ok(Joint::new(self.core_rc(), id))
    }

    pub fn try_create_filter_joint_id(&mut self, def: &FilterJointDef) -> ApiResult<JointId> {
        try_create_joint_id_impl(self, def, |world, raw| unsafe {
            ffi::b2CreateFilterJoint(world, raw)
        })
    }

    pub fn try_create_filter_joint_owned(&mut self, def: &FilterJointDef) -> ApiResult<OwnedJoint> {
        let id = self.try_create_filter_joint_id(def)?;
        Ok(OwnedJoint::new(self.core_rc(), id))
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        self.core()
            .destroy_joint_now(id, wake_bodies)
            .expect("invalid joint id or joint belongs to a different world");
    }

    pub fn try_destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core().destroy_joint_now(id, wake_bodies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiError;

    #[test]
    fn joint_creation_registers_identity_before_returning() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let body_a = world.create_body_id(crate::BodyBuilder::new().build());
        let body_b = world.create_body_id(crate::BodyBuilder::new().build());
        let joint = world
            .try_create_distance_joint_id(&DistanceJointDef::new(JointBase::new(body_a, body_b)))
            .unwrap();

        assert_eq!(world.core().check_joint(joint), Ok(()));
        assert_eq!(
            world.core().finish_created_joint(
                joint.into_raw(),
                body_a,
                body_b,
                JointType::Distance
            ),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().check_available(), Err(ApiError::WorldPoisoned));
    }

    #[test]
    fn invalid_joint_definitions_and_builder_inputs_do_not_reach_native_checks() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let body_a = world.create_body_id(crate::BodyBuilder::new().build());
        let body_b = world.create_body_id(crate::BodyBuilder::new().build());
        let base = JointBase::new(body_a, body_b);
        let before = world.core().native_object_check_count_for_test();

        assert_eq!(
            world.try_create_distance_joint_id(&DistanceJointDef::new(base).hertz(-1.0)),
            Err(ApiError::InvalidArgument)
        );
        assert_eq!(
            world
                .distance(body_a, body_b)
                .spring(-1.0, 0.0)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .revolute(body_a, body_b)
                .spring(-1.0, 0.0)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .prismatic(body_a, body_b)
                .spring(-1.0, 0.0)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .wheel(body_a, body_b)
                .spring(-1.0, 0.0)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .weld(body_a, body_b)
                .linear_stiffness(-1.0, 0.0)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .distance(body_a, body_b)
                .anchors_world(
                    crate::Position::new(crate::WorldScalar::NAN, 0.0),
                    crate::Position::ZERO,
                )
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .prismatic(body_a, body_b)
                .axis_world(crate::Vec2::ZERO)
                .try_build()
                .unwrap_err(),
            ApiError::InvalidArgument
        );
        assert_eq!(
            world.core().native_object_check_count_for_test(),
            before,
            "all pure definition and builder validation must finish before C"
        );
    }
}
