use std::collections::BTreeMap;

use boxdd::{Error, Result, prelude::*};

fn world_and_base() -> (World, JointBase) {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([-1.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let base = world.joint_base(body_a, body_b).unwrap();
    (world, base)
}

fn assert_invalid(result: Result<()>, expected: Error) {
    assert_eq!(result, Err(expected));
}

#[test]
fn cached_kinds_dispatch_every_typed_joint_capability() {
    let (mut world, base) = world_and_base();
    let distance = world
        .create_distance_joint(&DistanceJointDef::new(base).length(2.0))
        .unwrap();
    let motor = world.create_motor_joint(&MotorJointDef::new(base)).unwrap();
    let filter = world
        .create_filter_joint(&FilterJointDef::new(base))
        .unwrap();
    let prismatic = world
        .create_prismatic_joint(&PrismaticJointDef::new(base))
        .unwrap();
    let revolute = world
        .create_revolute_joint(&RevoluteJointDef::new(base))
        .unwrap();
    let weld = world.create_weld_joint(&WeldJointDef::new(base)).unwrap();
    let wheel = world.create_wheel_joint(&WheelJointDef::new(base)).unwrap();

    {
        let mut joint = world.joint(distance).unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Distance);
        assert_eq!(joint.id(), distance);
        joint.set_collide_connected(true).unwrap();
        joint
            .set_constraint_tuning(ConstraintTuning::new(4.0, 0.5).unwrap())
            .unwrap();
        joint.set_force_threshold(3.0).unwrap();
        joint.set_torque_threshold(5.0).unwrap();
        assert!(joint.collide_connected().unwrap());
        assert_eq!(joint.constraint_tuning().unwrap().hertz(), 4.0);
        assert_eq!(joint.force_threshold().unwrap(), 3.0);
        assert_eq!(joint.torque_threshold().unwrap(), 5.0);

        let mut joint = joint.into_distance().unwrap();
        joint.set_length(2.5).unwrap();
        joint.enable_spring(true).unwrap();
        joint.set_spring_force_range(-1.0, 6.0).unwrap();
        joint.set_spring_hertz(3.0).unwrap();
        joint.set_spring_damping_ratio(0.25).unwrap();
        joint.enable_limit(true).unwrap();
        joint.set_length_range(0.5, 4.0).unwrap();
        joint.enable_motor(true).unwrap();
        joint.set_motor_speed(-2.0).unwrap();
        joint.set_max_motor_force(8.0).unwrap();

        assert_eq!(joint.length().unwrap(), 2.5);
        assert!(joint.spring_enabled().unwrap());
        assert_eq!(joint.spring_force_range().unwrap(), (-1.0, 6.0));
        assert_eq!(joint.spring_hertz().unwrap(), 3.0);
        assert_eq!(joint.spring_damping_ratio().unwrap(), 0.25);
        assert!(joint.limit_enabled().unwrap());
        assert_eq!(joint.min_length().unwrap(), 0.5);
        assert_eq!(joint.max_length().unwrap(), 4.0);
        assert!(joint.motor_enabled().unwrap());
        assert_eq!(joint.motor_speed().unwrap(), -2.0);
        assert_eq!(joint.max_motor_force().unwrap(), 8.0);
    }

    {
        let mut joint = world.joint(motor).unwrap().into_motor().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Motor);
        joint.set_linear_velocity([1.5_f32, -0.5]).unwrap();
        joint.set_angular_velocity(-0.75).unwrap();
        joint.set_max_velocity_force(3.0).unwrap();
        joint.set_max_velocity_torque(4.0).unwrap();
        joint.set_linear_hertz(2.0).unwrap();
        joint.set_linear_damping_ratio(0.2).unwrap();
        joint.set_angular_hertz(5.0).unwrap();
        joint.set_angular_damping_ratio(0.4).unwrap();
        joint.set_max_spring_force(6.0).unwrap();
        joint.set_max_spring_torque(7.0).unwrap();

        assert_eq!(joint.linear_velocity().unwrap(), Vec2::new(1.5, -0.5));
        assert_eq!(joint.angular_velocity().unwrap(), -0.75);
        assert_eq!(joint.max_velocity_force().unwrap(), 3.0);
        assert_eq!(joint.max_velocity_torque().unwrap(), 4.0);
        assert_eq!(joint.linear_hertz().unwrap(), 2.0);
        assert_eq!(joint.linear_damping_ratio().unwrap(), 0.2);
        assert_eq!(joint.angular_hertz().unwrap(), 5.0);
        assert_eq!(joint.angular_damping_ratio().unwrap(), 0.4);
        assert_eq!(joint.max_spring_force().unwrap(), 6.0);
        assert_eq!(joint.max_spring_torque().unwrap(), 7.0);
    }

    {
        let joint = world.joint(filter).unwrap().into_filter().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Filter);
        assert_eq!(joint.id(), filter);
    }

    {
        let mut joint = world.joint(prismatic).unwrap().into_prismatic().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Prismatic);
        joint.enable_spring(true).unwrap();
        joint.set_spring_hertz(3.0).unwrap();
        joint.set_spring_damping_ratio(0.3).unwrap();
        joint.set_target_translation(0.75).unwrap();
        joint.enable_limit(true).unwrap();
        joint.set_limits(-1.0, 1.0).unwrap();
        joint.enable_motor(true).unwrap();
        joint.set_motor_speed(-1.5).unwrap();
        joint.set_max_motor_force(9.0).unwrap();

        assert!(joint.spring_enabled().unwrap());
        assert_eq!(joint.spring_hertz().unwrap(), 3.0);
        assert_eq!(joint.spring_damping_ratio().unwrap(), 0.3);
        assert_eq!(joint.target_translation().unwrap(), 0.75);
        assert!(joint.limit_enabled().unwrap());
        assert_eq!(joint.lower_limit().unwrap(), -1.0);
        assert_eq!(joint.upper_limit().unwrap(), 1.0);
        assert!(joint.motor_enabled().unwrap());
        assert_eq!(joint.motor_speed().unwrap(), -1.5);
        assert_eq!(joint.max_motor_force().unwrap(), 9.0);
    }

    {
        let mut joint = world.joint(revolute).unwrap().into_revolute().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Revolute);
        joint.enable_spring(true).unwrap();
        joint.set_spring_hertz(4.0).unwrap();
        joint.set_spring_damping_ratio(0.4).unwrap();
        joint.set_target_angle(0.25).unwrap();
        joint.enable_limit(true).unwrap();
        joint.set_limits(-0.5, 0.5).unwrap();
        joint.enable_motor(true).unwrap();
        joint.set_motor_speed(1.25).unwrap();
        joint.set_max_motor_torque(10.0).unwrap();

        assert!(joint.spring_enabled().unwrap());
        assert_eq!(joint.spring_hertz().unwrap(), 4.0);
        assert_eq!(joint.spring_damping_ratio().unwrap(), 0.4);
        assert_eq!(joint.target_angle().unwrap(), 0.25);
        assert!(joint.limit_enabled().unwrap());
        assert_eq!(joint.lower_limit().unwrap(), -0.5);
        assert_eq!(joint.upper_limit().unwrap(), 0.5);
        assert!(joint.motor_enabled().unwrap());
        assert_eq!(joint.motor_speed().unwrap(), 1.25);
        assert_eq!(joint.max_motor_torque().unwrap(), 10.0);
    }

    {
        let mut joint = world.joint(weld).unwrap().into_weld().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Weld);
        joint.set_linear_hertz(2.0).unwrap();
        joint.set_linear_damping_ratio(0.2).unwrap();
        joint.set_angular_hertz(3.0).unwrap();
        joint.set_angular_damping_ratio(0.3).unwrap();

        assert_eq!(joint.linear_hertz().unwrap(), 2.0);
        assert_eq!(joint.linear_damping_ratio().unwrap(), 0.2);
        assert_eq!(joint.angular_hertz().unwrap(), 3.0);
        assert_eq!(joint.angular_damping_ratio().unwrap(), 0.3);
    }

    {
        let mut joint = world.joint(wheel).unwrap().into_wheel().unwrap();
        assert_eq!(joint.joint_type().unwrap(), JointType::Wheel);
        joint.enable_spring(true).unwrap();
        joint.set_spring_hertz(5.0).unwrap();
        joint.set_spring_damping_ratio(0.5).unwrap();
        joint.enable_limit(true).unwrap();
        joint.set_limits(-0.75, 0.75).unwrap();
        joint.enable_motor(true).unwrap();
        joint.set_motor_speed(-2.5).unwrap();
        joint.set_max_motor_torque(11.0).unwrap();

        assert!(joint.spring_enabled().unwrap());
        assert_eq!(joint.spring_hertz().unwrap(), 5.0);
        assert_eq!(joint.spring_damping_ratio().unwrap(), 0.5);
        assert!(joint.limit_enabled().unwrap());
        assert_eq!(joint.lower_limit().unwrap(), -0.75);
        assert_eq!(joint.upper_limit().unwrap(), 0.75);
        assert!(joint.motor_enabled().unwrap());
        assert_eq!(joint.motor_speed().unwrap(), -2.5);
        assert_eq!(joint.max_motor_torque().unwrap(), 11.0);
    }
}

#[test]
fn invalid_joint_writes_do_not_mutate_native_state() {
    let (mut world, base) = world_and_base();
    let distance = world
        .create_distance_joint(&DistanceJointDef::new(base).length(2.0))
        .unwrap();
    let motor = world.create_motor_joint(&MotorJointDef::new(base)).unwrap();
    let revolute = world
        .create_revolute_joint(&RevoluteJointDef::new(base))
        .unwrap();

    {
        let mut joint = world.joint(distance).unwrap().into_distance().unwrap();
        let original_length = joint.length().unwrap();
        let invalid_length = Error::invalid_argument(
            "DistanceJoint::set_length",
            "length",
            "a finite positive value",
        );
        assert_invalid(joint.set_length(f32::NAN), invalid_length);
        assert_invalid(joint.set_length(0.0), invalid_length);
        assert_invalid(
            joint.set_spring_hertz(-1.0),
            Error::invalid_argument(
                "DistanceJoint::set_spring_hertz",
                "hertz",
                "a finite non-negative value",
            ),
        );
        assert_invalid(
            joint.set_length_range(2.0, 1.0),
            Error::invalid_argument(
                "DistanceJoint::set_length_range",
                "min/max",
                "finite values ordered 0 <= lower <= upper",
            ),
        );
        assert_eq!(joint.length().unwrap(), original_length);
    }

    {
        let mut joint = world.joint(motor).unwrap().into_motor().unwrap();
        let original_velocity = joint.linear_velocity().unwrap();
        assert_invalid(
            joint.set_linear_velocity([f32::NAN, 0.0]),
            Error::invalid_argument(
                "MotorJoint::set_linear_velocity",
                "velocity",
                "finite vector components",
            ),
        );
        assert_invalid(
            joint.set_max_velocity_force(-1.0),
            Error::invalid_argument(
                "MotorJoint::set_max_velocity_force",
                "force",
                "a finite non-negative value",
            ),
        );
        assert_eq!(joint.linear_velocity().unwrap(), original_velocity);
    }

    {
        let mut joint = world.joint(revolute).unwrap().into_revolute().unwrap();
        let original_limits = (joint.lower_limit().unwrap(), joint.upper_limit().unwrap());
        let invalid_limits = Error::invalid_argument(
            "RevoluteJoint::set_limits",
            "lower/upper",
            "finite ordered angles within the supported revolute limit",
        );
        assert_invalid(joint.set_limits(1.0, -1.0), invalid_limits);
        assert_invalid(joint.set_limits(-4.0, 0.0), invalid_limits);
        assert_eq!(
            (joint.lower_limit().unwrap(), joint.upper_limit().unwrap()),
            original_limits
        );
    }
}

#[test]
fn acquisition_and_typed_conversion_report_identity_errors() {
    let (mut source, base) = world_and_base();
    let distance = source
        .create_distance_joint(&DistanceJointDef::new(base).length(1.0))
        .unwrap();
    let motor = source
        .create_motor_joint(&MotorJointDef::new(base))
        .unwrap();
    let (mut target, _) = world_and_base();

    assert_eq!(target.joint(distance).err(), Some(Error::WrongWorld));
    assert_eq!(
        source.joint(motor).unwrap().into_distance().err(),
        Some(Error::WrongJointType {
            expected: JointType::Distance,
            actual: JointType::Motor,
        })
    );

    source.joint(distance).unwrap().destroy(true).unwrap();
    assert_eq!(source.joint(distance).err(), Some(Error::InvalidJointId));
}

#[test]
fn each_joint_runtime_ffi_operation_has_one_private_implementation() {
    let sources = [
        include_str!("../src/joints/base.rs"),
        include_str!("../src/joints/runtime.rs"),
        include_str!("../src/joints/typed.rs"),
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
        assert_eq!(count, 1, "{symbol} must have one private implementation");
    }
}
