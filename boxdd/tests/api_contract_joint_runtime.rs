use boxdd::{BodyId, JointBase, World};

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn create_dynamic_body(world: &mut World, position: [f32; 2]) -> BodyId {
    let builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let builder = boxdd::BodyBuilder::body_type(builder, boxdd::BodyType::Dynamic);
    let builder = boxdd::BodyBuilder::position(builder, position);
    let body_def = boxdd::BodyBuilder::build(builder).unwrap();
    boxdd::World::create_body(world, body_def).expect("body creation should succeed")
}

fn world_with_two_bodies() -> (World, BodyId, BodyId) {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_a = create_dynamic_body(&mut world, [0.0_f32, 0.0]);
    let body_b = create_dynamic_body(&mut world, [2.0_f32, 0.0]);
    (world, body_a, body_b)
}

fn joint_base(world: &World, body_a: BodyId, body_b: BodyId) -> JointBase {
    let base = boxdd::World::joint_base(world, body_a, body_b).unwrap();
    let base = boxdd::JointBase::with_force_threshold(base, 20.0).unwrap();
    boxdd::JointBase::with_torque_threshold(base, 30.0).unwrap()
}

#[test]
fn joint_default_constructors_prove_ordinary_and_recording_issuance() {
    let (mut world, body_a, body_b) = world_with_two_bodies();

    let ordinary = boxdd::World::joint_base(&world, body_a, body_b).unwrap();
    let _distance = boxdd::DistanceJointDef::new(ordinary);
    let _filter = boxdd::FilterJointDef::new(ordinary);
    let _motor = boxdd::MotorJointDef::new(ordinary);
    let _prismatic = boxdd::PrismaticJointDef::new(ordinary);
    let _revolute = boxdd::RevoluteJointDef::new(ordinary);
    let _weld = boxdd::WeldJointDef::new(ordinary);
    let _wheel = boxdd::WheelJointDef::new(ordinary);

    let mut recording =
        boxdd::World::start_recording(&mut world, boxdd::RecordingLimits::default()).unwrap();
    let recorded = boxdd::RecordingSession::joint_base(&recording, body_a, body_b).unwrap();
    let _distance = boxdd::DistanceJointDef::new(recorded);
    let _filter = boxdd::FilterJointDef::new(recorded);
    let _motor = boxdd::MotorJointDef::new(recorded);
    let _prismatic = boxdd::PrismaticJointDef::new(recorded);
    let _revolute = boxdd::RevoluteJointDef::new(recorded);
    let _weld = boxdd::WeldJointDef::new(recorded);
    let _wheel = boxdd::WheelJointDef::new(recorded);
    std::mem::drop(
        boxdd::RecordingSession::step(&mut recording, 0.0, 1)
            .expect("recording step should succeed"),
    );
    std::mem::drop(recording);
}

#[test]
fn distance_and_base_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::DistanceJointDef::new(joint_base(&world, body_a, body_b));
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

    let joint_id = boxdd::World::create_distance_joint(&mut world, &def)
        .expect("distance joint creation should succeed");
    let mut joint = boxdd::World::joint(&mut world, joint_id)
        .expect("joint capability acquisition should succeed");

    let joint_type = boxdd::Joint::joint_type(&joint).expect("joint type should be cached");

    boxdd::Joint::set_collide_connected(&mut joint, false).unwrap();
    boxdd::Joint::set_constraint_tuning(
        &mut joint,
        boxdd::ConstraintTuning::new(6.0, 0.6).unwrap(),
    )
    .unwrap();
    boxdd::Joint::set_force_threshold(&mut joint, 24.0).unwrap();
    boxdd::Joint::set_local_frame_a(&mut joint, boxdd::Transform::IDENTITY).unwrap();
    boxdd::Joint::set_local_frame_b(
        &mut joint,
        boxdd::Transform::from_pos_angle([0.25_f32, 0.0], 0.1).unwrap(),
    )
    .unwrap();
    boxdd::Joint::set_torque_threshold(&mut joint, 34.0).unwrap();
    boxdd::Joint::wake_bodies(&mut joint).unwrap();

    let joint_body_a = boxdd::Joint::body_a_id(&joint).unwrap();
    let joint_body_b = boxdd::Joint::body_b_id(&joint).unwrap();
    let collide_connected = boxdd::Joint::collide_connected(&joint).unwrap();
    let constraint_tuning = boxdd::Joint::constraint_tuning(&joint).unwrap();
    let local_frame_a = boxdd::Joint::local_frame_a(&joint).unwrap();
    let local_frame_b = boxdd::Joint::local_frame_b(&joint).unwrap();
    let linear_separation = boxdd::Joint::linear_separation(&joint).unwrap();
    let angular_separation = boxdd::Joint::angular_separation(&joint).unwrap();
    let constraint_force = boxdd::Joint::constraint_force(&joint).unwrap();
    let constraint_torque = boxdd::Joint::constraint_torque(&joint).unwrap();
    let force_threshold = boxdd::Joint::force_threshold(&joint).unwrap();
    let torque_threshold = boxdd::Joint::torque_threshold(&joint).unwrap();
    boxdd::Joint::set_user_data(&mut joint, 41_u32).unwrap();
    let user_data = boxdd::Joint::user_data_ptr_raw(&joint).unwrap();
    let cleared = boxdd::Joint::clear_user_data(&mut joint).unwrap();

    let mut joint = boxdd::Joint::into_distance(joint).expect("distance kind should match");
    boxdd::DistanceJoint::set_length(&mut joint, 2.25).unwrap();
    boxdd::DistanceJoint::set_length_range(&mut joint, 1.25, 2.75).unwrap();
    boxdd::DistanceJoint::set_max_motor_force(&mut joint, 12.0).unwrap();
    boxdd::DistanceJoint::set_motor_speed(&mut joint, 0.75).unwrap();
    boxdd::DistanceJoint::set_spring_force_range(&mut joint, -2.0, 8.0).unwrap();
    boxdd::DistanceJoint::enable_spring(&mut joint, true).unwrap();
    boxdd::DistanceJoint::set_spring_hertz(&mut joint, 4.0).unwrap();
    boxdd::DistanceJoint::set_spring_damping_ratio(&mut joint, 0.4).unwrap();
    boxdd::DistanceJoint::enable_limit(&mut joint, true).unwrap();
    boxdd::DistanceJoint::enable_motor(&mut joint, true).unwrap();

    let (lower_spring_force, upper_spring_force) =
        boxdd::DistanceJoint::spring_force_range(&joint).unwrap();
    let configured_length = boxdd::DistanceJoint::length(&joint).unwrap();
    let configured_min_length = boxdd::DistanceJoint::min_length(&joint).unwrap();
    let configured_max_length = boxdd::DistanceJoint::max_length(&joint).unwrap();
    let configured_motor_speed = boxdd::DistanceJoint::motor_speed(&joint).unwrap();
    let configured_max_motor_force = boxdd::DistanceJoint::max_motor_force(&joint).unwrap();
    let current_length = boxdd::DistanceJoint::current_length(&joint).unwrap();
    let motor_force = boxdd::DistanceJoint::motor_force(&joint).unwrap();
    let spring_damping_ratio = boxdd::DistanceJoint::spring_damping_ratio(&joint).unwrap();
    let spring_hertz = boxdd::DistanceJoint::spring_hertz(&joint).unwrap();
    let limit_enabled = boxdd::DistanceJoint::limit_enabled(&joint).unwrap();
    let motor_enabled = boxdd::DistanceJoint::motor_enabled(&joint).unwrap();
    let spring_enabled = boxdd::DistanceJoint::spring_enabled(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Distance);
    assert_eq!(joint_body_a, body_a);
    assert_eq!(joint_body_b, body_b);
    assert!(!collide_connected);
    assert_eq!(
        constraint_tuning,
        boxdd::ConstraintTuning::new(6.0, 0.6).unwrap()
    );
    assert_eq!(local_frame_a.position(), boxdd::Vec2::ZERO);
    assert!(local_frame_a.rotation().is_valid());
    assert!(approx_eq(local_frame_b.position().x, 0.25));
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
    assert!(!user_data.is_null());
    assert!(cleared);

    boxdd::Joint::destroy(boxdd::DistanceJoint::into_joint(joint), true).unwrap();
}

#[test]
fn prismatic_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::PrismaticJointDef::new(joint_base(&world, body_a, body_b));
    let def = boxdd::PrismaticJointDef::enable_spring(def, true);
    let def = boxdd::PrismaticJointDef::hertz(def, 5.0);
    let def = boxdd::PrismaticJointDef::damping_ratio(def, 0.3);
    let def = boxdd::PrismaticJointDef::lower_translation(def, -1.0);
    let def = boxdd::PrismaticJointDef::upper_translation(def, 1.0);
    let def = boxdd::PrismaticJointDef::enable_limit(def, true);
    let def = boxdd::PrismaticJointDef::enable_motor(def, true);
    let def = boxdd::PrismaticJointDef::max_motor_force(def, 10.0);
    let def = boxdd::PrismaticJointDef::motor_speed(def, 1.5);

    let joint_id = boxdd::World::create_prismatic_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let mut joint = boxdd::Joint::into_prismatic(joint).unwrap();

    boxdd::PrismaticJoint::enable_limit(&mut joint, true).unwrap();
    boxdd::PrismaticJoint::enable_spring(&mut joint, true).unwrap();
    boxdd::PrismaticJoint::set_limits(&mut joint, -0.75, 0.75).unwrap();
    boxdd::PrismaticJoint::set_max_motor_force(&mut joint, 12.0).unwrap();
    boxdd::PrismaticJoint::set_spring_hertz(&mut joint, 5.0).unwrap();
    boxdd::PrismaticJoint::set_spring_damping_ratio(&mut joint, 0.3).unwrap();
    boxdd::PrismaticJoint::set_target_translation(&mut joint, 0.25).unwrap();
    boxdd::PrismaticJoint::enable_motor(&mut joint, true).unwrap();
    boxdd::PrismaticJoint::set_motor_speed(&mut joint, 1.5).unwrap();

    let motor_force = boxdd::PrismaticJoint::motor_force(&joint).unwrap();
    let motor_speed = boxdd::PrismaticJoint::motor_speed(&joint).unwrap();
    let speed = boxdd::PrismaticJoint::speed(&joint).unwrap();
    let damping_ratio = boxdd::PrismaticJoint::spring_damping_ratio(&joint).unwrap();
    let hertz = boxdd::PrismaticJoint::spring_hertz(&joint).unwrap();
    let translation = boxdd::PrismaticJoint::translation(&joint).unwrap();
    let target_translation = boxdd::PrismaticJoint::target_translation(&joint).unwrap();
    let lower_limit = boxdd::PrismaticJoint::lower_limit(&joint).unwrap();
    let upper_limit = boxdd::PrismaticJoint::upper_limit(&joint).unwrap();
    let max_motor_force = boxdd::PrismaticJoint::max_motor_force(&joint).unwrap();
    let limit_enabled = boxdd::PrismaticJoint::limit_enabled(&joint).unwrap();
    let motor_enabled = boxdd::PrismaticJoint::motor_enabled(&joint).unwrap();
    let spring_enabled = boxdd::PrismaticJoint::spring_enabled(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Prismatic);
    assert!(motor_force.is_finite());
    assert!(approx_eq(motor_speed, 1.5));
    assert!(speed.is_finite());
    assert!(approx_eq(damping_ratio, 0.3));
    assert!(approx_eq(hertz, 5.0));
    assert!(translation.is_finite());
    assert!(approx_eq(target_translation, 0.25));
    assert!(approx_eq(lower_limit, -0.75));
    assert!(approx_eq(upper_limit, 0.75));
    assert!(approx_eq(max_motor_force, 12.0));
    assert!(limit_enabled);
    assert!(motor_enabled);
    assert!(spring_enabled);
}

#[test]
fn revolute_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::RevoluteJointDef::new(joint_base(&world, body_a, body_b));
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

    let joint_id = boxdd::World::create_revolute_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let mut joint = boxdd::Joint::into_revolute(joint).unwrap();

    boxdd::RevoluteJoint::set_limits(&mut joint, -0.4, 0.6).unwrap();
    boxdd::RevoluteJoint::set_max_motor_torque(&mut joint, 13.0).unwrap();
    boxdd::RevoluteJoint::set_target_angle(&mut joint, 0.2).unwrap();
    boxdd::RevoluteJoint::enable_spring(&mut joint, true).unwrap();
    boxdd::RevoluteJoint::set_spring_hertz(&mut joint, 5.5).unwrap();
    boxdd::RevoluteJoint::set_spring_damping_ratio(&mut joint, 0.35).unwrap();
    boxdd::RevoluteJoint::enable_limit(&mut joint, true).unwrap();
    boxdd::RevoluteJoint::enable_motor(&mut joint, true).unwrap();
    boxdd::RevoluteJoint::set_motor_speed(&mut joint, 1.75).unwrap();

    let angle = boxdd::RevoluteJoint::angle(&joint).unwrap();
    let motor_speed = boxdd::RevoluteJoint::motor_speed(&joint).unwrap();
    let motor_torque = boxdd::RevoluteJoint::motor_torque(&joint).unwrap();
    let damping_ratio = boxdd::RevoluteJoint::spring_damping_ratio(&joint).unwrap();
    let hertz = boxdd::RevoluteJoint::spring_hertz(&joint).unwrap();
    let target_angle = boxdd::RevoluteJoint::target_angle(&joint).unwrap();
    let lower_limit = boxdd::RevoluteJoint::lower_limit(&joint).unwrap();
    let upper_limit = boxdd::RevoluteJoint::upper_limit(&joint).unwrap();
    let max_motor_torque = boxdd::RevoluteJoint::max_motor_torque(&joint).unwrap();
    let limit_enabled = boxdd::RevoluteJoint::limit_enabled(&joint).unwrap();
    let motor_enabled = boxdd::RevoluteJoint::motor_enabled(&joint).unwrap();
    let spring_enabled = boxdd::RevoluteJoint::spring_enabled(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Revolute);
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
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::MotorJointDef::new(joint_base(&world, body_a, body_b));
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

    let joint_id = boxdd::World::create_motor_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let mut joint = boxdd::Joint::into_motor(joint).unwrap();

    boxdd::MotorJoint::set_angular_velocity(&mut joint, 1.25).unwrap();
    boxdd::MotorJoint::set_linear_hertz(&mut joint, 6.0).unwrap();
    boxdd::MotorJoint::set_max_spring_force(&mut joint, 14.0).unwrap();
    boxdd::MotorJoint::set_max_velocity_force(&mut joint, 15.0).unwrap();
    boxdd::MotorJoint::set_linear_velocity(&mut joint, [1.0_f32, -0.5]).unwrap();
    boxdd::MotorJoint::set_max_velocity_torque(&mut joint, 9.0).unwrap();
    boxdd::MotorJoint::set_linear_damping_ratio(&mut joint, 0.3).unwrap();
    boxdd::MotorJoint::set_angular_hertz(&mut joint, 4.0).unwrap();
    boxdd::MotorJoint::set_angular_damping_ratio(&mut joint, 0.4).unwrap();
    boxdd::MotorJoint::set_max_spring_torque(&mut joint, 11.0).unwrap();

    let angular_damping_ratio = boxdd::MotorJoint::angular_damping_ratio(&joint).unwrap();
    let angular_hertz = boxdd::MotorJoint::angular_hertz(&joint).unwrap();
    let linear_damping_ratio = boxdd::MotorJoint::linear_damping_ratio(&joint).unwrap();
    let linear_hertz = boxdd::MotorJoint::linear_hertz(&joint).unwrap();
    let angular_velocity = boxdd::MotorJoint::angular_velocity(&joint).unwrap();
    let linear_velocity = boxdd::MotorJoint::linear_velocity(&joint).unwrap();
    let max_spring_force = boxdd::MotorJoint::max_spring_force(&joint).unwrap();
    let max_spring_torque = boxdd::MotorJoint::max_spring_torque(&joint).unwrap();
    let max_velocity_force = boxdd::MotorJoint::max_velocity_force(&joint).unwrap();
    let max_velocity_torque = boxdd::MotorJoint::max_velocity_torque(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Motor);
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
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::WeldJointDef::new(joint_base(&world, body_a, body_b));
    let def = boxdd::WeldJointDef::linear_hertz(def, 3.0);
    let def = boxdd::WeldJointDef::linear_damping_ratio(def, 0.2);
    let def = boxdd::WeldJointDef::angular_hertz(def, 4.0);
    let def = boxdd::WeldJointDef::angular_damping_ratio(def, 0.4);

    let joint_id = boxdd::World::create_weld_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let mut joint = boxdd::Joint::into_weld(joint).unwrap();

    boxdd::WeldJoint::set_linear_hertz(&mut joint, 6.0).unwrap();
    boxdd::WeldJoint::set_linear_damping_ratio(&mut joint, 0.3).unwrap();
    boxdd::WeldJoint::set_angular_hertz(&mut joint, 7.0).unwrap();
    boxdd::WeldJoint::set_angular_damping_ratio(&mut joint, 0.5).unwrap();
    let linear_hertz = boxdd::WeldJoint::linear_hertz(&joint).unwrap();
    let linear_damping = boxdd::WeldJoint::linear_damping_ratio(&joint).unwrap();
    let angular_hertz = boxdd::WeldJoint::angular_hertz(&joint).unwrap();
    let angular_damping = boxdd::WeldJoint::angular_damping_ratio(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Weld);
    assert!(approx_eq(linear_hertz, 6.0));
    assert!(approx_eq(linear_damping, 0.3));
    assert!(approx_eq(angular_hertz, 7.0));
    assert!(approx_eq(angular_damping, 0.5));
}

#[test]
fn wheel_joint_runtime_paths_succeed() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::WheelJointDef::new(joint_base(&world, body_a, body_b));
    let def = boxdd::WheelJointDef::enable_spring(def, true);
    let def = boxdd::WheelJointDef::hertz(def, 4.0);
    let def = boxdd::WheelJointDef::damping_ratio(def, 0.25);
    let def = boxdd::WheelJointDef::enable_limit(def, true);
    let def = boxdd::WheelJointDef::lower_translation(def, -1.0);
    let def = boxdd::WheelJointDef::upper_translation(def, 1.0);
    let def = boxdd::WheelJointDef::enable_motor(def, true);
    let def = boxdd::WheelJointDef::max_motor_torque(def, 10.0);
    let def = boxdd::WheelJointDef::motor_speed(def, 1.5);

    let joint_id = boxdd::World::create_wheel_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let mut joint = boxdd::Joint::into_wheel(joint).unwrap();

    boxdd::WheelJoint::enable_limit(&mut joint, true).unwrap();
    boxdd::WheelJoint::enable_motor(&mut joint, true).unwrap();
    boxdd::WheelJoint::enable_spring(&mut joint, true).unwrap();
    boxdd::WheelJoint::set_limits(&mut joint, -0.75, 0.75).unwrap();
    boxdd::WheelJoint::set_max_motor_torque(&mut joint, 13.0).unwrap();
    boxdd::WheelJoint::set_motor_speed(&mut joint, 2.0).unwrap();
    boxdd::WheelJoint::set_spring_damping_ratio(&mut joint, 0.35).unwrap();
    boxdd::WheelJoint::set_spring_hertz(&mut joint, 6.0).unwrap();
    let lower_limit = boxdd::WheelJoint::lower_limit(&joint).unwrap();
    let max_motor_torque = boxdd::WheelJoint::max_motor_torque(&joint).unwrap();
    let motor_speed = boxdd::WheelJoint::motor_speed(&joint).unwrap();
    let motor_torque = boxdd::WheelJoint::motor_torque(&joint).unwrap();
    let damping_ratio = boxdd::WheelJoint::spring_damping_ratio(&joint).unwrap();
    let spring_hertz = boxdd::WheelJoint::spring_hertz(&joint).unwrap();
    let upper_limit = boxdd::WheelJoint::upper_limit(&joint).unwrap();
    let limit_enabled = boxdd::WheelJoint::limit_enabled(&joint).unwrap();
    let motor_enabled = boxdd::WheelJoint::motor_enabled(&joint).unwrap();
    let spring_enabled = boxdd::WheelJoint::spring_enabled(&joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Wheel);
    assert!(approx_eq(lower_limit, -0.75));
    assert!(approx_eq(max_motor_torque, 13.0));
    assert!(approx_eq(motor_speed, 2.0));
    assert!(motor_torque.is_finite());
    assert!(approx_eq(damping_ratio, 0.35));
    assert!(approx_eq(spring_hertz, 6.0));
    assert!(approx_eq(upper_limit, 0.75));
    assert!(limit_enabled);
    assert!(motor_enabled);
    assert!(spring_enabled);
}

#[test]
fn filter_joint_creation_runtime_path_succeeds() {
    let (mut world, body_a, body_b) = world_with_two_bodies();
    let def = boxdd::FilterJointDef::new(joint_base(&world, body_a, body_b));

    let joint_id = boxdd::World::create_filter_joint(&mut world, &def).unwrap();
    let joint = boxdd::World::joint(&mut world, joint_id).unwrap();
    let joint_type = boxdd::Joint::joint_type(&joint).unwrap();
    let joint = boxdd::Joint::into_filter(joint).unwrap();

    assert_eq!(joint_type, boxdd::JointType::Filter);
    assert_eq!(boxdd::Joint::id(&joint), joint_id);
}
