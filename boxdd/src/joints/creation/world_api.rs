use super::*;

trait NativeJointDef {
    type Raw;
    const KIND: JointType;
    const OPERATION: &'static str;

    fn base(&self) -> &JointBase;
    fn validate(&self) -> Result<()>;
    fn to_raw(&self) -> Self::Raw;
}

impl NativeJointDef for DistanceJointDef {
    type Raw = ffi::b2DistanceJointDef;
    const KIND: JointType = JointType::Distance;
    const OPERATION: &'static str = "World::create_distance_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_distance_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for RevoluteJointDef {
    type Raw = ffi::b2RevoluteJointDef;
    const KIND: JointType = JointType::Revolute;
    const OPERATION: &'static str = "World::create_revolute_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_revolute_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for PrismaticJointDef {
    type Raw = ffi::b2PrismaticJointDef;
    const KIND: JointType = JointType::Prismatic;
    const OPERATION: &'static str = "World::create_prismatic_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_prismatic_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for WheelJointDef {
    type Raw = ffi::b2WheelJointDef;
    const KIND: JointType = JointType::Wheel;
    const OPERATION: &'static str = "World::create_wheel_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_wheel_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for WeldJointDef {
    type Raw = ffi::b2WeldJointDef;
    const KIND: JointType = JointType::Weld;
    const OPERATION: &'static str = "World::create_weld_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_weld_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for MotorJointDef {
    type Raw = ffi::b2MotorJointDef;
    const KIND: JointType = JointType::Motor;
    const OPERATION: &'static str = "World::create_motor_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_motor_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

impl NativeJointDef for FilterJointDef {
    type Raw = ffi::b2FilterJointDef;
    const KIND: JointType = JointType::Filter;
    const OPERATION: &'static str = "World::create_filter_joint";

    fn base(&self) -> &JointBase {
        self.base()
    }

    fn validate(&self) -> Result<()> {
        check_filter_joint_def_valid(self)
    }

    fn to_raw(&self) -> Self::Raw {
        self.to_raw()
    }
}

pub(crate) fn check_joint_target_identity(world: &World, base: &JointBase) -> Result<()> {
    world.core().check_body_identity(base.body_a_id())?;
    world.core().check_body_identity(base.body_b_id())
}

pub(crate) fn check_joint_target_native(world: &World, base: &JointBase) -> Result<()> {
    world
        .core()
        .check_body_native_after_identity(base.body_a_id())?;
    world
        .core()
        .check_body_native_after_identity(base.body_b_id())
}

fn create_joint_id<D: NativeJointDef>(
    owner: &dyn crate::world::OwnerAdapter,
    def: &D,
    create: impl FnOnce(ffi::b2WorldId, &D::Raw) -> ffi::b2JointId,
) -> Result<JointId> {
    let creation = crate::world::OwnerCreation::begin(owner)?;
    let base = def.base();
    let core = creation.core();
    if let Err(error) = core.check_body_identity_after_preflight(base.body_a_id()) {
        return creation.abort(error);
    }
    if let Err(error) = core.check_body_identity_after_preflight(base.body_b_id()) {
        return creation.abort(error);
    }
    if let Err(error) = def.validate() {
        return creation.abort(error);
    }
    if let Err(error) = core.check_definition_length_scale(D::OPERATION, base.length_scale()) {
        return creation.abort(error);
    }
    if let Err(error) = core.check_body_native_after_identity(base.body_a_id()) {
        return creation.abort(error);
    }
    if let Err(error) = core.check_body_native_after_identity(base.body_b_id()) {
        return creation.abort(error);
    }
    let pending = match core.reserve_joint_creation(base.body_a_id(), base.body_b_id(), D::KIND) {
        Ok(pending) => pending,
        Err(error) => return creation.abort(error),
    };
    let raw_def = def.to_raw();
    let raw_id = create(core.id, &raw_def);
    let mut native = match core.claim_created_joint(raw_id) {
        Ok(native) => native,
        Err(error) => return creation.abort(error),
    };
    let bound = match core.bind_created_joint(pending, raw_id) {
        Ok(bound) => bound,
        Err(error) => return creation.abort(error),
    };
    creation.finish(|| {
        let id = bound.publish();
        native.commit();
        id
    })
}

pub(crate) fn create_distance_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &DistanceJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateDistanceJoint(world, raw)
    })
}

pub(crate) fn create_motor_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &MotorJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateMotorJoint(world, raw)
    })
}

pub(crate) fn create_filter_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &FilterJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateFilterJoint(world, raw)
    })
}

pub(crate) fn create_prismatic_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &PrismaticJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreatePrismaticJoint(world, raw)
    })
}

pub(crate) fn create_revolute_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &RevoluteJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateRevoluteJoint(world, raw)
    })
}

pub(crate) fn create_weld_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &WeldJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateWeldJoint(world, raw)
    })
}

pub(crate) fn create_wheel_joint_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: &WheelJointDef,
) -> Result<JointId> {
    create_joint_id(owner, def, |world, raw| unsafe {
        ffi::b2CreateWheelJoint(world, raw)
    })
}

impl World {
    pub fn create_distance_joint(&mut self, def: &DistanceJointDef) -> Result<JointId> {
        create_distance_joint_id(self, def)
    }

    pub fn create_revolute_joint(&mut self, def: &RevoluteJointDef) -> Result<JointId> {
        create_revolute_joint_id(self, def)
    }

    pub fn create_prismatic_joint(&mut self, def: &PrismaticJointDef) -> Result<JointId> {
        create_prismatic_joint_id(self, def)
    }

    pub fn create_wheel_joint(&mut self, def: &WheelJointDef) -> Result<JointId> {
        create_wheel_joint_id(self, def)
    }

    pub fn create_weld_joint(&mut self, def: &WeldJointDef) -> Result<JointId> {
        create_weld_joint_id(self, def)
    }

    pub fn create_motor_joint(&mut self, def: &MotorJointDef) -> Result<JointId> {
        create_motor_joint_id(self, def)
    }

    pub fn create_filter_joint(&mut self, def: &FilterJointDef) -> Result<JointId> {
        create_filter_joint_id(self, def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn joint_creation_registers_identity_before_returning() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let joint = world
            .create_distance_joint(&DistanceJointDef::new(
                world.joint_base(body_a, body_b).unwrap(),
            ))
            .unwrap();

        assert_eq!(world.core().check_joint(joint), Ok(()));
    }

    #[test]
    fn invalid_joint_definitions_and_builder_inputs_do_not_reach_native_checks() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let base = world.joint_base(body_a, body_b).unwrap();
        let before = world.core().native_object_check_count_for_test();

        let invalid_hertz = Error::invalid_argument(
            "DistanceJointDef::validate",
            "hertz",
            "a finite non-negative value",
        );
        assert_eq!(
            world.create_distance_joint(&DistanceJointDef::new(base).hertz(-1.0)),
            Err(invalid_hertz)
        );
        assert_eq!(
            world
                .distance(body_a, body_b)
                .spring(-1.0, 0.0)
                .build()
                .unwrap_err(),
            invalid_hertz
        );
        assert_eq!(
            world
                .revolute(body_a, body_b)
                .spring(-1.0, 0.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "RevoluteJointDef::validate",
                "hertz",
                "a finite non-negative value",
            )
        );
        assert_eq!(
            world
                .prismatic(body_a, body_b)
                .spring(-1.0, 0.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "PrismaticJointDef::validate",
                "hertz",
                "a finite non-negative value",
            )
        );
        assert_eq!(
            world
                .wheel(body_a, body_b)
                .spring(-1.0, 0.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "WheelJointDef::validate",
                "hertz",
                "a finite non-negative value",
            )
        );
        assert_eq!(
            world
                .weld(body_a, body_b)
                .linear_stiffness(-1.0, 0.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "WeldJointDef::validate",
                "linear_hertz",
                "a finite non-negative value",
            )
        );
        assert_eq!(
            world
                .distance(body_a, body_b)
                .anchors_world(
                    crate::Position::new(crate::WorldScalar::NAN, 0.0),
                    crate::Position::ZERO,
                )
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "DistanceJointBuilder::build",
                "anchor_a_world",
                "finite coordinates",
            )
        );
        assert_eq!(
            world
                .prismatic(body_a, body_b)
                .axis_world(crate::Vec2::ZERO)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "PrismaticJointBuilder::build",
                "axis_world",
                "a finite non-zero direction",
            )
        );
        assert_eq!(
            world.core().native_object_check_count_for_test(),
            before,
            "all pure definition and builder validation must finish before C"
        );
    }
}
