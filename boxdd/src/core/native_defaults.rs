use boxdd_sys::ffi;

// This is part of Box2D's pinned definition ABI. Parity tests compare every constructor below
// with the corresponding native default function so an upstream repin cannot silently drift.
pub(crate) const DEFINITION_COOKIE: i32 = boxdd_sys::DEFINITION_COOKIE;

#[inline]
pub(crate) const fn filter() -> ffi::b2Filter {
    ffi::b2Filter {
        categoryBits: ffi::B2_DEFAULT_CATEGORY_BITS as u64,
        maskBits: ffi::B2_DEFAULT_MASK_BITS,
        groupIndex: 0,
    }
}

#[inline]
pub(crate) const fn query_filter() -> ffi::b2QueryFilter {
    ffi::b2QueryFilter {
        categoryBits: ffi::B2_DEFAULT_CATEGORY_BITS as u64,
        maskBits: ffi::B2_DEFAULT_MASK_BITS,
    }
}

#[inline]
pub(crate) const fn surface_material() -> ffi::b2SurfaceMaterial {
    ffi::b2SurfaceMaterial {
        friction: 0.6,
        restitution: 0.0,
        rollingResistance: 0.0,
        tangentSpeed: 0.0,
        userMaterialId: 0,
        customColor: 0,
    }
}

pub(crate) fn world_def(length_units: f32, worker_count: i32) -> ffi::b2WorldDef {
    ffi::b2WorldDef {
        gravity: ffi::b2Vec2 { x: 0.0, y: -10.0 },
        restitutionThreshold: length_units,
        hitEventThreshold: length_units,
        contactHertz: 30.0,
        contactDampingRatio: 10.0,
        contactSpeed: 3.0 * length_units,
        maximumLinearSpeed: 400.0 * length_units,
        frictionCallback: None,
        restitutionCallback: None,
        enableSleep: true,
        enableContinuous: true,
        enableContactSoftening: false,
        workerCount: worker_count,
        enqueueTask: None,
        finishTask: None,
        userTaskContext: core::ptr::null_mut(),
        userData: core::ptr::null_mut(),
        capacity: ffi::b2Capacity {
            staticShapeCount: 0,
            dynamicShapeCount: 0,
            staticBodyCount: 0,
            dynamicBodyCount: 0,
            contactCount: 0,
        },
        internalValue: DEFINITION_COOKIE,
    }
}

pub(crate) fn body_def(length_units: f32) -> ffi::b2BodyDef {
    ffi::b2BodyDef {
        type_: ffi::b2BodyType_b2_staticBody,
        position: ffi::b2Pos { x: 0.0, y: 0.0 },
        rotation: ffi::b2Rot { c: 1.0, s: 0.0 },
        linearVelocity: ffi::b2Vec2 { x: 0.0, y: 0.0 },
        angularVelocity: 0.0,
        linearDamping: 0.0,
        angularDamping: 0.0,
        gravityScale: 1.0,
        sleepThreshold: 0.05 * length_units,
        name: core::ptr::null(),
        userData: core::ptr::null_mut(),
        motionLocks: ffi::b2MotionLocks {
            linearX: false,
            linearY: false,
            angularZ: false,
        },
        enableSleep: true,
        isAwake: true,
        isBullet: false,
        isEnabled: true,
        allowFastRotation: false,
        enableContactRecycling: true,
        internalValue: DEFINITION_COOKIE,
    }
}

pub(crate) const fn shape_def() -> ffi::b2ShapeDef {
    ffi::b2ShapeDef {
        userData: core::ptr::null_mut(),
        material: surface_material(),
        density: 1.0,
        filter: filter(),
        enableCustomFiltering: false,
        isSensor: false,
        enableSensorEvents: false,
        enableContactEvents: false,
        enableHitEvents: false,
        enablePreSolveEvents: false,
        invokeContactCreation: true,
        updateBodyMass: true,
        internalValue: DEFINITION_COOKIE,
    }
}

pub(crate) const fn chain_def(materials: *const ffi::b2SurfaceMaterial) -> ffi::b2ChainDef {
    ffi::b2ChainDef {
        userData: core::ptr::null_mut(),
        points: core::ptr::null(),
        count: 0,
        materials,
        materialCount: 1,
        filter: filter(),
        isLoop: false,
        enableSensorEvents: false,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn explosion_def() -> ffi::b2ExplosionDef {
    ffi::b2ExplosionDef {
        maskBits: ffi::B2_DEFAULT_MASK_BITS,
        position: ffi::b2Pos { x: 0.0, y: 0.0 },
        radius: 0.0,
        falloff: 0.0,
        impulsePerLength: 0.0,
    }
}

pub(crate) fn distance_joint_def(
    base: ffi::b2JointDef,
    length_units: f32,
) -> ffi::b2DistanceJointDef {
    ffi::b2DistanceJointDef {
        base,
        length: 1.0,
        enableSpring: false,
        lowerSpringForce: -f32::MAX,
        upperSpringForce: f32::MAX,
        hertz: 0.0,
        dampingRatio: 0.0,
        enableLimit: false,
        minLength: 0.0,
        maxLength: if cfg!(feature = "double-precision") {
            1.0e9 * length_units
        } else {
            1.0e5 * length_units
        },
        enableMotor: false,
        maxMotorForce: 0.0,
        motorSpeed: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn motor_joint_def(base: ffi::b2JointDef) -> ffi::b2MotorJointDef {
    ffi::b2MotorJointDef {
        base,
        linearVelocity: ffi::b2Vec2 { x: 0.0, y: 0.0 },
        maxVelocityForce: 0.0,
        angularVelocity: 0.0,
        maxVelocityTorque: 0.0,
        linearHertz: 0.0,
        linearDampingRatio: 0.0,
        maxSpringForce: 0.0,
        angularHertz: 0.0,
        angularDampingRatio: 0.0,
        maxSpringTorque: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn filter_joint_def(base: ffi::b2JointDef) -> ffi::b2FilterJointDef {
    ffi::b2FilterJointDef {
        base,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn prismatic_joint_def(base: ffi::b2JointDef) -> ffi::b2PrismaticJointDef {
    ffi::b2PrismaticJointDef {
        base,
        enableSpring: false,
        hertz: 0.0,
        dampingRatio: 0.0,
        targetTranslation: 0.0,
        enableLimit: false,
        lowerTranslation: 0.0,
        upperTranslation: 0.0,
        enableMotor: false,
        maxMotorForce: 0.0,
        motorSpeed: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn revolute_joint_def(base: ffi::b2JointDef) -> ffi::b2RevoluteJointDef {
    ffi::b2RevoluteJointDef {
        base,
        targetAngle: 0.0,
        enableSpring: false,
        hertz: 0.0,
        dampingRatio: 0.0,
        enableLimit: false,
        lowerAngle: 0.0,
        upperAngle: 0.0,
        enableMotor: false,
        maxMotorTorque: 0.0,
        motorSpeed: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn weld_joint_def(base: ffi::b2JointDef) -> ffi::b2WeldJointDef {
    ffi::b2WeldJointDef {
        base,
        linearHertz: 0.0,
        angularHertz: 0.0,
        linearDampingRatio: 0.0,
        angularDampingRatio: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[inline]
pub(crate) const fn wheel_joint_def(base: ffi::b2JointDef) -> ffi::b2WheelJointDef {
    ffi::b2WheelJointDef {
        base,
        enableSpring: true,
        hertz: 1.0,
        dampingRatio: 0.7,
        enableLimit: false,
        lowerTranslation: 0.0,
        upperTranslation: 0.0,
        enableMotor: false,
        maxMotorTorque: 0.0,
        motorSpeed: 0.0,
        internalValue: DEFINITION_COOKIE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_fields_eq {
        ($actual:expr, $native:expr; $($field:ident),+ $(,)?) => {{
            let actual = &$actual;
            let native = &$native;
            $(assert_eq!(actual.$field, native.$field, stringify!($field));)+
        }};
    }

    fn assert_vec2(actual: ffi::b2Vec2, native: ffi::b2Vec2) {
        assert_fields_eq!(actual, native; x, y);
    }

    fn assert_pos(actual: ffi::b2Pos, native: ffi::b2Pos) {
        assert_fields_eq!(actual, native; x, y);
    }

    fn assert_rot(actual: ffi::b2Rot, native: ffi::b2Rot) {
        assert_fields_eq!(actual, native; c, s);
    }

    fn assert_transform(actual: ffi::b2Transform, native: ffi::b2Transform) {
        assert_vec2(actual.p, native.p);
        assert_rot(actual.q, native.q);
    }

    fn assert_body_id(actual: ffi::b2BodyId, native: ffi::b2BodyId) {
        assert_fields_eq!(actual, native; index1, world0, generation);
    }

    fn assert_filter(actual: ffi::b2Filter, native: ffi::b2Filter) {
        assert_fields_eq!(actual, native; categoryBits, maskBits, groupIndex);
    }

    fn assert_query_filter(actual: ffi::b2QueryFilter, native: ffi::b2QueryFilter) {
        assert_fields_eq!(actual, native; categoryBits, maskBits);
    }

    fn assert_material(actual: ffi::b2SurfaceMaterial, native: ffi::b2SurfaceMaterial) {
        assert_fields_eq!(
            actual,
            native;
            friction,
            restitution,
            rollingResistance,
            tangentSpeed,
            userMaterialId,
            customColor,
        );
    }

    fn assert_joint_base(actual: ffi::b2JointDef, native: ffi::b2JointDef) {
        assert!(actual.userData.is_null());
        assert!(native.userData.is_null());
        assert_body_id(actual.bodyIdA, native.bodyIdA);
        assert_body_id(actual.bodyIdB, native.bodyIdB);
        assert_transform(actual.localFrameA, native.localFrameA);
        assert_transform(actual.localFrameB, native.localFrameB);
        assert_fields_eq!(
            actual,
            native;
            forceThreshold,
            torqueThreshold,
            constraintHertz,
            constraintDampingRatio,
            drawScale,
            collideConnected,
        );
    }

    #[test]
    fn rust_defaults_match_the_pinned_native_definition_contract() {
        crate::Foundation::initialize_default().unwrap();

        let _lease = crate::core::foundation::acquire_transient_lease().unwrap();
        let length_units = unsafe { ffi::b2GetLengthUnitsPerMeter() };

        let native_filter = unsafe { ffi::b2DefaultFilter() };
        assert_filter(filter(), native_filter);

        let native_query_filter = unsafe { ffi::b2DefaultQueryFilter() };
        assert_query_filter(query_filter(), native_query_filter);

        let native_material = unsafe { ffi::b2DefaultSurfaceMaterial() };
        assert_material(surface_material(), native_material);

        let native_world = unsafe { ffi::b2DefaultWorldDef() };
        let default_worker_count = crate::world::WorkerCount::default().as_i32();
        assert_eq!(native_world.workerCount, 0);
        assert_eq!(default_worker_count, 1);
        let actual_world = world_def(length_units, default_worker_count);
        assert_eq!(actual_world.workerCount, default_worker_count);
        assert_vec2(actual_world.gravity, native_world.gravity);
        assert_fields_eq!(
            actual_world,
            native_world;
            restitutionThreshold,
            hitEventThreshold,
            contactHertz,
            contactDampingRatio,
            contactSpeed,
            maximumLinearSpeed,
            enableSleep,
            enableContinuous,
            enableContactSoftening,
            internalValue,
        );
        assert_eq!(
            actual_world.frictionCallback.is_none(),
            native_world.frictionCallback.is_none()
        );
        assert_eq!(
            actual_world.restitutionCallback.is_none(),
            native_world.restitutionCallback.is_none()
        );
        assert_eq!(
            actual_world.enqueueTask.is_none(),
            native_world.enqueueTask.is_none()
        );
        assert_eq!(
            actual_world.finishTask.is_none(),
            native_world.finishTask.is_none()
        );
        assert!(actual_world.userTaskContext.is_null());
        assert!(native_world.userTaskContext.is_null());
        assert!(actual_world.userData.is_null());
        assert!(native_world.userData.is_null());
        assert_fields_eq!(
            actual_world.capacity,
            native_world.capacity;
            staticShapeCount,
            dynamicShapeCount,
            staticBodyCount,
            dynamicBodyCount,
            contactCount,
        );

        let native_body = unsafe { ffi::b2DefaultBodyDef() };
        let actual_body = body_def(length_units);
        assert_fields_eq!(
            actual_body,
            native_body;
            type_,
            angularVelocity,
            linearDamping,
            angularDamping,
            gravityScale,
            sleepThreshold,
            enableSleep,
            isAwake,
            isBullet,
            isEnabled,
            allowFastRotation,
            enableContactRecycling,
            internalValue,
        );
        assert_pos(actual_body.position, native_body.position);
        assert_rot(actual_body.rotation, native_body.rotation);
        assert_vec2(actual_body.linearVelocity, native_body.linearVelocity);
        assert!(actual_body.name.is_null());
        assert!(native_body.name.is_null());
        assert!(actual_body.userData.is_null());
        assert!(native_body.userData.is_null());
        assert_fields_eq!(
            actual_body.motionLocks,
            native_body.motionLocks;
            linearX,
            linearY,
            angularZ,
        );

        let native_shape = unsafe { ffi::b2DefaultShapeDef() };
        let actual_shape = shape_def();
        assert!(actual_shape.userData.is_null());
        assert!(native_shape.userData.is_null());
        assert_material(actual_shape.material, native_shape.material);
        assert_filter(actual_shape.filter, native_shape.filter);
        assert_fields_eq!(
            actual_shape,
            native_shape;
            density,
            enableCustomFiltering,
            isSensor,
            enableSensorEvents,
            enableContactEvents,
            enableHitEvents,
            enablePreSolveEvents,
            invokeContactCreation,
            updateBodyMass,
            internalValue,
        );

        let native_chain = unsafe { ffi::b2DefaultChainDef() };
        let actual_chain = chain_def(native_chain.materials);
        assert!(actual_chain.userData.is_null());
        assert!(native_chain.userData.is_null());
        assert!(actual_chain.points.is_null());
        assert!(native_chain.points.is_null());
        assert_eq!(actual_chain.materials, native_chain.materials);
        assert_filter(actual_chain.filter, native_chain.filter);
        assert_fields_eq!(
            actual_chain,
            native_chain;
            count,
            materialCount,
            isLoop,
            enableSensorEvents,
            internalValue,
        );
        assert_material(unsafe { *actual_chain.materials }, unsafe {
            *native_chain.materials
        });

        let native_explosion = unsafe { ffi::b2DefaultExplosionDef() };
        let actual_explosion = explosion_def();
        assert_pos(actual_explosion.position, native_explosion.position);
        assert_fields_eq!(
            actual_explosion,
            native_explosion;
            maskBits,
            radius,
            falloff,
            impulsePerLength,
        );

        let native = unsafe { ffi::b2DefaultDistanceJointDef() };
        let actual = distance_joint_def(native.base, length_units);
        assert_joint_base(actual.base, native.base);
        assert_fields_eq!(
            actual,
            native;
            length,
            enableSpring,
            lowerSpringForce,
            upperSpringForce,
            hertz,
            dampingRatio,
            enableLimit,
            minLength,
            maxLength,
            enableMotor,
            maxMotorForce,
            motorSpeed,
            internalValue,
        );

        let native = unsafe { ffi::b2DefaultMotorJointDef() };
        let actual = motor_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_vec2(actual.linearVelocity, native.linearVelocity);
        assert_fields_eq!(
            actual,
            native;
            maxVelocityForce,
            angularVelocity,
            maxVelocityTorque,
            linearHertz,
            linearDampingRatio,
            maxSpringForce,
            angularHertz,
            angularDampingRatio,
            maxSpringTorque,
            internalValue,
        );

        let native = unsafe { ffi::b2DefaultFilterJointDef() };
        let actual = filter_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_eq!(actual.internalValue, native.internalValue);

        let native = unsafe { ffi::b2DefaultPrismaticJointDef() };
        let actual = prismatic_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_fields_eq!(
            actual,
            native;
            enableSpring,
            hertz,
            dampingRatio,
            targetTranslation,
            enableLimit,
            lowerTranslation,
            upperTranslation,
            enableMotor,
            maxMotorForce,
            motorSpeed,
            internalValue,
        );

        let native = unsafe { ffi::b2DefaultRevoluteJointDef() };
        let actual = revolute_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_fields_eq!(
            actual,
            native;
            targetAngle,
            enableSpring,
            hertz,
            dampingRatio,
            enableLimit,
            lowerAngle,
            upperAngle,
            enableMotor,
            maxMotorTorque,
            motorSpeed,
            internalValue,
        );

        let native = unsafe { ffi::b2DefaultWeldJointDef() };
        let actual = weld_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_fields_eq!(
            actual,
            native;
            linearHertz,
            angularHertz,
            linearDampingRatio,
            angularDampingRatio,
            internalValue,
        );

        let native = unsafe { ffi::b2DefaultWheelJointDef() };
        let actual = wheel_joint_def(native.base);
        assert_joint_base(actual.base, native.base);
        assert_fields_eq!(
            actual,
            native;
            enableSpring,
            hertz,
            dampingRatio,
            enableLimit,
            lowerTranslation,
            upperTranslation,
            enableMotor,
            maxMotorTorque,
            motorSpeed,
            internalValue,
        );
    }
}
