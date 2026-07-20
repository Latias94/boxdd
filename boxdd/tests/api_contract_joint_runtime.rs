use boxdd::{prelude::*, shapes};

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn create_dynamic_body(world: &mut World, position: [f32; 2]) -> BodyId {
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(position)
            .build(),
    );
    let shape_def = ShapeDef::builder().density(1.0).build();
    let shape =
        world.create_polygon_shape_for(body, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    assert!(world.shape(shape).is_some());
    body
}

fn world_with_two_bodies() -> (World, BodyId, BodyId) {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [2.0_f32, 0.0]);
    (world, body_a, body_b)
}

fn joint_base(body_a: BodyId, body_b: BodyId) -> JointBase {
    JointBase::new(body_a, body_b)
        .with_force_threshold(20.0)
        .with_torque_threshold(30.0)
}

#[test]
fn distance_and_base_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = DistanceJointDef::new(joint_base(body_a, body_b))
        .length(2.0)
        .enable_spring(true)
        .lower_spring_force(-3.0)
        .upper_spring_force(7.0)
        .hertz(4.0)
        .damping_ratio(0.4)
        .enable_limit(true)
        .min_length(1.0)
        .max_length(3.0)
        .enable_motor(true)
        .max_motor_force(9.0)
        .motor_speed(1.25);

    let mut joint = boxdd::World::create_distance_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Distance);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    boxdd::Joint::set_constraint_tuning(&mut joint, ConstraintTuning::new(6.0, 0.6));
    boxdd::Joint::set_force_threshold(&mut joint, 24.0);
    boxdd::Joint::set_local_frame_b(&mut joint, Transform::from_pos_angle([0.25_f32, 0.0], 0.1));
    boxdd::Joint::set_torque_threshold(&mut joint, 34.0);
    boxdd::Joint::wake_bodies(&mut joint);

    boxdd::Joint::distance_set_length(&mut joint, 2.25);
    boxdd::Joint::distance_set_length_range(&mut joint, 1.25, 2.75);
    boxdd::Joint::distance_set_max_motor_force(&mut joint, 12.0);
    boxdd::Joint::distance_set_motor_speed(&mut joint, 0.75);
    boxdd::Joint::distance_set_spring_force_range(&mut joint, -2.0, 8.0);

    let (lower_spring_force, upper_spring_force) =
        boxdd::Joint::distance_spring_force_range(&joint);
    assert!(approx_eq(lower_spring_force, -2.0));
    assert!(approx_eq(upper_spring_force, 8.0));

    let linear_separation = boxdd::Joint::linear_separation(&joint);
    assert!(linear_separation.is_finite());
    let angular_separation = boxdd::Joint::angular_separation(&joint);
    assert!(angular_separation.is_finite());
    let constraint_force = boxdd::Joint::constraint_force(&joint);
    assert!(constraint_force.x.is_finite() && constraint_force.y.is_finite());
    let constraint_torque = boxdd::Joint::constraint_torque(&joint);
    assert!(constraint_torque.is_finite());
    let force_threshold = boxdd::Joint::force_threshold(&joint);
    assert!(approx_eq(force_threshold, 24.0));
    let torque_threshold = boxdd::Joint::torque_threshold(&joint);
    assert!(approx_eq(torque_threshold, 34.0));

    let current_length = boxdd::Joint::distance_current_length(&joint);
    assert!(current_length.is_finite());
    let motor_force = boxdd::Joint::distance_motor_force(&joint);
    assert!(motor_force.is_finite());
    let spring_damping_ratio = boxdd::Joint::distance_spring_damping_ratio(&joint);
    assert!(approx_eq(spring_damping_ratio, 0.4));
    let spring_hertz = boxdd::Joint::distance_spring_hertz(&joint);
    assert!(approx_eq(spring_hertz, 4.0));
    let limit_enabled = boxdd::Joint::distance_limit_enabled(&joint);
    assert!(limit_enabled);
    let motor_enabled = boxdd::Joint::distance_motor_enabled(&joint);
    assert!(motor_enabled);
    let spring_enabled = boxdd::Joint::distance_spring_enabled(&joint);
    assert!(spring_enabled);

    joint.set_user_data(41_u32);
    let user_data = boxdd::Joint::try_user_data_ptr_raw(&joint);
    assert!(!user_data.expect("user-data query should succeed").is_null());
    let cleared = boxdd::Joint::clear_user_data(&mut joint);
    assert!(cleared);

    boxdd::Joint::destroy(joint, true);
}

#[test]
fn prismatic_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = PrismaticJointDef::new(joint_base(body_a, body_b))
        .enable_spring(true)
        .hertz(5.0)
        .damping_ratio(0.3)
        .lower_translation(-1.0)
        .upper_translation(1.0)
        .enable_limit(true)
        .enable_motor(true)
        .max_motor_force(10.0)
        .motor_speed(1.5);

    let mut joint = boxdd::World::create_prismatic_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Prismatic);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    boxdd::Joint::prismatic_enable_limit(&mut joint, true);
    boxdd::Joint::prismatic_enable_spring(&mut joint, true);
    boxdd::Joint::prismatic_set_limits(&mut joint, -0.75, 0.75);
    boxdd::Joint::prismatic_set_max_motor_force(&mut joint, 12.0);

    let motor_force = boxdd::Joint::prismatic_motor_force(&joint);
    assert!(motor_force.is_finite());
    let motor_speed = boxdd::Joint::prismatic_motor_speed(&joint);
    assert!(approx_eq(motor_speed, 1.5));
    let speed = boxdd::Joint::prismatic_speed(&joint);
    assert!(speed.is_finite());
    let damping_ratio = boxdd::Joint::prismatic_spring_damping_ratio(&joint);
    assert!(approx_eq(damping_ratio, 0.3));
    let hertz = boxdd::Joint::prismatic_spring_hertz(&joint);
    assert!(approx_eq(hertz, 5.0));
    let translation = boxdd::Joint::prismatic_translation(&joint);
    assert!(translation.is_finite());
    let limit_enabled = boxdd::Joint::prismatic_limit_enabled(&joint);
    assert!(limit_enabled);
    let motor_enabled = boxdd::Joint::prismatic_motor_enabled(&joint);
    assert!(motor_enabled);
    let spring_enabled = boxdd::Joint::prismatic_spring_enabled(&joint);
    assert!(spring_enabled);
}

#[test]
fn revolute_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = RevoluteJointDef::new(joint_base(body_a, body_b))
        .target_angle(0.1)
        .enable_spring(true)
        .hertz(5.5)
        .damping_ratio(0.35)
        .enable_limit(true)
        .lower_angle(-0.5)
        .upper_angle(0.5)
        .enable_motor(true)
        .max_motor_torque(11.0)
        .motor_speed(1.75);

    let mut joint = boxdd::World::create_revolute_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Revolute);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    boxdd::Joint::revolute_set_limits(&mut joint, -0.4, 0.6);
    boxdd::Joint::revolute_set_max_motor_torque(&mut joint, 13.0);
    boxdd::Joint::revolute_set_target_angle(&mut joint, 0.2);

    let angle = boxdd::Joint::revolute_angle(&joint);
    assert!(angle.is_finite());
    let motor_speed = boxdd::Joint::revolute_motor_speed(&joint);
    assert!(approx_eq(motor_speed, 1.75));
    let motor_torque = boxdd::Joint::revolute_motor_torque(&joint);
    assert!(motor_torque.is_finite());
    let damping_ratio = boxdd::Joint::revolute_spring_damping_ratio(&joint);
    assert!(approx_eq(damping_ratio, 0.35));
    let hertz = boxdd::Joint::revolute_spring_hertz(&joint);
    assert!(approx_eq(hertz, 5.5));
    let limit_enabled = boxdd::Joint::revolute_limit_enabled(&joint);
    assert!(limit_enabled);
    let motor_enabled = boxdd::Joint::revolute_motor_enabled(&joint);
    assert!(motor_enabled);
    let spring_enabled = boxdd::Joint::revolute_spring_enabled(&joint);
    assert!(spring_enabled);
}

#[test]
fn motor_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = MotorJointDef::new(joint_base(body_a, body_b))
        .linear_velocity([1.0_f32, -0.5])
        .max_velocity_force(8.0)
        .angular_velocity(0.75)
        .max_velocity_torque(9.0)
        .linear_hertz(3.0)
        .linear_damping_ratio(0.3)
        .max_spring_force(10.0)
        .angular_hertz(4.0)
        .angular_damping_ratio(0.4)
        .max_spring_torque(11.0);

    let mut joint = boxdd::World::create_motor_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Motor);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    boxdd::Joint::motor_set_angular_velocity(&mut joint, 1.25);
    boxdd::Joint::motor_set_linear_hertz(&mut joint, 6.0);
    boxdd::Joint::motor_set_max_spring_force(&mut joint, 14.0);
    boxdd::Joint::motor_set_max_velocity_force(&mut joint, 15.0);

    let angular_damping_ratio = boxdd::Joint::motor_angular_damping_ratio(&joint);
    assert!(approx_eq(angular_damping_ratio, 0.4));
    let angular_hertz = boxdd::Joint::motor_angular_hertz(&joint);
    assert!(approx_eq(angular_hertz, 4.0));
    let linear_damping_ratio = boxdd::Joint::motor_linear_damping_ratio(&joint);
    assert!(approx_eq(linear_damping_ratio, 0.3));
    let linear_hertz = boxdd::Joint::motor_linear_hertz(&joint);
    assert!(approx_eq(linear_hertz, 6.0));
    let angular_velocity = joint.motor_angular_velocity();
    assert!(approx_eq(angular_velocity, 1.25));
    let max_spring_force = joint.motor_max_spring_force();
    assert!(approx_eq(max_spring_force, 14.0));
    let max_velocity_force = joint.motor_max_velocity_force();
    assert!(approx_eq(max_velocity_force, 15.0));
}

#[test]
fn weld_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = WeldJointDef::new(joint_base(body_a, body_b))
        .linear_hertz(3.0)
        .linear_damping_ratio(0.2)
        .angular_hertz(4.0)
        .angular_damping_ratio(0.4);

    let mut joint = boxdd::World::create_weld_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Weld);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    let set_linear_hertz = boxdd::Joint::try_weld_set_linear_hertz(&mut joint, 6.0);
    assert!(set_linear_hertz.is_ok());
    let set_linear_damping = boxdd::Joint::try_weld_set_linear_damping_ratio(&mut joint, 0.3);
    assert!(set_linear_damping.is_ok());
    let set_angular_hertz = boxdd::Joint::try_weld_set_angular_hertz(&mut joint, 7.0);
    assert!(set_angular_hertz.is_ok());
    let set_angular_damping = boxdd::Joint::try_weld_set_angular_damping_ratio(&mut joint, 0.5);
    assert!(set_angular_damping.is_ok());

    let linear_hertz = boxdd::Joint::try_weld_linear_hertz(&joint);
    assert!(approx_eq(
        linear_hertz.expect("linear hertz query should succeed"),
        6.0
    ));
    let linear_damping = boxdd::Joint::try_weld_linear_damping_ratio(&joint);
    assert!(approx_eq(
        linear_damping.expect("linear damping query should succeed"),
        0.3
    ));
    let angular_hertz = boxdd::Joint::try_weld_angular_hertz(&joint);
    assert!(approx_eq(
        angular_hertz.expect("angular hertz query should succeed"),
        7.0
    ));
    let angular_damping = boxdd::Joint::try_weld_angular_damping_ratio(&joint);
    assert!(approx_eq(
        angular_damping.expect("angular damping query should succeed"),
        0.5
    ));
}

#[test]
fn wheel_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = WheelJointDef::new(joint_base(body_a, body_b))
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.25)
        .enable_limit(true)
        .lower_translation(-1.0)
        .upper_translation(1.0)
        .enable_motor(true)
        .max_motor_torque(10.0)
        .motor_speed(1.5);

    let mut joint = boxdd::World::create_wheel_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Wheel);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);

    let enable_limit = boxdd::Joint::try_wheel_enable_limit(&mut joint, true);
    assert!(enable_limit.is_ok());
    let enable_motor = boxdd::Joint::try_wheel_enable_motor(&mut joint, true);
    assert!(enable_motor.is_ok());
    let enable_spring = boxdd::Joint::try_wheel_enable_spring(&mut joint, true);
    assert!(enable_spring.is_ok());
    let set_limits = boxdd::Joint::try_wheel_set_limits(&mut joint, -0.75, 0.75);
    assert!(set_limits.is_ok());
    let set_max_motor_torque = boxdd::Joint::try_wheel_set_max_motor_torque(&mut joint, 13.0);
    assert!(set_max_motor_torque.is_ok());
    let set_motor_speed = boxdd::Joint::try_wheel_set_motor_speed(&mut joint, 2.0);
    assert!(set_motor_speed.is_ok());
    let set_spring_damping = boxdd::Joint::try_wheel_set_spring_damping_ratio(&mut joint, 0.35);
    assert!(set_spring_damping.is_ok());
    let set_spring_hertz = boxdd::Joint::try_wheel_set_spring_hertz(&mut joint, 6.0);
    assert!(set_spring_hertz.is_ok());

    let lower_limit = boxdd::Joint::try_wheel_lower_limit(&joint);
    assert!(approx_eq(
        lower_limit.expect("lower-limit query should succeed"),
        -0.75
    ));
    let max_motor_torque = boxdd::Joint::try_wheel_max_motor_torque(&joint);
    assert!(approx_eq(
        max_motor_torque.expect("max-torque query should succeed"),
        13.0
    ));
    let motor_speed = boxdd::Joint::try_wheel_motor_speed(&joint);
    assert!(approx_eq(
        motor_speed.expect("motor-speed query should succeed"),
        2.0
    ));
    let motor_torque = boxdd::Joint::try_wheel_motor_torque(&joint);
    assert!(
        motor_torque
            .expect("motor-torque query should succeed")
            .is_finite()
    );
    let damping_ratio = boxdd::Joint::try_wheel_spring_damping_ratio(&joint);
    assert!(approx_eq(
        damping_ratio.expect("damping query should succeed"),
        0.35
    ));
    let spring_hertz = boxdd::Joint::try_wheel_spring_hertz(&joint);
    assert!(approx_eq(
        spring_hertz.expect("spring-hertz query should succeed"),
        6.0
    ));
    let upper_limit = boxdd::Joint::try_wheel_upper_limit(&joint);
    assert!(approx_eq(
        upper_limit.expect("upper-limit query should succeed"),
        0.75
    ));
    let limit_enabled = boxdd::Joint::try_wheel_limit_enabled(&joint);
    assert!(limit_enabled.expect("limit-state query should succeed"));
    let motor_enabled = boxdd::Joint::try_wheel_motor_enabled(&joint);
    assert!(motor_enabled.expect("motor-state query should succeed"));
    let spring_enabled = boxdd::Joint::try_wheel_spring_enabled(&joint);
    assert!(spring_enabled.expect("spring-state query should succeed"));
}

#[test]
fn filter_joint_creation_runtime_path_succeeds() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = FilterJointDef::new(joint_base(body_a, body_b));

    let joint = boxdd::World::create_filter_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    assert_eq!(joint_type, JointType::Filter);
    let is_valid = boxdd::Joint::is_valid(&joint);
    assert!(is_valid);
}
