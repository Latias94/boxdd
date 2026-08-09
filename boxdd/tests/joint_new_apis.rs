use boxdd::prelude::*;

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= 1.0e-6
}

fn approx_transform(left: Transform, right: Transform) -> bool {
    left.position() == right.position()
        && approx_eq(left.rotation().angle(), right.rotation().angle())
}

fn body_pair(world: &mut World) -> (BodyId, BodyId) {
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
    (body_a, body_b)
}

#[test]
fn joint_base_uses_upstream_defaults_and_remains_a_validated_value() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body_a, body_b) = body_pair(&mut world);
    let base = world.joint_base(body_a, body_b).unwrap();

    assert_eq!(base.body_a_id(), body_a);
    assert_eq!(base.body_b_id(), body_b);
    for frame in [base.local_frame_a(), base.local_frame_b()] {
        assert_eq!(frame.position(), Vec2::ZERO);
        assert!(approx_eq(frame.rotation().angle(), 0.0));
    }
    assert!(!base.collide_connected());
    assert_eq!(base.force_threshold(), f32::MAX);
    assert_eq!(base.torque_threshold(), f32::MAX);
    assert!(approx_eq(base.constraint_tuning().hertz(), 60.0));
    assert!(approx_eq(base.constraint_tuning().damping_ratio(), 2.0));
    base.validate().unwrap();

    let frame_a = Transform::from_pos_angle([0.25_f32, -0.5], 0.3).unwrap();
    let frame_b = Transform::from_pos_angle([1.0_f32, 2.0], -0.6).unwrap();
    let tuning = ConstraintTuning::new(4.0, 0.25).unwrap();
    let configured = base
        .with_local_frames(frame_a, frame_b)
        .with_collide_connected(true)
        .with_force_threshold(2.5)
        .unwrap()
        .with_torque_threshold(3.5)
        .unwrap()
        .with_constraint_tuning(tuning)
        .with_draw_scale(1.25)
        .unwrap();

    assert!(approx_transform(configured.local_frame_a(), frame_a));
    assert!(approx_transform(configured.local_frame_b(), frame_b));
    assert!(configured.collide_connected());
    assert_eq!(configured.force_threshold(), 2.5);
    assert_eq!(configured.torque_threshold(), 3.5);
    assert_eq!(configured.constraint_tuning(), tuning);
    assert_eq!(configured.draw_scale(), 1.25);
    configured.validate().unwrap();
}

#[test]
fn checked_joint_configuration_rejects_non_finite_and_negative_values() {
    let expected_tuning_error = Error::InvalidArgument {
        operation: "ConstraintTuning::new",
        argument: "hertz/damping_ratio",
        constraint: "finite non-negative hertz and damping ratio values",
    };
    for (hertz, damping_ratio) in [
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
        (f32::NEG_INFINITY, 0.0),
        (-1.0, 0.0),
        (0.0, f32::NAN),
        (0.0, f32::INFINITY),
        (0.0, f32::NEG_INFINITY),
        (0.0, -1.0),
    ] {
        assert_eq!(
            ConstraintTuning::new(hertz, damping_ratio).unwrap_err(),
            expected_tuning_error
        );
    }

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body_a, body_b) = body_pair(&mut world);
    let base = world.joint_base(body_a, body_b).unwrap();

    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
        assert_eq!(
            base.with_force_threshold(value).unwrap_err(),
            Error::InvalidArgument {
                operation: "JointBase::with_force_threshold",
                argument: "force_threshold",
                constraint: "a finite non-negative value",
            }
        );
        assert_eq!(
            base.with_torque_threshold(value).unwrap_err(),
            Error::InvalidArgument {
                operation: "JointBase::with_torque_threshold",
                argument: "torque_threshold",
                constraint: "a finite non-negative value",
            }
        );
        assert_eq!(
            base.with_draw_scale(value).unwrap_err(),
            Error::InvalidArgument {
                operation: "JointBase::with_draw_scale",
                argument: "draw_scale",
                constraint: "a finite non-negative value",
            }
        );
    }
}

#[test]
fn every_joint_definition_is_a_readable_validated_value() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body_a, body_b) = body_pair(&mut world);

    let distance = DistanceJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .length(3.5)
        .enable_spring(true)
        .hertz(5.0)
        .damping_ratio(0.6)
        .enable_limit(true)
        .min_length(1.5)
        .max_length(6.5)
        .enable_motor(true)
        .max_motor_force(9.0)
        .motor_speed(-2.0);
    assert_eq!(distance.target_length(), 3.5);
    assert!(distance.spring_enabled());
    assert_eq!(distance.spring_hertz(), 5.0);
    assert_eq!(distance.maximum_length(), 6.5);
    distance.validate().unwrap();

    let prismatic = PrismaticJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .enable_spring(true)
        .hertz(7.0)
        .damping_ratio(0.4)
        .translation(0.125)
        .lower_translation(-0.25)
        .upper_translation(0.75)
        .enable_limit(true)
        .enable_motor(true)
        .max_motor_force(11.0)
        .motor_speed(1.5);
    assert_eq!(prismatic.target_translation(), 0.125);
    assert_eq!(prismatic.maximum_motor_force(), 11.0);
    prismatic.validate().unwrap();

    let revolute = RevoluteJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .target_angle(0.2)
        .enable_spring(true)
        .hertz(8.0)
        .damping_ratio(0.3)
        .enable_limit(true)
        .lower_angle(-0.5)
        .upper_angle(0.5)
        .enable_motor(true)
        .max_motor_torque(12.0)
        .motor_speed(0.9);
    assert_eq!(revolute.target_angle_value(), 0.2);
    assert_eq!(revolute.maximum_motor_torque(), 12.0);
    revolute.validate().unwrap();

    let weld = WeldJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .linear_hertz(3.0)
        .angular_hertz(4.0)
        .linear_damping_ratio(0.2)
        .angular_damping_ratio(0.7);
    assert_eq!(weld.configured_linear_hertz(), 3.0);
    assert_eq!(weld.configured_angular_damping_ratio(), 0.7);
    weld.validate().unwrap();

    let wheel = WheelJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .enable_spring(true)
        .hertz(6.0)
        .damping_ratio(0.5)
        .enable_limit(true)
        .lower_translation(-0.2)
        .upper_translation(0.4)
        .enable_motor(true)
        .max_motor_torque(7.0)
        .motor_speed(-1.25);
    assert_eq!(wheel.spring_hertz(), 6.0);
    assert_eq!(wheel.maximum_motor_torque(), 7.0);
    wheel.validate().unwrap();

    let motor = MotorJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .linear_velocity([2.0_f32, -1.0])
        .max_velocity_force(5.0)
        .angular_velocity(0.75)
        .max_velocity_torque(6.0)
        .linear_hertz(2.5)
        .linear_damping_ratio(0.15)
        .max_spring_force(8.0)
        .angular_hertz(3.5)
        .angular_damping_ratio(0.45)
        .max_spring_torque(9.0);
    assert_eq!(motor.target_linear_velocity(), Vec2::new(2.0, -1.0));
    assert_eq!(motor.maximum_spring_torque(), 9.0);
    motor.validate().unwrap();

    FilterJointDef::new(world.joint_base(body_a, body_b).unwrap())
        .validate()
        .unwrap();
}

#[test]
fn scoped_joint_metadata_and_mutation_use_the_capability_contract() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body_a, body_b) = body_pair(&mut world);
    let frame_a = Transform::from_pos_angle([0.25_f32, -0.5], 0.3).unwrap();
    let frame_b = Transform::from_pos_angle([1.0_f32, 2.0], -0.6).unwrap();
    let initial_tuning = ConstraintTuning::new(4.0, 0.25).unwrap();
    let base = world
        .joint_base(body_a, body_b)
        .unwrap()
        .with_local_frames(frame_a, frame_b)
        .with_collide_connected(true)
        .with_force_threshold(2.5)
        .unwrap()
        .with_torque_threshold(3.5)
        .unwrap()
        .with_constraint_tuning(initial_tuning);
    let joint_id = world
        .create_distance_joint(&DistanceJointDef::new(base).length(3.5))
        .unwrap();
    {
        let mut joint = world.joint(joint_id).unwrap();
        assert_eq!(joint.id(), joint_id);
        assert_eq!(joint.joint_type().unwrap(), JointType::Distance);
        assert_eq!(joint.body_a_id().unwrap(), body_a);
        assert_eq!(joint.body_b_id().unwrap(), body_b);
        assert!(joint.collide_connected().unwrap());
        assert_eq!(joint.constraint_tuning().unwrap(), initial_tuning);
        assert!(approx_transform(joint.local_frame_a().unwrap(), frame_a));
        assert!(approx_transform(joint.local_frame_b().unwrap(), frame_b));

        joint
            .set_constraint_tuning(ConstraintTuning::new(8.0, 0.75).unwrap())
            .unwrap();
        joint.set_force_threshold(6.0).unwrap();
        joint.set_torque_threshold(7.0).unwrap();

        let mut joint = joint.into_distance().unwrap();
        joint.set_length(4.0).unwrap();
        assert_eq!(joint.length().unwrap(), 4.0);
        assert_eq!(joint.joint_type().unwrap(), JointType::Distance);
    }

    let joint = world.joint(joint_id).unwrap();
    assert_eq!(
        joint.constraint_tuning().unwrap(),
        ConstraintTuning::new(8.0, 0.75).unwrap()
    );
    assert_eq!(joint.force_threshold().unwrap(), 6.0);
    assert_eq!(joint.torque_threshold().unwrap(), 7.0);
}
