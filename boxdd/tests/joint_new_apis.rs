use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn approx_vec2(a: Vec2, b: Vec2, eps: f32) -> bool {
    approx_eq(a.x, b.x, eps) && approx_eq(a.y, b.y, eps)
}

fn approx_transform(a: Transform, b: Transform, eps: f32) -> bool {
    approx_vec2(a.position(), b.position(), eps)
        && approx_eq(a.rotation().angle(), b.rotation().angle(), eps)
}

fn approx_tuning(a: ConstraintTuning, b: ConstraintTuning, eps: f32) -> bool {
    approx_eq(a.hertz, b.hertz, eps) && approx_eq(a.damping_ratio, b.damping_ratio, eps)
}

fn same_body_id(a: BodyId, b: BodyId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

fn create_dynamic_body(world: &mut World, position: [f32; 2]) -> BodyId {
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(position)
            .build(),
    );
    let shape_def = ShapeDef::builder().density(1.0).build();
    let _shape = world.create_polygon_shape_for(body, &shape_def, &shapes::box_polygon(0.5, 0.5));
    body
}

#[test]
fn joint_defs_are_readable_value_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [-1.0, 0.0]);
    let body_b = create_dynamic_body(&mut world, [2.0, 1.0]);

    let frame_a = Transform::from_pos_angle([0.25_f32, -0.5], 0.3);
    let frame_b = Transform::from_pos_angle([1.0_f32, 2.0], -0.6);
    let tuning = ConstraintTuning::new(4.0, 0.25);

    let base = JointBase::builder()
        .bodies_by_id(body_a, body_b)
        .local_frames(
            frame_a.position(),
            frame_a.rotation().angle(),
            frame_b.position(),
            frame_b.rotation().angle(),
        )
        .collide_connected(true)
        .force_threshold(2.5)
        .torque_threshold(3.5)
        .constraint_hertz(tuning.hertz)
        .constraint_damping_ratio(tuning.damping_ratio)
        .draw_scale(1.25)
        .build();

    assert!(same_body_id(base.body_a_id(), body_a));
    assert!(same_body_id(base.body_b_id(), body_b));
    assert!(approx_transform(base.local_frame_a(), frame_a, 1.0e-6));
    assert!(approx_transform(base.local_frame_b(), frame_b, 1.0e-6));
    assert!(base.collide_connected());
    assert!(approx_eq(base.force_threshold(), 2.5, 1.0e-6));
    assert!(approx_eq(base.torque_threshold(), 3.5, 1.0e-6));
    assert!(approx_tuning(base.constraint_tuning(), tuning, 1.0e-6));
    assert!(approx_eq(base.draw_scale(), 1.25, 1.0e-6));

    let rebuilt_base = JointBaseBuilder::from(base.clone()).draw_scale(2.0).build();
    assert!(same_body_id(rebuilt_base.body_a_id(), body_a));
    assert!(approx_eq(rebuilt_base.draw_scale(), 2.0, 1.0e-6));
    assert!(approx_transform(
        rebuilt_base.local_frame_b(),
        frame_b,
        1.0e-6
    ));
    let roundtrip_base = JointBase::from_raw(base.clone().into_raw());
    assert!(same_body_id(roundtrip_base.body_a_id(), body_a));
    assert!(approx_tuning(
        roundtrip_base.constraint_tuning(),
        tuning,
        1.0e-6
    ));
    assert!(approx_transform(
        roundtrip_base.local_frame_a(),
        frame_a,
        1.0e-6
    ));

    let distance = DistanceJointDef::new(base.clone())
        .length(3.5)
        .enable_spring(true)
        .lower_spring_force(-1.0)
        .upper_spring_force(8.0)
        .hertz(5.0)
        .damping_ratio(0.6)
        .enable_limit(true)
        .min_length(1.5)
        .max_length(6.5)
        .enable_motor(true)
        .max_motor_force(9.0)
        .motor_speed(-2.0);
    assert!(same_body_id(distance.base().body_a_id(), body_a));
    assert!(approx_eq(distance.target_length(), 3.5, 1.0e-6));
    assert!(distance.spring_enabled());
    assert!(approx_eq(distance.minimum_spring_force(), -1.0, 1.0e-6));
    assert!(approx_eq(distance.maximum_spring_force(), 8.0, 1.0e-6));
    assert!(approx_eq(distance.spring_hertz(), 5.0, 1.0e-6));
    assert!(approx_eq(distance.spring_damping_ratio(), 0.6, 1.0e-6));
    assert!(distance.limit_enabled());
    assert!(approx_eq(distance.minimum_length(), 1.5, 1.0e-6));
    assert!(approx_eq(distance.maximum_length(), 6.5, 1.0e-6));
    assert!(distance.motor_enabled());
    assert!(approx_eq(distance.maximum_motor_force(), 9.0, 1.0e-6));
    assert!(approx_eq(distance.target_motor_speed(), -2.0, 1.0e-6));
    let distance_roundtrip = DistanceJointDef::from_raw(distance.clone().into_raw());
    assert!(approx_eq(distance_roundtrip.target_length(), 3.5, 1.0e-6));
    assert!(distance_roundtrip.motor_enabled());

    let prismatic = PrismaticJointDef::new(base.clone())
        .enable_spring(true)
        .hertz(7.0)
        .damping_ratio(0.4)
        .lower_translation(-0.25)
        .upper_translation(0.75)
        .enable_limit(true)
        .enable_motor(true)
        .max_motor_force(11.0)
        .motor_speed(1.5);
    assert!(same_body_id(prismatic.base().body_b_id(), body_b));
    assert!(prismatic.spring_enabled());
    assert!(approx_eq(prismatic.spring_hertz(), 7.0, 1.0e-6));
    assert!(approx_eq(prismatic.spring_damping_ratio(), 0.4, 1.0e-6));
    assert!(approx_eq(prismatic.minimum_translation(), -0.25, 1.0e-6));
    assert!(approx_eq(prismatic.maximum_translation(), 0.75, 1.0e-6));
    assert!(prismatic.limit_enabled());
    assert!(prismatic.motor_enabled());
    assert!(approx_eq(prismatic.maximum_motor_force(), 11.0, 1.0e-6));
    assert!(approx_eq(prismatic.target_motor_speed(), 1.5, 1.0e-6));
    let prismatic_roundtrip = PrismaticJointDef::from_raw(prismatic.clone().into_raw());
    assert!(prismatic_roundtrip.motor_enabled());
    assert!(approx_eq(
        prismatic_roundtrip.maximum_motor_force(),
        11.0,
        1.0e-6
    ));

    let revolute = RevoluteJointDef::new(base.clone())
        .target_angle(0.2)
        .enable_spring(true)
        .hertz(8.0)
        .damping_ratio(0.3)
        .enable_limit(true)
        .lower_angle(-0.5)
        .upper_angle(1.0)
        .enable_motor(true)
        .max_motor_torque(12.0)
        .motor_speed(0.9);
    assert!(same_body_id(revolute.base().body_a_id(), body_a));
    assert!(approx_eq(revolute.target_angle_value(), 0.2, 1.0e-6));
    assert!(revolute.spring_enabled());
    assert!(approx_eq(revolute.spring_hertz(), 8.0, 1.0e-6));
    assert!(approx_eq(revolute.spring_damping_ratio(), 0.3, 1.0e-6));
    assert!(revolute.limit_enabled());
    assert!(approx_eq(revolute.minimum_angle(), -0.5, 1.0e-6));
    assert!(approx_eq(revolute.maximum_angle(), 1.0, 1.0e-6));
    assert!(revolute.motor_enabled());
    assert!(approx_eq(revolute.maximum_motor_torque(), 12.0, 1.0e-6));
    assert!(approx_eq(revolute.target_motor_speed(), 0.9, 1.0e-6));
    let revolute_roundtrip = RevoluteJointDef::from_raw(revolute.clone().into_raw());
    assert!(revolute_roundtrip.limit_enabled());
    assert!(approx_eq(
        revolute_roundtrip.maximum_motor_torque(),
        12.0,
        1.0e-6
    ));

    let weld = WeldJointDef::new(base.clone())
        .linear_hertz(3.0)
        .angular_hertz(4.0)
        .linear_damping_ratio(0.2)
        .angular_damping_ratio(0.7);
    assert!(same_body_id(weld.base().body_a_id(), body_a));
    assert!(approx_eq(weld.configured_linear_hertz(), 3.0, 1.0e-6));
    assert!(approx_eq(weld.configured_angular_hertz(), 4.0, 1.0e-6));
    assert!(approx_eq(
        weld.configured_linear_damping_ratio(),
        0.2,
        1.0e-6
    ));
    assert!(approx_eq(
        weld.configured_angular_damping_ratio(),
        0.7,
        1.0e-6
    ));
    let weld_roundtrip = WeldJointDef::from_raw(weld.clone().into_raw());
    assert!(approx_eq(
        weld_roundtrip.configured_linear_hertz(),
        3.0,
        1.0e-6
    ));
    assert!(approx_eq(
        weld_roundtrip.configured_angular_damping_ratio(),
        0.7,
        1.0e-6
    ));

    let wheel = WheelJointDef::new(base.clone())
        .enable_spring(true)
        .hertz(6.0)
        .damping_ratio(0.5)
        .enable_limit(true)
        .lower_translation(-0.2)
        .upper_translation(0.4)
        .enable_motor(true)
        .max_motor_torque(7.0)
        .motor_speed(-1.25);
    assert!(same_body_id(wheel.base().body_b_id(), body_b));
    assert!(wheel.spring_enabled());
    assert!(approx_eq(wheel.spring_hertz(), 6.0, 1.0e-6));
    assert!(approx_eq(wheel.spring_damping_ratio(), 0.5, 1.0e-6));
    assert!(wheel.limit_enabled());
    assert!(approx_eq(wheel.minimum_translation(), -0.2, 1.0e-6));
    assert!(approx_eq(wheel.maximum_translation(), 0.4, 1.0e-6));
    assert!(wheel.motor_enabled());
    assert!(approx_eq(wheel.maximum_motor_torque(), 7.0, 1.0e-6));
    assert!(approx_eq(wheel.target_motor_speed(), -1.25, 1.0e-6));
    let wheel_roundtrip = WheelJointDef::from_raw(wheel.clone().into_raw());
    assert!(wheel_roundtrip.spring_enabled());
    assert!(approx_eq(
        wheel_roundtrip.maximum_motor_torque(),
        7.0,
        1.0e-6
    ));

    let motor = MotorJointDef::new(base.clone())
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
    assert!(same_body_id(motor.base().body_a_id(), body_a));
    assert!(approx_vec2(
        motor.target_linear_velocity(),
        Vec2::new(2.0, -1.0),
        1.0e-6
    ));
    assert!(approx_eq(motor.maximum_velocity_force(), 5.0, 1.0e-6));
    assert!(approx_eq(motor.target_angular_velocity(), 0.75, 1.0e-6));
    assert!(approx_eq(motor.maximum_velocity_torque(), 6.0, 1.0e-6));
    assert!(approx_eq(motor.linear_spring_hertz(), 2.5, 1.0e-6));
    assert!(approx_eq(motor.linear_spring_damping_ratio(), 0.15, 1.0e-6));
    assert!(approx_eq(motor.maximum_spring_force(), 8.0, 1.0e-6));
    assert!(approx_eq(motor.angular_spring_hertz(), 3.5, 1.0e-6));
    assert!(approx_eq(
        motor.angular_spring_damping_ratio(),
        0.45,
        1.0e-6
    ));
    assert!(approx_eq(motor.maximum_spring_torque(), 9.0, 1.0e-6));
    let motor_roundtrip = MotorJointDef::from_raw(motor.clone().into_raw());
    assert!(approx_vec2(
        motor_roundtrip.target_linear_velocity(),
        Vec2::new(2.0, -1.0),
        1.0e-6
    ));
    assert!(approx_eq(
        motor_roundtrip.maximum_spring_torque(),
        9.0,
        1.0e-6
    ));

    let filter = FilterJointDef::new(base.clone());
    assert!(same_body_id(filter.base().body_b_id(), body_b));
    assert!(filter.base().collide_connected());
    let filter_roundtrip = FilterJointDef::from_raw(filter.into_raw());
    assert!(same_body_id(filter_roundtrip.base().body_b_id(), body_b));
}

#[test]
fn joint_runtime_metadata_and_tuning_are_available_across_owned_scoped_and_world_apis() {
    let mut world = World::new(WorldDef::default()).unwrap();

    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 0.5])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([2.0_f32, -0.25])
            .build(),
    );

    let expected_frame_a = Transform::from_pos_angle([0.25_f32, -0.5], 0.3);
    let expected_frame_b = Transform::from_pos_angle([1.0_f32, 2.0], -0.6);
    let initial_tuning = ConstraintTuning::new(4.0, 0.25);
    let updated_from_scoped = ConstraintTuning::new(8.0, 0.75);
    let updated_from_owned = ConstraintTuning::new(5.0, 0.5);
    let updated_from_world = ConstraintTuning::new(2.0, 0.1);

    let base = JointBaseBuilder::new()
        .bodies_by_id(body_a, body_b)
        .local_frames(
            expected_frame_a.position(),
            expected_frame_a.rotation().angle(),
            expected_frame_b.position(),
            expected_frame_b.rotation().angle(),
        )
        .collide_connected(true)
        .constraint_hertz(initial_tuning.hertz)
        .constraint_damping_ratio(initial_tuning.damping_ratio)
        .build();

    let mut joint = world.create_distance_joint_owned(&DistanceJointDef::new(base).length(3.5));
    let joint_id = joint.id();

    assert_eq!(joint.joint_type(), JointType::Distance);
    assert_eq!(joint.try_joint_type().unwrap(), JointType::Distance);
    assert_eq!(
        JointType::from_raw(joint.joint_type_raw()),
        Some(JointType::Distance)
    );
    assert_eq!(
        JointType::from_raw(joint.try_joint_type_raw().unwrap()),
        Some(JointType::Distance)
    );
    assert!(same_body_id(joint.body_a_id(), body_a));
    assert!(same_body_id(joint.try_body_a_id().unwrap(), body_a));
    assert!(same_body_id(joint.body_b_id(), body_b));
    assert!(same_body_id(joint.try_body_b_id().unwrap(), body_b));
    assert!(joint.collide_connected());
    assert!(joint.try_collide_connected().unwrap());
    assert!(approx_tuning(
        joint.constraint_tuning(),
        initial_tuning,
        1.0e-6
    ));
    assert!(approx_tuning(
        joint.try_constraint_tuning().unwrap(),
        initial_tuning,
        1.0e-6
    ));
    assert!(approx_transform(
        joint.local_frame_a(),
        expected_frame_a,
        1.0e-6
    ));
    assert!(approx_transform(
        joint.try_local_frame_a().unwrap(),
        expected_frame_a,
        1.0e-6
    ));
    assert!(approx_transform(
        joint.local_frame_b(),
        expected_frame_b,
        1.0e-6
    ));
    assert!(approx_transform(
        joint.try_local_frame_b().unwrap(),
        expected_frame_b,
        1.0e-6
    ));

    assert_eq!(world.joint_type(joint_id), JointType::Distance);
    assert_eq!(world.try_joint_type(joint_id).unwrap(), JointType::Distance);
    assert_eq!(
        JointType::from_raw(world.joint_type_raw(joint_id)),
        Some(JointType::Distance)
    );
    assert_eq!(
        JointType::from_raw(world.try_joint_type_raw(joint_id).unwrap()),
        Some(JointType::Distance)
    );
    assert!(same_body_id(world.joint_body_a_id(joint_id), body_a));
    assert!(same_body_id(
        world.try_joint_body_a_id(joint_id).unwrap(),
        body_a
    ));
    assert!(same_body_id(world.joint_body_b_id(joint_id), body_b));
    assert!(same_body_id(
        world.try_joint_body_b_id(joint_id).unwrap(),
        body_b
    ));
    assert!(world.joint_collide_connected(joint_id));
    assert!(world.try_joint_collide_connected(joint_id).unwrap());
    assert!(approx_tuning(
        world.joint_constraint_tuning(joint_id),
        initial_tuning,
        1.0e-6
    ));
    assert!(approx_tuning(
        world.try_joint_constraint_tuning(joint_id).unwrap(),
        initial_tuning,
        1.0e-6
    ));
    assert!(approx_transform(
        world.joint_local_frame_a(joint_id),
        expected_frame_a,
        1.0e-6
    ));
    assert!(approx_transform(
        world.try_joint_local_frame_a(joint_id).unwrap(),
        expected_frame_a,
        1.0e-6
    ));
    assert!(approx_transform(
        world.joint_local_frame_b(joint_id),
        expected_frame_b,
        1.0e-6
    ));
    assert!(approx_transform(
        world.try_joint_local_frame_b(joint_id).unwrap(),
        expected_frame_b,
        1.0e-6
    ));

    {
        let mut scoped = world.joint(joint_id).expect("joint should still be valid");
        assert_eq!(scoped.joint_type(), JointType::Distance);
        assert_eq!(scoped.try_joint_type().unwrap(), JointType::Distance);
        assert!(same_body_id(scoped.body_a_id(), body_a));
        assert!(same_body_id(scoped.body_b_id(), body_b));
        assert!(scoped.collide_connected());
        assert!(approx_tuning(
            scoped.constraint_tuning(),
            initial_tuning,
            1.0e-6
        ));
        assert!(approx_transform(
            scoped.local_frame_a(),
            expected_frame_a,
            1.0e-6
        ));
        assert!(approx_transform(
            scoped.local_frame_b(),
            expected_frame_b,
            1.0e-6
        ));

        scoped.set_collide_connected(false);
        scoped
            .try_set_constraint_tuning(updated_from_scoped)
            .unwrap();
    }

    assert!(!joint.collide_connected());
    assert!(approx_tuning(
        joint.constraint_tuning(),
        updated_from_scoped,
        1.0e-6
    ));

    joint.set_collide_connected(true);
    joint.try_set_constraint_tuning(updated_from_owned).unwrap();
    assert!(joint.collide_connected());
    assert!(approx_tuning(
        joint.constraint_tuning(),
        updated_from_owned,
        1.0e-6
    ));

    world.set_joint_collide_connected(joint_id, false);
    world
        .try_set_joint_constraint_tuning(joint_id, updated_from_world)
        .unwrap();
    assert!(!joint.collide_connected());
    assert!(approx_tuning(
        joint.constraint_tuning(),
        updated_from_world,
        1.0e-6
    ));

    {
        let mut body = world.body(body_a).expect("body A should still be valid");
        body.set_awake(false);
    }
    {
        let mut body = world.body(body_b).expect("body B should still be valid");
        body.set_awake(false);
    }
    assert!(!world.body(body_a).unwrap().is_awake());
    assert!(!world.body(body_b).unwrap().is_awake());

    joint.wake_bodies();
    assert!(world.body(body_a).unwrap().is_awake());
    assert!(world.body(body_b).unwrap().is_awake());

    {
        let mut body = world.body(body_a).expect("body A should still be valid");
        body.set_awake(false);
    }
    {
        let mut body = world.body(body_b).expect("body B should still be valid");
        body.set_awake(false);
    }
    world.try_joint_wake_bodies(joint_id).unwrap();
    assert!(world.body(body_a).unwrap().is_awake());
    assert!(world.body(body_b).unwrap().is_awake());
}

#[test]
fn world_joint_builders_preserve_base_flags_when_populating_runtime_frames() {
    let mut world = World::new(WorldDef::default()).unwrap();

    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );

    let motor = world
        .motor_joint(body_a, body_b)
        .collide_connected(true)
        .build_owned();
    assert_eq!(motor.joint_type(), JointType::Motor);
    assert!(motor.collide_connected());

    let revolute = world
        .revolute(body_a, body_b)
        .anchor_world([0.5_f32, 0.0])
        .collide_connected(true)
        .build_owned();
    assert_eq!(revolute.joint_type(), JointType::Revolute);
    assert!(revolute.collide_connected());
}

#[test]
fn distance_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [2.0_f32, 0.0]);

    let base = world.joint_base_from_world_points(body_a, body_b, [0.0_f32, 0.0], [2.0_f32, 0.0]);
    let def = DistanceJointDef::new(base)
        .length(2.0)
        .enable_spring(true)
        .lower_spring_force(1.0)
        .upper_spring_force(7.0)
        .hertz(5.0)
        .damping_ratio(0.25)
        .enable_limit(true)
        .min_length(1.5)
        .max_length(2.5)
        .enable_motor(true)
        .motor_speed(1.25)
        .max_motor_force(9.0);

    let mut joint = world.create_distance_joint_owned(&def);
    let joint_id = joint.id();

    assert!(approx_eq(joint.distance_length(), 2.0, 1.0e-6));
    assert!(joint.distance_spring_enabled());
    assert!(approx_eq(joint.distance_lower_spring_force(), 1.0, 1.0e-6));
    assert!(approx_eq(joint.distance_upper_spring_force(), 7.0, 1.0e-6));
    assert!(approx_eq(joint.distance_spring_hertz(), 5.0, 1.0e-6));
    assert!(approx_eq(
        joint.distance_spring_damping_ratio(),
        0.25,
        1.0e-6
    ));
    assert!(joint.distance_limit_enabled());
    assert!(approx_eq(joint.distance_min_length(), 1.5, 1.0e-6));
    assert!(approx_eq(joint.distance_max_length(), 2.5, 1.0e-6));
    assert!(joint.distance_motor_enabled());
    assert!(approx_eq(joint.distance_motor_speed(), 1.25, 1.0e-6));
    assert!(approx_eq(joint.distance_max_motor_force(), 9.0, 1.0e-6));

    joint.distance_set_spring_force_range(2.0, 8.0);
    world.distance_set_length(joint_id, 2.25);
    world.distance_set_length_range(joint_id, 1.25, 2.75);
    world.distance_set_motor_speed(joint_id, 0.75);
    world.distance_set_max_motor_force(joint_id, 12.0);
    world.distance_enable_limit(joint_id, false);
    world.distance_enable_motor(joint_id, false);

    {
        let mut scoped = world.joint(joint_id).expect("distance joint should exist");
        scoped.distance_enable_spring(true);
        scoped.distance_set_spring_hertz(6.0);
        scoped.distance_set_spring_damping_ratio(0.55);
        scoped.distance_enable_limit(true);
        scoped.distance_enable_motor(true);

        assert!(approx_eq(scoped.distance_length(), 2.25, 1.0e-6));
        assert!(approx_eq(scoped.distance_lower_spring_force(), 2.0, 1.0e-6));
        assert!(approx_eq(scoped.distance_upper_spring_force(), 8.0, 1.0e-6));
        assert!(approx_eq(scoped.distance_min_length(), 1.25, 1.0e-6));
        assert!(approx_eq(scoped.distance_max_length(), 2.75, 1.0e-6));
        assert!(approx_eq(scoped.distance_motor_speed(), 0.75, 1.0e-6));
        assert!(approx_eq(scoped.distance_max_motor_force(), 12.0, 1.0e-6));
    }

    world.step(1.0 / 60.0, 4);

    assert!(approx_eq(world.distance_length(joint_id), 2.25, 1.0e-6));
    assert!(joint.distance_spring_enabled());
    assert!(approx_eq(joint.distance_lower_spring_force(), 2.0, 1.0e-6));
    assert!(approx_eq(joint.distance_upper_spring_force(), 8.0, 1.0e-6));
    assert!(approx_eq(
        world.distance_spring_hertz(joint_id),
        6.0,
        1.0e-6
    ));
    assert!(approx_eq(
        joint.distance_spring_damping_ratio(),
        0.55,
        1.0e-6
    ));
    assert!(joint.distance_limit_enabled());
    assert!(joint.distance_motor_enabled());
    assert!(approx_eq(world.distance_min_length(joint_id), 1.25, 1.0e-6));
    assert!(approx_eq(world.distance_max_length(joint_id), 2.75, 1.0e-6));
    assert!(approx_eq(joint.distance_motor_speed(), 0.75, 1.0e-6));
    assert!(approx_eq(
        world.distance_max_motor_force(joint_id),
        12.0,
        1.0e-6
    ));
    assert!(joint.distance_current_length().is_finite());
    assert!(world.distance_motor_force(joint_id).is_finite());
}

#[test]
fn prismatic_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [1.0_f32, 0.0]);

    let base = world.joint_base_from_world_with_axis(
        body_a,
        body_b,
        [0.0_f32, 0.0],
        [1.0_f32, 0.0],
        [1.0_f32, 0.0],
    );
    let def = PrismaticJointDef::new(base)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.2)
        .enable_limit(true)
        .lower_translation(-0.5)
        .upper_translation(0.75)
        .enable_motor(true)
        .max_motor_force(9.0)
        .motor_speed(1.5);

    let mut joint = world.create_prismatic_joint_owned(&def);
    let joint_id = joint.id();

    assert!(joint.prismatic_spring_enabled());
    assert!(approx_eq(joint.prismatic_spring_hertz(), 4.0, 1.0e-6));
    assert!(approx_eq(
        joint.prismatic_spring_damping_ratio(),
        0.2,
        1.0e-6
    ));
    assert!(joint.prismatic_limit_enabled());
    assert!(approx_eq(joint.prismatic_lower_limit(), -0.5, 1.0e-6));
    assert!(approx_eq(joint.prismatic_upper_limit(), 0.75, 1.0e-6));
    assert!(joint.prismatic_motor_enabled());
    assert!(approx_eq(joint.prismatic_motor_speed(), 1.5, 1.0e-6));
    assert!(approx_eq(joint.prismatic_max_motor_force(), 9.0, 1.0e-6));

    joint.prismatic_set_target_translation(0.25);
    world.prismatic_enable_motor(joint_id, false);
    world.prismatic_set_max_motor_force(joint_id, 11.0);
    world.prismatic_set_limits(joint_id, -0.25, 0.9);

    {
        let mut scoped = world.joint(joint_id).expect("prismatic joint should exist");
        scoped.prismatic_enable_motor(true);
        scoped.prismatic_set_motor_speed(2.0);
        scoped.prismatic_set_spring_hertz(6.0);
        scoped.prismatic_set_spring_damping_ratio(0.35);
        scoped.prismatic_set_target_translation(0.4);

        assert!(approx_eq(
            scoped.prismatic_target_translation(),
            0.4,
            1.0e-6
        ));
        assert!(approx_eq(scoped.prismatic_lower_limit(), -0.25, 1.0e-6));
        assert!(approx_eq(scoped.prismatic_upper_limit(), 0.9, 1.0e-6));
        assert!(approx_eq(scoped.prismatic_max_motor_force(), 11.0, 1.0e-6));
    }

    world.step(1.0 / 60.0, 4);

    assert!(joint.prismatic_spring_enabled());
    assert!(joint.prismatic_limit_enabled());
    assert!(joint.prismatic_motor_enabled());
    assert!(approx_eq(
        world.prismatic_spring_hertz(joint_id),
        6.0,
        1.0e-6
    ));
    assert!(approx_eq(
        joint.prismatic_spring_damping_ratio(),
        0.35,
        1.0e-6
    ));
    assert!(approx_eq(joint.prismatic_target_translation(), 0.4, 1.0e-6));
    assert!(approx_eq(
        world.prismatic_lower_limit(joint_id),
        -0.25,
        1.0e-6
    ));
    assert!(approx_eq(
        world.prismatic_upper_limit(joint_id),
        0.9,
        1.0e-6
    ));
    assert!(approx_eq(joint.prismatic_motor_speed(), 2.0, 1.0e-6));
    assert!(approx_eq(
        world.prismatic_max_motor_force(joint_id),
        11.0,
        1.0e-6
    ));
    assert!(joint.prismatic_translation().is_finite());
    assert!(joint.prismatic_speed().is_finite());
    assert!(world.prismatic_motor_force(joint_id).is_finite());
}

#[test]
fn revolute_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [1.0_f32, 0.0]);

    let base = world.joint_base_from_world_points(body_a, body_b, [0.5_f32, 0.0], [0.5_f32, 0.0]);
    let def = RevoluteJointDef::new(base)
        .target_angle(0.15)
        .enable_spring(true)
        .hertz(5.0)
        .damping_ratio(0.25)
        .enable_limit(true)
        .lower_angle(-0.4)
        .upper_angle(0.5)
        .enable_motor(true)
        .max_motor_torque(12.0)
        .motor_speed(1.5);

    let mut joint = world.create_revolute_joint_owned(&def);
    let joint_id = joint.id();

    assert!(joint.revolute_spring_enabled());
    assert!(approx_eq(joint.revolute_spring_hertz(), 5.0, 1.0e-6));
    assert!(approx_eq(
        joint.revolute_spring_damping_ratio(),
        0.25,
        1.0e-6
    ));
    assert!(approx_eq(joint.revolute_target_angle(), 0.15, 1.0e-6));
    assert!(joint.revolute_limit_enabled());
    assert!(approx_eq(joint.revolute_lower_limit(), -0.4, 1.0e-6));
    assert!(approx_eq(joint.revolute_upper_limit(), 0.5, 1.0e-6));
    assert!(joint.revolute_motor_enabled());
    assert!(approx_eq(joint.revolute_motor_speed(), 1.5, 1.0e-6));
    assert!(approx_eq(joint.revolute_max_motor_torque(), 12.0, 1.0e-6));

    joint.revolute_set_target_angle(0.2);
    world.revolute_enable_motor(joint_id, false);
    world.revolute_enable_limit(joint_id, false);
    world.revolute_set_limits(joint_id, -0.3, 0.6);
    world.revolute_set_max_motor_torque(joint_id, 14.0);

    {
        let mut scoped = world.joint(joint_id).expect("revolute joint should exist");
        scoped.revolute_enable_motor(true);
        scoped.revolute_enable_limit(true);
        scoped.revolute_enable_spring(true);
        scoped.revolute_set_motor_speed(2.0);
        scoped.revolute_set_spring_hertz(6.0);
        scoped.revolute_set_spring_damping_ratio(0.4);

        assert!(approx_eq(scoped.revolute_target_angle(), 0.2, 1.0e-6));
        assert!(approx_eq(scoped.revolute_lower_limit(), -0.3, 1.0e-6));
        assert!(approx_eq(scoped.revolute_upper_limit(), 0.6, 1.0e-6));
        assert!(approx_eq(scoped.revolute_max_motor_torque(), 14.0, 1.0e-6));
    }

    world.step(1.0 / 60.0, 4);

    assert!(joint.revolute_spring_enabled());
    assert!(joint.revolute_limit_enabled());
    assert!(joint.revolute_motor_enabled());
    assert!(approx_eq(
        world.revolute_target_angle(joint_id),
        0.2,
        1.0e-6
    ));
    assert!(approx_eq(joint.revolute_spring_hertz(), 6.0, 1.0e-6));
    assert!(approx_eq(
        world.revolute_spring_damping_ratio(joint_id),
        0.4,
        1.0e-6
    ));
    assert!(approx_eq(joint.revolute_lower_limit(), -0.3, 1.0e-6));
    assert!(approx_eq(joint.revolute_upper_limit(), 0.6, 1.0e-6));
    assert!(approx_eq(world.revolute_motor_speed(joint_id), 2.0, 1.0e-6));
    assert!(approx_eq(joint.revolute_max_motor_torque(), 14.0, 1.0e-6));
    assert!(joint.revolute_angle().is_finite());
    assert!(world.revolute_motor_torque(joint_id).is_finite());
}

#[test]
fn weld_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [1.0_f32, 0.0]);

    let base = world.joint_base_from_world_points(body_a, body_b, [0.5_f32, 0.0], [0.5_f32, 0.0]);
    let def = WeldJointDef::new(base)
        .linear_hertz(3.0)
        .linear_damping_ratio(0.2)
        .angular_hertz(5.0)
        .angular_damping_ratio(0.4);

    let mut joint = world.create_weld_joint_owned(&def);
    let joint_id = joint.id();

    assert!(approx_eq(joint.weld_linear_hertz(), 3.0, 1.0e-6));
    assert!(approx_eq(joint.weld_linear_damping_ratio(), 0.2, 1.0e-6));
    assert!(approx_eq(joint.weld_angular_hertz(), 5.0, 1.0e-6));
    assert!(approx_eq(joint.weld_angular_damping_ratio(), 0.4, 1.0e-6));

    joint.weld_set_linear_hertz(4.0);
    world.weld_set_linear_damping_ratio(joint_id, 0.3);
    world.weld_set_angular_hertz(joint_id, 6.0);

    {
        let mut scoped = world.joint(joint_id).expect("weld joint should exist");
        scoped.weld_set_angular_damping_ratio(0.5);

        assert!(approx_eq(scoped.weld_linear_hertz(), 4.0, 1.0e-6));
        assert!(approx_eq(scoped.weld_linear_damping_ratio(), 0.3, 1.0e-6));
        assert!(approx_eq(scoped.weld_angular_hertz(), 6.0, 1.0e-6));
        assert!(approx_eq(scoped.weld_angular_damping_ratio(), 0.5, 1.0e-6));
    }

    assert!(approx_eq(world.weld_linear_hertz(joint_id), 4.0, 1.0e-6));
    assert!(approx_eq(joint.weld_linear_damping_ratio(), 0.3, 1.0e-6));
    assert!(approx_eq(world.weld_angular_hertz(joint_id), 6.0, 1.0e-6));
    assert!(approx_eq(joint.weld_angular_damping_ratio(), 0.5, 1.0e-6));
}

#[test]
fn wheel_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [1.0_f32, 0.0]);

    let base = world.joint_base_from_world_with_axis(
        body_a,
        body_b,
        [0.0_f32, 0.0],
        [1.0_f32, 0.0],
        [0.0_f32, 1.0],
    );
    let def = WheelJointDef::new(base)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.2)
        .enable_limit(true)
        .lower_translation(-0.5)
        .upper_translation(0.5)
        .enable_motor(true)
        .max_motor_torque(6.0)
        .motor_speed(1.25);

    let joint = world.create_wheel_joint_owned(&def);
    let joint_id = joint.id();

    assert!(joint.wheel_spring_enabled());
    assert!(approx_eq(joint.wheel_spring_hertz(), 4.0, 1.0e-6));
    assert!(approx_eq(joint.wheel_spring_damping_ratio(), 0.2, 1.0e-6));
    assert!(joint.wheel_limit_enabled());
    assert!(approx_eq(joint.wheel_lower_limit(), -0.5, 1.0e-6));
    assert!(approx_eq(joint.wheel_upper_limit(), 0.5, 1.0e-6));
    assert!(joint.wheel_motor_enabled());
    assert!(approx_eq(joint.wheel_motor_speed(), 1.25, 1.0e-6));
    assert!(approx_eq(joint.wheel_max_motor_torque(), 6.0, 1.0e-6));

    world.wheel_enable_motor(joint_id, false);
    world.wheel_enable_limit(joint_id, false);
    world.wheel_set_limits(joint_id, -0.25, 0.75);
    world.wheel_set_max_motor_torque(joint_id, 7.5);

    {
        let mut scoped = world.joint(joint_id).expect("wheel joint should exist");
        scoped.wheel_enable_motor(true);
        scoped.wheel_enable_limit(true);
        scoped.wheel_enable_spring(true);
        scoped.wheel_set_motor_speed(1.75);
        scoped.wheel_set_spring_hertz(5.5);
        scoped.wheel_set_spring_damping_ratio(0.35);

        assert!(approx_eq(scoped.wheel_lower_limit(), -0.25, 1.0e-6));
        assert!(approx_eq(scoped.wheel_upper_limit(), 0.75, 1.0e-6));
        assert!(approx_eq(scoped.wheel_max_motor_torque(), 7.5, 1.0e-6));
    }

    world.step(1.0 / 60.0, 4);

    assert!(joint.wheel_spring_enabled());
    assert!(joint.wheel_limit_enabled());
    assert!(joint.wheel_motor_enabled());
    assert!(approx_eq(world.wheel_spring_hertz(joint_id), 5.5, 1.0e-6));
    assert!(approx_eq(joint.wheel_spring_damping_ratio(), 0.35, 1.0e-6));
    assert!(approx_eq(joint.wheel_lower_limit(), -0.25, 1.0e-6));
    assert!(approx_eq(world.wheel_upper_limit(joint_id), 0.75, 1.0e-6));
    assert!(approx_eq(joint.wheel_motor_speed(), 1.75, 1.0e-6));
    assert!(approx_eq(
        world.wheel_max_motor_torque(joint_id),
        7.5,
        1.0e-6
    ));
    assert!(joint.wheel_motor_torque().is_finite());
}

#[test]
fn motor_joint_runtime_specific_apis_are_available_across_handle_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [1.0_f32, 0.0]);

    let base = JointBaseBuilder::new().bodies_by_id(body_a, body_b).build();
    let def = MotorJointDef::new(base)
        .linear_velocity([1.0_f32, -0.5])
        .angular_velocity(0.75)
        .max_velocity_force(10.0)
        .max_velocity_torque(9.0)
        .linear_hertz(3.0)
        .linear_damping_ratio(0.2)
        .angular_hertz(4.0)
        .angular_damping_ratio(0.3)
        .max_spring_force(7.0)
        .max_spring_torque(8.0);

    let mut joint = world.create_motor_joint_owned(&def);
    let joint_id = joint.id();

    assert!(approx_vec2(
        joint.motor_linear_velocity(),
        Vec2::new(1.0, -0.5),
        1.0e-6
    ));
    assert!(approx_eq(joint.motor_angular_velocity(), 0.75, 1.0e-6));
    assert!(approx_eq(joint.motor_max_velocity_force(), 10.0, 1.0e-6));
    assert!(approx_eq(joint.motor_max_velocity_torque(), 9.0, 1.0e-6));
    assert!(approx_eq(joint.motor_linear_hertz(), 3.0, 1.0e-6));
    assert!(approx_eq(joint.motor_linear_damping_ratio(), 0.2, 1.0e-6));
    assert!(approx_eq(joint.motor_angular_hertz(), 4.0, 1.0e-6));
    assert!(approx_eq(joint.motor_angular_damping_ratio(), 0.3, 1.0e-6));
    assert!(approx_eq(joint.motor_max_spring_force(), 7.0, 1.0e-6));
    assert!(approx_eq(joint.motor_max_spring_torque(), 8.0, 1.0e-6));

    joint.motor_set_linear_velocity([0.5_f32, 1.0]);
    world.motor_set_angular_velocity(joint_id, 1.5);
    world.motor_set_max_velocity_force(joint_id, 11.0);
    world.motor_set_linear_hertz(joint_id, 4.0);
    world.motor_set_max_spring_force(joint_id, 9.0);

    {
        let mut scoped = world.joint(joint_id).expect("motor joint should exist");
        scoped.motor_set_linear_velocity([2.0_f32, -1.0]);
        scoped.motor_set_max_velocity_torque(12.0);
        scoped.motor_set_linear_damping_ratio(0.4);
        scoped.motor_set_angular_hertz(6.0);
        scoped.motor_set_angular_damping_ratio(0.5);
        scoped.motor_set_max_spring_torque(10.0);

        assert!(approx_vec2(
            scoped.motor_linear_velocity(),
            Vec2::new(2.0, -1.0),
            1.0e-6
        ));
        assert!(approx_eq(scoped.motor_angular_velocity(), 1.5, 1.0e-6));
        assert!(approx_eq(scoped.motor_max_velocity_force(), 11.0, 1.0e-6));
        assert!(approx_eq(scoped.motor_max_velocity_torque(), 12.0, 1.0e-6));
        assert!(approx_eq(scoped.motor_max_spring_force(), 9.0, 1.0e-6));
        assert!(approx_eq(scoped.motor_max_spring_torque(), 10.0, 1.0e-6));
    }

    assert!(approx_vec2(
        world.motor_linear_velocity(joint_id),
        Vec2::new(2.0, -1.0),
        1.0e-6
    ));
    assert!(approx_eq(joint.motor_angular_velocity(), 1.5, 1.0e-6));
    assert!(approx_eq(
        world.motor_max_velocity_force(joint_id),
        11.0,
        1.0e-6
    ));
    assert!(approx_eq(joint.motor_max_velocity_torque(), 12.0, 1.0e-6));
    assert!(approx_eq(joint.motor_linear_hertz(), 4.0, 1.0e-6));
    assert!(approx_eq(
        world.motor_linear_damping_ratio(joint_id),
        0.4,
        1.0e-6
    ));
    assert!(approx_eq(joint.motor_angular_hertz(), 6.0, 1.0e-6));
    assert!(approx_eq(
        world.motor_angular_damping_ratio(joint_id),
        0.5,
        1.0e-6
    ));
    assert!(approx_eq(joint.motor_max_spring_force(), 9.0, 1.0e-6));
    assert!(approx_eq(
        world.motor_max_spring_torque(joint_id),
        10.0,
        1.0e-6
    ));
}
