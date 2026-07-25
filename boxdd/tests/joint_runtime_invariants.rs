use boxdd::{ApiError, ApiResult, prelude::*};
use std::collections::BTreeMap;

fn world_and_base() -> (World, JointBase) {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );
    (world, JointBase::new(body_a, body_b))
}

fn assert_invalid(result: ApiResult<()>) {
    assert_eq!(result, Err(ApiError::InvalidArgument));
}

#[test]
fn six_joint_families_have_explicit_receiver_parity() {
    let (mut world, base) = world_and_base();
    let mut distance = world.create_distance_joint_owned(&DistanceJointDef::new(base).length(1.0));
    let mut motor = world.create_motor_joint_owned(&MotorJointDef::new(base));
    let mut prismatic = world.create_prismatic_joint_owned(&PrismaticJointDef::new(base));
    let mut revolute = world.create_revolute_joint_owned(&RevoluteJointDef::new(base));
    let mut weld = world.create_weld_joint_owned(&WeldJointDef::new(base));
    let mut wheel = world.create_wheel_joint_owned(&WheelJointDef::new(base));
    let handle = world.handle();

    assert_eq!(
        JointType::from_raw(handle.joint_type_raw(distance.id())),
        Some(JointType::Distance)
    );
    assert_eq!(
        JointType::from_raw(handle.try_joint_type_raw(distance.id()).unwrap()),
        Some(JointType::Distance)
    );

    world.try_distance_set_length(distance.id(), 2.0).unwrap();
    assert_eq!(handle.try_distance_length(distance.id()).unwrap(), 2.0);
    assert_eq!(distance.try_distance_length().unwrap(), 2.0);
    {
        let scoped = world.joint(distance.id()).unwrap();
        assert_eq!(scoped.try_distance_length().unwrap(), 2.0);
    }
    distance.try_distance_set_length(2.5).unwrap();
    assert_eq!(world.try_distance_length(distance.id()).unwrap(), 2.5);
    {
        let mut scoped = world.joint(distance.id()).unwrap();
        scoped.try_distance_set_length(3.0).unwrap();
    }
    assert_eq!(handle.try_distance_length(distance.id()).unwrap(), 3.0);

    world.try_motor_set_linear_hertz(motor.id(), 2.0).unwrap();
    assert_eq!(handle.try_motor_linear_hertz(motor.id()).unwrap(), 2.0);
    assert_eq!(motor.try_motor_linear_hertz().unwrap(), 2.0);
    {
        let scoped = world.joint(motor.id()).unwrap();
        assert_eq!(scoped.try_motor_linear_hertz().unwrap(), 2.0);
    }
    motor.try_motor_set_linear_hertz(2.5).unwrap();
    {
        let mut scoped = world.joint(motor.id()).unwrap();
        scoped.try_motor_set_linear_hertz(3.0).unwrap();
    }
    assert_eq!(world.try_motor_linear_hertz(motor.id()).unwrap(), 3.0);

    world
        .try_prismatic_set_target_translation(prismatic.id(), 0.25)
        .unwrap();
    assert_eq!(
        handle
            .try_prismatic_target_translation(prismatic.id())
            .unwrap(),
        0.25
    );
    assert_eq!(prismatic.try_prismatic_target_translation().unwrap(), 0.25);
    {
        let scoped = world.joint(prismatic.id()).unwrap();
        assert_eq!(scoped.try_prismatic_target_translation().unwrap(), 0.25);
    }
    prismatic.try_prismatic_set_target_translation(0.5).unwrap();
    {
        let mut scoped = world.joint(prismatic.id()).unwrap();
        scoped.try_prismatic_set_target_translation(0.75).unwrap();
    }
    assert_eq!(
        world
            .try_prismatic_target_translation(prismatic.id())
            .unwrap(),
        0.75
    );

    world
        .try_revolute_set_target_angle(revolute.id(), 0.1)
        .unwrap();
    assert_eq!(
        handle.try_revolute_target_angle(revolute.id()).unwrap(),
        0.1
    );
    assert_eq!(revolute.try_revolute_target_angle().unwrap(), 0.1);
    {
        let scoped = world.joint(revolute.id()).unwrap();
        assert_eq!(scoped.try_revolute_target_angle().unwrap(), 0.1);
    }
    revolute.try_revolute_set_target_angle(0.2).unwrap();
    {
        let mut scoped = world.joint(revolute.id()).unwrap();
        scoped.try_revolute_set_target_angle(0.3).unwrap();
    }
    assert_eq!(world.try_revolute_target_angle(revolute.id()).unwrap(), 0.3);

    world.try_weld_set_linear_hertz(weld.id(), 2.0).unwrap();
    assert_eq!(handle.try_weld_linear_hertz(weld.id()).unwrap(), 2.0);
    assert_eq!(weld.try_weld_linear_hertz().unwrap(), 2.0);
    {
        let scoped = world.joint(weld.id()).unwrap();
        assert_eq!(scoped.try_weld_linear_hertz().unwrap(), 2.0);
    }
    weld.try_weld_set_linear_hertz(2.5).unwrap();
    {
        let mut scoped = world.joint(weld.id()).unwrap();
        scoped.try_weld_set_linear_hertz(3.0).unwrap();
    }
    assert_eq!(world.try_weld_linear_hertz(weld.id()).unwrap(), 3.0);

    world.try_wheel_set_motor_speed(wheel.id(), -1.0).unwrap();
    assert_eq!(handle.try_wheel_motor_speed(wheel.id()).unwrap(), -1.0);
    assert_eq!(wheel.try_wheel_motor_speed().unwrap(), -1.0);
    {
        let scoped = world.joint(wheel.id()).unwrap();
        assert_eq!(scoped.try_wheel_motor_speed().unwrap(), -1.0);
    }
    wheel.try_wheel_set_motor_speed(-2.0).unwrap();
    {
        let mut scoped = world.joint(wheel.id()).unwrap();
        scoped.try_wheel_set_motor_speed(-3.0).unwrap();
    }
    assert_eq!(world.try_wheel_motor_speed(wheel.id()).unwrap(), -3.0);
}

#[test]
fn every_numeric_joint_operation_rejects_invalid_values_before_native_mutation() {
    let (mut world, base) = world_and_base();
    let mut distance = world.create_distance_joint_owned(&DistanceJointDef::new(base).length(1.0));
    let mut motor = world.create_motor_joint_owned(&MotorJointDef::new(base));
    let mut prismatic = world.create_prismatic_joint_owned(&PrismaticJointDef::new(base));
    let mut revolute = world.create_revolute_joint_owned(&RevoluteJointDef::new(base));
    let mut weld = world.create_weld_joint_owned(&WeldJointDef::new(base));
    let mut wheel = world.create_wheel_joint_owned(&WheelJointDef::new(base));

    assert_invalid(distance.try_set_constraint_tuning(ConstraintTuning::new(f32::NAN, 0.0)));
    assert_invalid(distance.try_set_constraint_tuning(ConstraintTuning::new(0.0, -1.0)));
    assert_invalid(distance.try_set_force_threshold(f32::INFINITY));
    assert_invalid(distance.try_set_torque_threshold(-1.0));
    assert_invalid(distance.try_set_local_frame_a(Transform::from_pos_angle([f32::NAN, 0.0], 0.0)));

    let original_length = distance.distance_length();
    assert_invalid(distance.try_distance_set_length(f32::NAN));
    assert_invalid(distance.try_distance_set_length(f32::INFINITY));
    assert_invalid(distance.try_distance_set_length(0.0));
    assert_invalid(distance.try_distance_set_spring_force_range(f32::NAN, 1.0));
    assert_invalid(distance.try_distance_set_spring_force_range(2.0, 1.0));
    assert_invalid(distance.try_distance_set_spring_hertz(-1.0));
    assert_invalid(distance.try_distance_set_spring_hertz(f32::INFINITY));
    assert_invalid(distance.try_distance_set_spring_damping_ratio(f32::NAN));
    assert_invalid(distance.try_distance_set_spring_damping_ratio(-1.0));
    assert_invalid(distance.try_distance_set_length_range(-1.0, 1.0));
    assert_invalid(distance.try_distance_set_length_range(2.0, 1.0));
    assert_invalid(distance.try_distance_set_length_range(0.0, f32::INFINITY));
    assert_invalid(distance.try_distance_set_motor_speed(f32::NAN));
    assert_invalid(distance.try_distance_set_motor_speed(f32::INFINITY));
    assert_invalid(distance.try_distance_set_max_motor_force(-1.0));
    assert_invalid(distance.try_distance_set_max_motor_force(f32::INFINITY));
    assert_eq!(distance.distance_length(), original_length);

    assert_invalid(motor.try_motor_set_linear_velocity([f32::NAN, 0.0]));
    assert_invalid(motor.try_motor_set_linear_velocity([f32::INFINITY, 0.0]));
    assert_invalid(motor.try_motor_set_angular_velocity(f32::NAN));
    assert_invalid(motor.try_motor_set_angular_velocity(f32::INFINITY));
    assert_invalid(motor.try_motor_set_max_velocity_force(-1.0));
    assert_invalid(motor.try_motor_set_max_velocity_torque(f32::INFINITY));
    assert_invalid(motor.try_motor_set_linear_hertz(-1.0));
    assert_invalid(motor.try_motor_set_linear_damping_ratio(f32::NAN));
    assert_invalid(motor.try_motor_set_angular_hertz(f32::INFINITY));
    assert_invalid(motor.try_motor_set_angular_damping_ratio(-1.0));
    assert_invalid(motor.try_motor_set_max_spring_force(f32::NAN));
    assert_invalid(motor.try_motor_set_max_spring_torque(-1.0));

    assert_invalid(prismatic.try_prismatic_set_spring_hertz(f32::NAN));
    assert_invalid(prismatic.try_prismatic_set_spring_damping_ratio(-1.0));
    assert_invalid(prismatic.try_prismatic_set_target_translation(f32::INFINITY));
    assert_invalid(prismatic.try_prismatic_set_limits(f32::NAN, 1.0));
    assert_invalid(prismatic.try_prismatic_set_limits(2.0, 1.0));
    assert_invalid(prismatic.try_prismatic_set_motor_speed(f32::NAN));
    assert_invalid(prismatic.try_prismatic_set_max_motor_force(-1.0));

    assert_invalid(revolute.try_revolute_set_spring_hertz(-1.0));
    assert_invalid(revolute.try_revolute_set_spring_damping_ratio(f32::NAN));
    assert_invalid(revolute.try_revolute_set_target_angle(f32::INFINITY));
    assert_invalid(revolute.try_revolute_set_limits(f32::NAN, 0.0));
    assert_invalid(revolute.try_revolute_set_limits(1.0, -1.0));
    assert_invalid(revolute.try_revolute_set_limits(-4.0, 0.0));
    assert_invalid(revolute.try_revolute_set_motor_speed(f32::NAN));
    assert_invalid(revolute.try_revolute_set_max_motor_torque(-1.0));

    assert_invalid(weld.try_weld_set_linear_hertz(f32::NAN));
    assert_invalid(weld.try_weld_set_linear_damping_ratio(-1.0));
    assert_invalid(weld.try_weld_set_angular_hertz(f32::INFINITY));
    assert_invalid(weld.try_weld_set_angular_damping_ratio(-1.0));

    assert_invalid(wheel.try_wheel_set_spring_hertz(f32::NAN));
    assert_invalid(wheel.try_wheel_set_spring_damping_ratio(-1.0));
    assert_invalid(wheel.try_wheel_set_limits(f32::NAN, 1.0));
    assert_invalid(wheel.try_wheel_set_limits(1.0, -1.0));
    assert_invalid(wheel.try_wheel_set_motor_speed(f32::INFINITY));
    assert_invalid(wheel.try_wheel_set_max_motor_torque(-1.0));
}

#[test]
fn valid_numeric_boundaries_reach_native() {
    let (mut world, base) = world_and_base();
    let mut distance = world.create_distance_joint_owned(&DistanceJointDef::new(base).length(1.0));
    let mut motor = world.create_motor_joint_owned(&MotorJointDef::new(base));
    let mut prismatic = world.create_prismatic_joint_owned(&PrismaticJointDef::new(base));
    let mut revolute = world.create_revolute_joint_owned(&RevoluteJointDef::new(base));
    let mut weld = world.create_weld_joint_owned(&WeldJointDef::new(base));
    let mut wheel = world.create_wheel_joint_owned(&WheelJointDef::new(base));

    distance
        .try_set_constraint_tuning(ConstraintTuning::new(0.0, 0.0))
        .unwrap();
    distance.try_set_force_threshold(0.0).unwrap();
    distance.try_set_torque_threshold(0.0).unwrap();
    distance
        .try_distance_set_spring_force_range(0.0, 0.0)
        .unwrap();
    distance.try_distance_set_spring_hertz(0.0).unwrap();
    distance.try_distance_set_spring_damping_ratio(0.0).unwrap();
    distance.try_distance_set_length_range(0.0, 0.0).unwrap();
    distance.try_distance_set_motor_speed(-1.0).unwrap();
    distance.try_distance_set_max_motor_force(0.0).unwrap();

    motor.try_motor_set_linear_velocity([0.0, 0.0]).unwrap();
    motor.try_motor_set_angular_velocity(-1.0).unwrap();
    motor.try_motor_set_max_velocity_force(0.0).unwrap();
    motor.try_motor_set_max_velocity_torque(0.0).unwrap();
    motor.try_motor_set_linear_hertz(0.0).unwrap();
    motor.try_motor_set_linear_damping_ratio(0.0).unwrap();
    motor.try_motor_set_angular_hertz(0.0).unwrap();
    motor.try_motor_set_angular_damping_ratio(0.0).unwrap();
    motor.try_motor_set_max_spring_force(0.0).unwrap();
    motor.try_motor_set_max_spring_torque(0.0).unwrap();

    prismatic.try_prismatic_set_spring_hertz(0.0).unwrap();
    prismatic
        .try_prismatic_set_spring_damping_ratio(0.0)
        .unwrap();
    prismatic
        .try_prismatic_set_target_translation(-1.0)
        .unwrap();
    prismatic.try_prismatic_set_limits(0.0, 0.0).unwrap();
    prismatic.try_prismatic_set_motor_speed(-1.0).unwrap();
    prismatic.try_prismatic_set_max_motor_force(0.0).unwrap();

    revolute.try_revolute_set_spring_hertz(0.0).unwrap();
    revolute.try_revolute_set_spring_damping_ratio(0.0).unwrap();
    revolute.try_revolute_set_target_angle(-1.0).unwrap();
    revolute.try_revolute_set_limits(0.0, 0.0).unwrap();
    revolute.try_revolute_set_motor_speed(-1.0).unwrap();
    revolute.try_revolute_set_max_motor_torque(0.0).unwrap();

    weld.try_weld_set_linear_hertz(0.0).unwrap();
    weld.try_weld_set_linear_damping_ratio(0.0).unwrap();
    weld.try_weld_set_angular_hertz(0.0).unwrap();
    weld.try_weld_set_angular_damping_ratio(0.0).unwrap();

    wheel.try_wheel_set_spring_hertz(0.0).unwrap();
    wheel.try_wheel_set_spring_damping_ratio(0.0).unwrap();
    wheel.try_wheel_set_limits(0.0, 0.0).unwrap();
    wheel.try_wheel_set_motor_speed(-1.0).unwrap();
    wheel.try_wheel_set_max_motor_torque(0.0).unwrap();

    assert_eq!(distance.distance_spring_hertz(), 0.0);
    assert_eq!(motor.motor_max_spring_torque(), 0.0);
    assert_eq!(prismatic.prismatic_target_translation(), -1.0);
    assert_eq!(revolute.revolute_target_angle(), -1.0);
    assert_eq!(weld.weld_angular_hertz(), 0.0);
    assert_eq!(wheel.wheel_motor_speed(), -1.0);
}

#[test]
fn infallible_joint_mutations_panic_before_native_state_changes() {
    let (mut world, base) = world_and_base();
    let mut distance = world.create_distance_joint_owned(&DistanceJointDef::new(base).length(1.0));
    let original_length = distance.distance_length();
    let original_threshold = distance.force_threshold();

    let bad_length = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        distance.distance_set_length(f32::NAN);
    }));
    assert!(bad_length.is_err());
    assert_eq!(distance.distance_length(), original_length);

    let bad_threshold = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.set_joint_force_threshold(distance.id(), -1.0);
    }));
    assert!(bad_threshold.is_err());
    assert_eq!(distance.force_threshold(), original_threshold);
}

#[test]
fn typed_joint_identity_errors_are_consistent_before_kind_dispatch() {
    let (mut source, base) = world_and_base();
    let mut distance = source.create_distance_joint_owned(&DistanceJointDef::new(base).length(1.0));
    let motor = source.create_motor_joint_owned(&MotorJointDef::new(base));
    let prismatic = source.create_prismatic_joint_owned(&PrismaticJointDef::new(base));
    let revolute = source.create_revolute_joint_owned(&RevoluteJointDef::new(base));
    let weld = source.create_weld_joint_owned(&WeldJointDef::new(base));
    let wheel = source.create_wheel_joint_owned(&WheelJointDef::new(base));
    let source_handle = source.handle();
    let (mut target, _) = world_and_base();
    let target_handle = target.handle();

    assert_eq!(
        target.try_distance_length(distance.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_distance_length(distance.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_distance_set_length(distance.id(), f32::NAN),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_motor_linear_velocity(motor.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_motor_linear_velocity(motor.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_prismatic_spring_enabled(prismatic.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_prismatic_spring_enabled(prismatic.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_revolute_angle(revolute.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_revolute_angle(revolute.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_weld_linear_hertz(weld.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_weld_linear_hertz(weld.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target.try_wheel_spring_enabled(wheel.id()),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        target_handle.try_wheel_spring_enabled(wheel.id()),
        Err(ApiError::WrongWorld)
    );

    assert_eq!(
        source.try_distance_length(motor.id()),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        source_handle.try_distance_length(motor.id()),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(motor.try_distance_length(), Err(ApiError::InvalidJointType));
    let mut motor = motor;
    assert_eq!(
        motor.try_distance_set_length(f32::NAN),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        distance.try_motor_linear_velocity(),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        distance.try_prismatic_spring_enabled(),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        distance.try_revolute_angle(),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        distance.try_weld_linear_hertz(),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        distance.try_wheel_spring_enabled(),
        Err(ApiError::InvalidJointType)
    );
    {
        let scoped = source.joint(motor.id()).unwrap();
        assert_eq!(
            scoped.try_distance_length(),
            Err(ApiError::InvalidJointType)
        );
    }

    let stale = distance.id();
    source.destroy_joint_id(stale, true);
    assert_eq!(
        source.try_distance_length(stale),
        Err(ApiError::InvalidJointId)
    );
    assert_eq!(
        source_handle.try_distance_length(stale),
        Err(ApiError::InvalidJointId)
    );
    assert_eq!(
        distance.try_distance_length(),
        Err(ApiError::InvalidJointId)
    );
    assert!(source.joint(stale).is_none());
    assert_eq!(
        distance.try_distance_set_length(f32::NAN),
        Err(ApiError::InvalidJointId)
    );
}

#[test]
fn each_typed_joint_ffi_operation_has_one_private_implementation() {
    let sources = [
        include_str!("../src/joints/base.rs"),
        include_str!("../src/joints/runtime.rs"),
        include_str!("../src/joints/runtime_typed_distance.rs"),
        include_str!("../src/joints/runtime_typed_motor.rs"),
        include_str!("../src/joints/runtime_typed_prismatic.rs"),
        include_str!("../src/joints/runtime_typed_revolute.rs"),
        include_str!("../src/joints/runtime_typed_weld.rs"),
        include_str!("../src/joints/runtime_typed_wheel.rs"),
    ];
    let mut occurrences = BTreeMap::<&str, usize>::new();

    for source in sources {
        for suffix in source.split("ffi::").skip(1) {
            let symbol = suffix
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()
                .unwrap();
            if symbol.contains("Joint_") {
                *occurrences.entry(symbol).or_default() += 1;
            }
        }
    }

    assert!(!occurrences.is_empty());
    for (symbol, count) in occurrences {
        assert_eq!(
            count, 1,
            "{symbol} must have one private FFI implementation"
        );
    }
}
