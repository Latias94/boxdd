use boxdd::{BodyId, JointBase, World};

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn create_dynamic_body(world: &mut World, position: [f32; 2]) -> BodyId {
    let builder = boxdd::BodyBuilder::new();
    let builder = boxdd::BodyBuilder::body_type(builder, boxdd::BodyType::Dynamic);
    let builder = boxdd::BodyBuilder::position(builder, position);
    let body_def = boxdd::BodyBuilder::build(builder);
    let body = boxdd::World::create_body_id(world, body_def);
    let shape_builder = boxdd::ShapeDef::builder();
    let shape_builder = boxdd::ShapeDefBuilder::density(shape_builder, 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder);
    let polygon = boxdd::shapes::box_polygon(0.5_f32, 0.5);
    boxdd::World::create_polygon_shape_for(world, body, &shape_def, &polygon);
    body
}

fn world_with_two_bodies() -> (World, BodyId, BodyId) {
    let mut world =
        boxdd::World::new(boxdd::WorldDef::default()).expect("world creation should succeed");
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [2.0_f32, 0.0]);
    (world, body_a, body_b)
}

fn joint_base(body_a: BodyId, body_b: BodyId) -> JointBase {
    let base = boxdd::JointBase::new(body_a, body_b);
    let base = boxdd::JointBase::with_force_threshold(base, 20.0);
    boxdd::JointBase::with_torque_threshold(base, 30.0)
}

#[test]
fn distance_and_base_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::DistanceJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::DistanceJointDef::length(def, 2.0);
    let def = boxdd::DistanceJointDef::enable_spring(def, true);
    let def = boxdd::DistanceJointDef::lower_spring_force(def, -3.0);
    let def = boxdd::DistanceJointDef::upper_spring_force(def, 7.0);
    let def = boxdd::DistanceJointDef::hertz(def, 4.0);
    let def = boxdd::DistanceJointDef::damping_ratio(def, 0.4);
    let def = boxdd::DistanceJointDef::enable_limit(def, true);
    let def = boxdd::DistanceJointDef::min_length(def, 1.0);
    let def = boxdd::DistanceJointDef::max_length(def, 3.0);
    let def = boxdd::DistanceJointDef::enable_motor(def, true);
    let def = boxdd::DistanceJointDef::max_motor_force(def, 9.0);
    let def = boxdd::DistanceJointDef::motor_speed(def, 1.25);

    let joint_id = boxdd::World::create_distance_joint_id(&mut world, &def);
    let mut joint: boxdd::Joint<'_> =
        boxdd::World::try_joint(&mut world, joint_id).expect("joint should remain valid");

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    boxdd::Joint::set_collide_connected(&mut joint, false);
    boxdd::Joint::set_constraint_tuning(&mut joint, boxdd::ConstraintTuning::new(6.0, 0.6));
    boxdd::Joint::set_force_threshold(&mut joint, 24.0);
    boxdd::Joint::set_local_frame_a(&mut joint, boxdd::Transform::IDENTITY);
    boxdd::Joint::set_local_frame_b(
        &mut joint,
        boxdd::Transform::from_pos_angle([0.25_f32, 0.0], 0.1),
    );
    boxdd::Joint::set_torque_threshold(&mut joint, 34.0);
    boxdd::Joint::wake_bodies(&mut joint);

    boxdd::Joint::distance_set_length(&mut joint, 2.25);
    boxdd::Joint::distance_set_length_range(&mut joint, 1.25, 2.75);
    boxdd::Joint::distance_set_max_motor_force(&mut joint, 12.0);
    boxdd::Joint::distance_set_motor_speed(&mut joint, 0.75);
    boxdd::Joint::distance_set_spring_force_range(&mut joint, -2.0, 8.0);
    boxdd::Joint::distance_enable_spring(&mut joint, true);
    boxdd::Joint::distance_set_spring_hertz(&mut joint, 4.0);
    boxdd::Joint::distance_set_spring_damping_ratio(&mut joint, 0.4);
    boxdd::Joint::distance_enable_limit(&mut joint, true);
    boxdd::Joint::distance_enable_motor(&mut joint, true);

    let joint_body_a = boxdd::Joint::body_a_id(&joint);
    let joint_body_b = boxdd::Joint::body_b_id(&joint);
    let joint_world = boxdd::Joint::try_world_id_raw(&joint);
    let collide_connected = boxdd::Joint::collide_connected(&joint);
    let _constraint_tuning = boxdd::Joint::constraint_tuning(&joint);
    let _local_frame_a = boxdd::Joint::local_frame_a(&joint);
    let _local_frame_b = boxdd::Joint::local_frame_b(&joint);
    let (lower_spring_force, upper_spring_force) =
        boxdd::Joint::distance_spring_force_range(&joint);
    let configured_length = boxdd::Joint::distance_length(&joint);
    let configured_min_length = boxdd::Joint::distance_min_length(&joint);
    let configured_max_length = boxdd::Joint::distance_max_length(&joint);
    let configured_motor_speed = boxdd::Joint::distance_motor_speed(&joint);
    let configured_max_motor_force = boxdd::Joint::distance_max_motor_force(&joint);
    let linear_separation = boxdd::Joint::linear_separation(&joint);
    let angular_separation = boxdd::Joint::angular_separation(&joint);
    let constraint_force = boxdd::Joint::constraint_force(&joint);
    let constraint_torque = boxdd::Joint::constraint_torque(&joint);
    let force_threshold = boxdd::Joint::force_threshold(&joint);
    let torque_threshold = boxdd::Joint::torque_threshold(&joint);
    let current_length = boxdd::Joint::distance_current_length(&joint);
    let motor_force = boxdd::Joint::distance_motor_force(&joint);
    let spring_damping_ratio = boxdd::Joint::distance_spring_damping_ratio(&joint);
    let spring_hertz = boxdd::Joint::distance_spring_hertz(&joint);
    let limit_enabled = boxdd::Joint::distance_limit_enabled(&joint);
    let motor_enabled = boxdd::Joint::distance_motor_enabled(&joint);
    let spring_enabled = boxdd::Joint::distance_spring_enabled(&joint);
    boxdd::Joint::set_user_data(&mut joint, 41_u32);
    let user_data = boxdd::Joint::try_user_data_ptr_raw(&joint);
    let cleared = boxdd::Joint::clear_user_data(&mut joint);
    boxdd::Joint::destroy(joint, true);

    assert_eq!(joint_type, boxdd::JointType::Distance);
    assert!(is_valid);
    assert_eq!(joint_body_a, body_a);
    assert_eq!(joint_body_b, body_b);
    assert!(joint_world.is_ok());
    assert!(!collide_connected);
    assert!(approx_eq(lower_spring_force, -2.0));
    assert!(approx_eq(upper_spring_force, 8.0));
    assert!(approx_eq(configured_length, 2.25));
    assert!(approx_eq(configured_min_length, 1.25));
    assert!(approx_eq(configured_max_length, 2.75));
    assert!(approx_eq(configured_motor_speed, 0.75));
    assert!(approx_eq(configured_max_motor_force, 12.0));
    assert!(linear_separation.is_finite());
    assert!(angular_separation.is_finite());
    assert!(constraint_force.x.is_finite() && constraint_force.y.is_finite());
    assert!(constraint_torque.is_finite());
    assert!(approx_eq(force_threshold, 24.0));
    assert!(approx_eq(torque_threshold, 34.0));
    assert!(current_length.is_finite());
    assert!(motor_force.is_finite());
    assert!(approx_eq(spring_damping_ratio, 0.4));
    assert!(approx_eq(spring_hertz, 4.0));
    assert!(limit_enabled);
    assert!(motor_enabled);
    assert!(spring_enabled);
    assert!(!user_data.expect("user-data query should succeed").is_null());
    assert!(cleared);
}

#[test]
fn prismatic_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::PrismaticJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::PrismaticJointDef::enable_spring(def, true);
    let def = boxdd::PrismaticJointDef::hertz(def, 5.0);
    let def = boxdd::PrismaticJointDef::damping_ratio(def, 0.3);
    let def = boxdd::PrismaticJointDef::lower_translation(def, -1.0);
    let def = boxdd::PrismaticJointDef::upper_translation(def, 1.0);
    let def = boxdd::PrismaticJointDef::enable_limit(def, true);
    let def = boxdd::PrismaticJointDef::enable_motor(def, true);
    let def = boxdd::PrismaticJointDef::max_motor_force(def, 10.0);
    let def = boxdd::PrismaticJointDef::motor_speed(def, 1.5);

    let mut joint = boxdd::World::create_prismatic_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    boxdd::Joint::prismatic_enable_limit(&mut joint, true);
    boxdd::Joint::prismatic_enable_spring(&mut joint, true);
    boxdd::Joint::prismatic_set_limits(&mut joint, -0.75, 0.75);
    boxdd::Joint::prismatic_set_max_motor_force(&mut joint, 12.0);
    boxdd::Joint::prismatic_set_spring_hertz(&mut joint, 5.0);
    boxdd::Joint::prismatic_set_spring_damping_ratio(&mut joint, 0.3);
    boxdd::Joint::prismatic_set_target_translation(&mut joint, 0.25);
    boxdd::Joint::prismatic_enable_motor(&mut joint, true);
    boxdd::Joint::prismatic_set_motor_speed(&mut joint, 1.5);

    let motor_force = boxdd::Joint::prismatic_motor_force(&joint);
    let motor_speed = boxdd::Joint::prismatic_motor_speed(&joint);
    let speed = boxdd::Joint::prismatic_speed(&joint);
    let damping_ratio = boxdd::Joint::prismatic_spring_damping_ratio(&joint);
    let hertz = boxdd::Joint::prismatic_spring_hertz(&joint);
    let translation = boxdd::Joint::prismatic_translation(&joint);
    let target_translation = boxdd::Joint::prismatic_target_translation(&joint);
    let lower_limit = boxdd::Joint::prismatic_lower_limit(&joint);
    let upper_limit = boxdd::Joint::prismatic_upper_limit(&joint);
    let max_motor_force = boxdd::Joint::prismatic_max_motor_force(&joint);
    let limit_enabled = boxdd::Joint::prismatic_limit_enabled(&joint);
    let motor_enabled = boxdd::Joint::prismatic_motor_enabled(&joint);
    let spring_enabled = boxdd::Joint::prismatic_spring_enabled(&joint);

    assert_eq!(joint_type, boxdd::JointType::Prismatic);
    assert!(is_valid);
    assert!(motor_force.is_finite());
    assert!(approx_eq(motor_speed, 1.5));
    assert!(speed.is_finite());
    assert!(approx_eq(damping_ratio, 0.3));
    assert!(approx_eq(hertz, 5.0));
    assert!(translation.is_finite());
    assert!(target_translation.is_finite());
    assert!(approx_eq(lower_limit, -0.75));
    assert!(approx_eq(upper_limit, 0.75));
    assert!(approx_eq(max_motor_force, 12.0));
    assert!(limit_enabled);
    assert!(motor_enabled);
    assert!(spring_enabled);
}

#[test]
fn revolute_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::RevoluteJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::RevoluteJointDef::target_angle(def, 0.1);
    let def = boxdd::RevoluteJointDef::enable_spring(def, true);
    let def = boxdd::RevoluteJointDef::hertz(def, 5.5);
    let def = boxdd::RevoluteJointDef::damping_ratio(def, 0.35);
    let def = boxdd::RevoluteJointDef::enable_limit(def, true);
    let def = boxdd::RevoluteJointDef::lower_angle(def, -0.5);
    let def = boxdd::RevoluteJointDef::upper_angle(def, 0.5);
    let def = boxdd::RevoluteJointDef::enable_motor(def, true);
    let def = boxdd::RevoluteJointDef::max_motor_torque(def, 11.0);
    let def = boxdd::RevoluteJointDef::motor_speed(def, 1.75);

    let mut joint = boxdd::World::create_revolute_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    boxdd::Joint::revolute_set_limits(&mut joint, -0.4, 0.6);
    boxdd::Joint::revolute_set_max_motor_torque(&mut joint, 13.0);
    boxdd::Joint::revolute_set_target_angle(&mut joint, 0.2);
    boxdd::Joint::revolute_enable_spring(&mut joint, true);
    boxdd::Joint::revolute_set_spring_hertz(&mut joint, 5.5);
    boxdd::Joint::revolute_set_spring_damping_ratio(&mut joint, 0.35);
    boxdd::Joint::revolute_enable_limit(&mut joint, true);
    boxdd::Joint::revolute_enable_motor(&mut joint, true);
    boxdd::Joint::revolute_set_motor_speed(&mut joint, 1.75);

    let angle = boxdd::Joint::revolute_angle(&joint);
    let motor_speed = boxdd::Joint::revolute_motor_speed(&joint);
    let motor_torque = boxdd::Joint::revolute_motor_torque(&joint);
    let damping_ratio = boxdd::Joint::revolute_spring_damping_ratio(&joint);
    let hertz = boxdd::Joint::revolute_spring_hertz(&joint);
    let target_angle = boxdd::Joint::revolute_target_angle(&joint);
    let lower_limit = boxdd::Joint::revolute_lower_limit(&joint);
    let upper_limit = boxdd::Joint::revolute_upper_limit(&joint);
    let max_motor_torque = boxdd::Joint::revolute_max_motor_torque(&joint);
    let limit_enabled = boxdd::Joint::revolute_limit_enabled(&joint);
    let motor_enabled = boxdd::Joint::revolute_motor_enabled(&joint);
    let spring_enabled = boxdd::Joint::revolute_spring_enabled(&joint);

    assert_eq!(joint_type, boxdd::JointType::Revolute);
    assert!(is_valid);
    assert!(angle.is_finite());
    assert!(approx_eq(motor_speed, 1.75));
    assert!(motor_torque.is_finite());
    assert!(approx_eq(damping_ratio, 0.35));
    assert!(approx_eq(hertz, 5.5));
    assert!(approx_eq(target_angle, 0.2));
    assert!(approx_eq(lower_limit, -0.4));
    assert!(approx_eq(upper_limit, 0.6));
    assert!(approx_eq(max_motor_torque, 13.0));
    assert!(limit_enabled);
    assert!(motor_enabled);
    assert!(spring_enabled);
}

#[test]
fn motor_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::MotorJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::MotorJointDef::linear_velocity(def, [1.0_f32, -0.5]);
    let def = boxdd::MotorJointDef::max_velocity_force(def, 8.0);
    let def = boxdd::MotorJointDef::angular_velocity(def, 0.75);
    let def = boxdd::MotorJointDef::max_velocity_torque(def, 9.0);
    let def = boxdd::MotorJointDef::linear_hertz(def, 3.0);
    let def = boxdd::MotorJointDef::linear_damping_ratio(def, 0.3);
    let def = boxdd::MotorJointDef::max_spring_force(def, 10.0);
    let def = boxdd::MotorJointDef::angular_hertz(def, 4.0);
    let def = boxdd::MotorJointDef::angular_damping_ratio(def, 0.4);
    let def = boxdd::MotorJointDef::max_spring_torque(def, 11.0);

    let mut joint = boxdd::World::create_motor_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    boxdd::Joint::motor_set_angular_velocity(&mut joint, 1.25);
    boxdd::Joint::motor_set_linear_hertz(&mut joint, 6.0);
    boxdd::Joint::motor_set_max_spring_force(&mut joint, 14.0);
    boxdd::Joint::motor_set_max_velocity_force(&mut joint, 15.0);
    boxdd::Joint::motor_set_linear_velocity(&mut joint, [1.0_f32, -0.5]);
    boxdd::Joint::motor_set_max_velocity_torque(&mut joint, 9.0);
    boxdd::Joint::motor_set_linear_damping_ratio(&mut joint, 0.3);
    boxdd::Joint::motor_set_angular_hertz(&mut joint, 4.0);
    boxdd::Joint::motor_set_angular_damping_ratio(&mut joint, 0.4);
    boxdd::Joint::motor_set_max_spring_torque(&mut joint, 11.0);

    let angular_damping_ratio = boxdd::Joint::motor_angular_damping_ratio(&joint);
    let angular_hertz = boxdd::Joint::motor_angular_hertz(&joint);
    let linear_damping_ratio = boxdd::Joint::motor_linear_damping_ratio(&joint);
    let linear_hertz = boxdd::Joint::motor_linear_hertz(&joint);
    let angular_velocity = boxdd::Joint::motor_angular_velocity(&joint);
    let linear_velocity = boxdd::Joint::motor_linear_velocity(&joint);
    let max_spring_force = boxdd::Joint::motor_max_spring_force(&joint);
    let max_spring_torque = boxdd::Joint::motor_max_spring_torque(&joint);
    let max_velocity_force = boxdd::Joint::motor_max_velocity_force(&joint);
    let max_velocity_torque = boxdd::Joint::motor_max_velocity_torque(&joint);

    assert_eq!(joint_type, boxdd::JointType::Motor);
    assert!(is_valid);
    assert!(approx_eq(angular_damping_ratio, 0.4));
    assert!(approx_eq(angular_hertz, 4.0));
    assert!(approx_eq(linear_damping_ratio, 0.3));
    assert!(approx_eq(linear_hertz, 6.0));
    assert!(approx_eq(angular_velocity, 1.25));
    assert!(approx_eq(linear_velocity.x, 1.0));
    assert!(approx_eq(linear_velocity.y, -0.5));
    assert!(approx_eq(max_spring_force, 14.0));
    assert!(approx_eq(max_spring_torque, 11.0));
    assert!(approx_eq(max_velocity_force, 15.0));
    assert!(approx_eq(max_velocity_torque, 9.0));
}

#[test]
fn weld_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::WeldJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::WeldJointDef::linear_hertz(def, 3.0);
    let def = boxdd::WeldJointDef::linear_damping_ratio(def, 0.2);
    let def = boxdd::WeldJointDef::angular_hertz(def, 4.0);
    let def = boxdd::WeldJointDef::angular_damping_ratio(def, 0.4);

    let mut joint = boxdd::World::create_weld_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    let set_linear_hertz = boxdd::Joint::try_weld_set_linear_hertz(&mut joint, 6.0);
    let set_linear_damping = boxdd::Joint::try_weld_set_linear_damping_ratio(&mut joint, 0.3);
    let set_angular_hertz = boxdd::Joint::try_weld_set_angular_hertz(&mut joint, 7.0);
    let set_angular_damping = boxdd::Joint::try_weld_set_angular_damping_ratio(&mut joint, 0.5);
    let linear_hertz = boxdd::Joint::try_weld_linear_hertz(&joint);
    let linear_damping = boxdd::Joint::try_weld_linear_damping_ratio(&joint);
    let angular_hertz = boxdd::Joint::try_weld_angular_hertz(&joint);
    let angular_damping = boxdd::Joint::try_weld_angular_damping_ratio(&joint);

    assert_eq!(joint_type, boxdd::JointType::Weld);
    assert!(is_valid);
    assert!(set_linear_hertz.is_ok());
    assert!(set_linear_damping.is_ok());
    assert!(set_angular_hertz.is_ok());
    assert!(set_angular_damping.is_ok());
    assert!(approx_eq(
        linear_hertz.expect("linear hertz query should succeed"),
        6.0
    ));
    assert!(approx_eq(
        linear_damping.expect("linear damping query should succeed"),
        0.3
    ));
    assert!(approx_eq(
        angular_hertz.expect("angular hertz query should succeed"),
        7.0
    ));
    assert!(approx_eq(
        angular_damping.expect("angular damping query should succeed"),
        0.5
    ));
}

#[test]
fn wheel_joint_runtime_paths_succeed() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::WheelJointDef::new(joint_base(body_a, body_b));
    let def = boxdd::WheelJointDef::enable_spring(def, true);
    let def = boxdd::WheelJointDef::hertz(def, 4.0);
    let def = boxdd::WheelJointDef::damping_ratio(def, 0.25);
    let def = boxdd::WheelJointDef::enable_limit(def, true);
    let def = boxdd::WheelJointDef::lower_translation(def, -1.0);
    let def = boxdd::WheelJointDef::upper_translation(def, 1.0);
    let def = boxdd::WheelJointDef::enable_motor(def, true);
    let def = boxdd::WheelJointDef::max_motor_torque(def, 10.0);
    let def = boxdd::WheelJointDef::motor_speed(def, 1.5);

    let mut joint = boxdd::World::create_wheel_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    let enable_limit = boxdd::Joint::try_wheel_enable_limit(&mut joint, true);
    let enable_motor = boxdd::Joint::try_wheel_enable_motor(&mut joint, true);
    let enable_spring = boxdd::Joint::try_wheel_enable_spring(&mut joint, true);
    let set_limits = boxdd::Joint::try_wheel_set_limits(&mut joint, -0.75, 0.75);
    let set_max_motor_torque = boxdd::Joint::try_wheel_set_max_motor_torque(&mut joint, 13.0);
    let set_motor_speed = boxdd::Joint::try_wheel_set_motor_speed(&mut joint, 2.0);
    let set_spring_damping = boxdd::Joint::try_wheel_set_spring_damping_ratio(&mut joint, 0.35);
    let set_spring_hertz = boxdd::Joint::try_wheel_set_spring_hertz(&mut joint, 6.0);
    let lower_limit = boxdd::Joint::try_wheel_lower_limit(&joint);
    let max_motor_torque = boxdd::Joint::try_wheel_max_motor_torque(&joint);
    let motor_speed = boxdd::Joint::try_wheel_motor_speed(&joint);
    let motor_torque = boxdd::Joint::try_wheel_motor_torque(&joint);
    let damping_ratio = boxdd::Joint::try_wheel_spring_damping_ratio(&joint);
    let spring_hertz = boxdd::Joint::try_wheel_spring_hertz(&joint);
    let upper_limit = boxdd::Joint::try_wheel_upper_limit(&joint);
    let limit_enabled = boxdd::Joint::try_wheel_limit_enabled(&joint);
    let motor_enabled = boxdd::Joint::try_wheel_motor_enabled(&joint);
    let spring_enabled = boxdd::Joint::try_wheel_spring_enabled(&joint);

    assert_eq!(joint_type, boxdd::JointType::Wheel);
    assert!(is_valid);
    assert!(enable_limit.is_ok());
    assert!(enable_motor.is_ok());
    assert!(enable_spring.is_ok());
    assert!(set_limits.is_ok());
    assert!(set_max_motor_torque.is_ok());
    assert!(set_motor_speed.is_ok());
    assert!(set_spring_damping.is_ok());
    assert!(set_spring_hertz.is_ok());
    assert!(approx_eq(
        lower_limit.expect("lower-limit query should succeed"),
        -0.75
    ));
    assert!(approx_eq(
        max_motor_torque.expect("max-torque query should succeed"),
        13.0
    ));
    assert!(approx_eq(
        motor_speed.expect("motor-speed query should succeed"),
        2.0
    ));
    assert!(
        motor_torque
            .expect("motor-torque query should succeed")
            .is_finite()
    );
    assert!(approx_eq(
        damping_ratio.expect("damping query should succeed"),
        0.35
    ));
    assert!(approx_eq(
        spring_hertz.expect("spring-hertz query should succeed"),
        6.0
    ));
    assert!(approx_eq(
        upper_limit.expect("upper-limit query should succeed"),
        0.75
    ));
    assert!(limit_enabled.expect("limit-state query should succeed"));
    assert!(motor_enabled.expect("motor-state query should succeed"));
    assert!(spring_enabled.expect("spring-state query should succeed"));
}

#[test]
fn filter_joint_creation_runtime_path_succeeds() {
    let (world, body_a, body_b) = world_with_two_bodies();
    let mut world: World = world;
    let def = boxdd::FilterJointDef::new(joint_base(body_a, body_b));

    let joint = boxdd::World::create_filter_joint(&mut world, &def);

    let joint_type = boxdd::Joint::joint_type(&joint);
    let is_valid = boxdd::Joint::is_valid(&joint);

    assert_eq!(joint_type, boxdd::JointType::Filter);
    assert!(is_valid);
}
