use super::*;

trait NativeJointDef {
    type Raw;

    fn base(&self) -> &JointBase;
    fn validate(&self) -> ApiResult<()>;
    fn to_raw(&self) -> Self::Raw;
    unsafe fn create(world: ffi::b2WorldId, raw: &Self::Raw) -> ffi::b2JointId;
}

macro_rules! impl_native_joint_def {
    ($definition:ty, $raw:ty, $validate:path, $create:path) => {
        impl NativeJointDef for $definition {
            type Raw = $raw;

            fn base(&self) -> &JointBase {
                self.base()
            }

            fn validate(&self) -> ApiResult<()> {
                $validate(self)
            }

            fn to_raw(&self) -> Self::Raw {
                self.to_raw()
            }

            unsafe fn create(world: ffi::b2WorldId, raw: &Self::Raw) -> ffi::b2JointId {
                unsafe { $create(world, raw) }
            }
        }
    };
}

impl_native_joint_def!(
    DistanceJointDef,
    ffi::b2DistanceJointDef,
    check_distance_joint_def_valid,
    ffi::b2CreateDistanceJoint
);
impl_native_joint_def!(
    RevoluteJointDef,
    ffi::b2RevoluteJointDef,
    check_revolute_joint_def_valid,
    ffi::b2CreateRevoluteJoint
);
impl_native_joint_def!(
    PrismaticJointDef,
    ffi::b2PrismaticJointDef,
    check_prismatic_joint_def_valid,
    ffi::b2CreatePrismaticJoint
);
impl_native_joint_def!(
    WheelJointDef,
    ffi::b2WheelJointDef,
    check_wheel_joint_def_valid,
    ffi::b2CreateWheelJoint
);
impl_native_joint_def!(
    WeldJointDef,
    ffi::b2WeldJointDef,
    check_weld_joint_def_valid,
    ffi::b2CreateWeldJoint
);
impl_native_joint_def!(
    MotorJointDef,
    ffi::b2MotorJointDef,
    check_motor_joint_def_valid,
    ffi::b2CreateMotorJoint
);
impl_native_joint_def!(
    FilterJointDef,
    ffi::b2FilterJointDef,
    check_filter_joint_def_valid,
    ffi::b2CreateFilterJoint
);

fn check_joint_target<D: NativeJointDef>(world: &World, def: &D) -> ApiResult<()> {
    let base = def.base();
    world.core().check_body(base.body_a_id())?;
    world.core().check_body(base.body_b_id())?;
    def.validate()
}

fn try_create_joint_id_impl<D: NativeJointDef>(world: &mut World, def: &D) -> ApiResult<JointId> {
    crate::core::callback_state::check_not_in_callback()?;
    check_joint_target(world, def)?;

    let raw_def = def.to_raw();
    let raw_id = unsafe { D::create(world.raw(), &raw_def) };
    world.core().finish_created_joint(raw_id)
}

fn create_joint_id_impl<D: NativeJointDef>(world: &mut World, def: &D) -> JointId {
    try_create_joint_id_impl(world, def).expect("invalid joint definition or target world")
}

fn try_create_joint_scoped_impl<'w, D: NativeJointDef>(
    world: &'w mut World,
    def: &D,
) -> ApiResult<Joint<'w>> {
    let id = try_create_joint_id_impl(world, def)?;
    Ok(Joint::new(world.core_rc(), id))
}

fn create_joint_scoped_impl<'w, D: NativeJointDef>(world: &'w mut World, def: &D) -> Joint<'w> {
    let id = create_joint_id_impl(world, def);
    Joint::new(world.core_rc(), id)
}

fn try_create_joint_owned_impl<D: NativeJointDef>(
    world: &mut World,
    def: &D,
) -> ApiResult<OwnedJoint> {
    let id = try_create_joint_id_impl(world, def)?;
    Ok(OwnedJoint::new(world.core_rc(), id))
}

fn create_joint_owned_impl<D: NativeJointDef>(world: &mut World, def: &D) -> OwnedJoint {
    let id = create_joint_id_impl(world, def);
    OwnedJoint::new(world.core_rc(), id)
}

macro_rules! impl_world_joint_creation {
    (
        $definition:ty,
        $create_scoped:ident,
        $create_id:ident,
        $create_owned:ident,
        $try_create_scoped:ident,
        $try_create_id:ident,
        $try_create_owned:ident
    ) => {
        impl World {
            pub fn $create_scoped<'w>(&'w mut self, def: &$definition) -> Joint<'w> {
                create_joint_scoped_impl(self, def)
            }

            pub fn $create_id(&mut self, def: &$definition) -> JointId {
                create_joint_id_impl(self, def)
            }

            pub fn $create_owned(&mut self, def: &$definition) -> OwnedJoint {
                create_joint_owned_impl(self, def)
            }

            pub fn $try_create_scoped<'w>(&'w mut self, def: &$definition) -> ApiResult<Joint<'w>> {
                try_create_joint_scoped_impl(self, def)
            }

            pub fn $try_create_id(&mut self, def: &$definition) -> ApiResult<JointId> {
                try_create_joint_id_impl(self, def)
            }

            pub fn $try_create_owned(&mut self, def: &$definition) -> ApiResult<OwnedJoint> {
                try_create_joint_owned_impl(self, def)
            }
        }
    };
}

impl_world_joint_creation!(
    DistanceJointDef,
    create_distance_joint,
    create_distance_joint_id,
    create_distance_joint_owned,
    try_create_distance_joint,
    try_create_distance_joint_id,
    try_create_distance_joint_owned
);
impl_world_joint_creation!(
    RevoluteJointDef,
    create_revolute_joint,
    create_revolute_joint_id,
    create_revolute_joint_owned,
    try_create_revolute_joint,
    try_create_revolute_joint_id,
    try_create_revolute_joint_owned
);
impl_world_joint_creation!(
    PrismaticJointDef,
    create_prismatic_joint,
    create_prismatic_joint_id,
    create_prismatic_joint_owned,
    try_create_prismatic_joint,
    try_create_prismatic_joint_id,
    try_create_prismatic_joint_owned
);
impl_world_joint_creation!(
    WheelJointDef,
    create_wheel_joint,
    create_wheel_joint_id,
    create_wheel_joint_owned,
    try_create_wheel_joint,
    try_create_wheel_joint_id,
    try_create_wheel_joint_owned
);
impl_world_joint_creation!(
    WeldJointDef,
    create_weld_joint,
    create_weld_joint_id,
    create_weld_joint_owned,
    try_create_weld_joint,
    try_create_weld_joint_id,
    try_create_weld_joint_owned
);
impl_world_joint_creation!(
    MotorJointDef,
    create_motor_joint,
    create_motor_joint_id,
    create_motor_joint_owned,
    try_create_motor_joint,
    try_create_motor_joint_id,
    try_create_motor_joint_owned
);
impl_world_joint_creation!(
    FilterJointDef,
    create_filter_joint,
    create_filter_joint_id,
    create_filter_joint_owned,
    try_create_filter_joint,
    try_create_filter_joint_id,
    try_create_filter_joint_owned
);

impl World {
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
            world.core().finish_created_joint(joint.into_raw()),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(world.core().check_available(), Err(ApiError::WorldPoisoned));
    }
}
