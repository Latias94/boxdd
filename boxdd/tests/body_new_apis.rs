use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

#[test]
fn body_def_is_a_readable_value_type_and_can_seed_a_builder() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let def = foundation
        .body_builder()
        .body_type(BodyType::Dynamic)
        .position([1.5_f32, -2.25])
        .angle(0.75)
        .linear_velocity([-3.0_f32, 4.5])
        .angular_velocity(1.25)
        .linear_damping(0.2)
        .angular_damping(0.4)
        .sleep_threshold(0.3)
        .gravity_scale(1.75)
        .name("dynamic")
        .unwrap()
        .enable_sleep(false)
        .awake(false)
        .bullet(true)
        .allow_fast_rotation(true)
        .enabled(false)
        .build()
        .unwrap();

    assert_eq!(def.body_type(), BodyType::Dynamic);
    assert_eq!(def.position(), Position::new(1.5, -2.25));
    assert!(approx_eq(def.angle(), 0.75, 1.0e-6));
    assert!(approx_eq(def.rotation().angle(), 0.75, 1.0e-6));
    assert_eq!(def.linear_velocity(), Vec2::new(-3.0, 4.5));
    assert!(approx_eq(def.angular_velocity(), 1.25, 1.0e-6));
    assert!(approx_eq(def.linear_damping(), 0.2, 1.0e-6));
    assert!(approx_eq(def.angular_damping(), 0.4, 1.0e-6));
    assert!(approx_eq(def.sleep_threshold(), 0.3, 1.0e-6));
    assert!(approx_eq(def.gravity_scale(), 1.75, 1.0e-6));
    assert_eq!(def.name(), Some(c"dynamic"));
    assert!(!def.is_sleep_enabled());
    assert!(!def.is_awake());
    assert!(def.is_bullet());
    assert!(def.is_fast_rotation_allowed());
    assert!(!def.is_enabled());

    let rebuilt = BodyBuilder::from(def.clone())
        .position([0.0_f32, 2.0])
        .enabled(true)
        .build()
        .unwrap();
    assert_eq!(rebuilt.body_type(), BodyType::Dynamic);
    assert_eq!(rebuilt.position(), Position::new(0.0, 2.0));
    assert!(approx_eq(rebuilt.angle(), 0.75, 1.0e-6));
    assert_eq!(rebuilt.linear_velocity(), Vec2::new(-3.0, 4.5));
    assert!(rebuilt.is_enabled());
    assert_eq!(rebuilt.name(), Some(c"dynamic"));
}

#[test]
fn body_def_owns_names_across_clone_and_creation() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let definition = foundation
        .body_builder()
        .name("owned")
        .unwrap()
        .build()
        .unwrap();
    let cloned = definition.clone();
    assert_ne!(
        definition.name().unwrap().as_ptr(),
        cloned.name().unwrap().as_ptr()
    );
    assert_eq!(definition.name(), Some(c"owned"));

    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let body_id = world
        .create_body(
            foundation
                .body_builder()
                .name("created")
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        world.body(body_id).unwrap().name().unwrap().as_deref(),
        Some("created")
    );
}

#[test]
fn invalid_body_names_are_recoverable_errors() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    assert_eq!(
        foundation.body_builder().name("nul\0byte").unwrap_err(),
        Error::NulByteInString
    );
    assert_eq!(
        foundation.body_builder().name("12345678901").unwrap_err(),
        Error::invalid_argument("BodyBuilder::name", "name", "at most 10 UTF-8 bytes")
    );
}

#[test]
fn body_capability_controls_runtime_state_and_enumerates_attachments() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 1.0])
                .angle(0.5)
                .enable_sleep(true)
                .build()
                .unwrap(),
        )
        .unwrap();
    let other_body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 1.0])
                .build()
                .unwrap(),
        )
        .unwrap();

    let (shape_a, shape_b) = {
        let mut body = world.body(body_id).unwrap();
        let shape_a = body
            .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 0.5)
            .unwrap();
        let shape_b = body
            .create_box(
                &ShapeDef::builder().density(1.0).build().unwrap(),
                0.25,
                0.75,
            )
            .unwrap();
        (shape_a, shape_b)
    };
    let joint_id = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body_id, other_body_id).unwrap(),
        ))
        .unwrap();

    let mut body = world.body(body_id).unwrap();
    assert!(approx_eq(body.rotation().unwrap().angle(), 0.5, 1.0e-6));
    assert_eq!(body.linear_velocity().unwrap(), Vec2::ZERO);
    body.set_linear_velocity([2.5_f32, -1.25]).unwrap();
    body.set_angular_velocity(1.5).unwrap();
    assert_eq!(body.linear_velocity().unwrap(), Vec2::new(2.5, -1.25));
    assert!(approx_eq(body.angular_velocity().unwrap(), 1.5, 1.0e-6));

    assert!(body.is_sleep_enabled().unwrap());
    body.enable_sleep(false).unwrap();
    assert!(!body.is_sleep_enabled().unwrap());
    body.enable_sleep(true).unwrap();
    body.set_sleep_threshold(0.5).unwrap();
    assert!(approx_eq(body.sleep_threshold().unwrap(), 0.5, 1.0e-6));

    assert!(body.is_enabled().unwrap());
    body.disable().unwrap();
    assert!(!body.is_enabled().unwrap());
    body.enable().unwrap();
    body.set_bullet(true).unwrap();
    assert!(body.is_bullet().unwrap());

    body.set_name("runtime").unwrap();
    assert_eq!(body.name().unwrap().as_deref(), Some("runtime"));
    body.enable_contact_events(true).unwrap();
    body.enable_hit_events(true).unwrap();

    assert_eq!(body.shape_count().unwrap(), 2);
    let shapes = body.shapes().unwrap();
    assert_eq!(shapes.len(), 2);
    assert!(shapes.contains(&shape_a));
    assert!(shapes.contains(&shape_b));
    assert_eq!(body.joint_count().unwrap(), 1);
    assert_eq!(body.joints().unwrap(), vec![joint_id]);
}

#[test]
fn body_and_shape_capabilities_agree_on_bounds() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([2.0_f32, 3.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 0.5)
        .unwrap();

    let shape_bounds = world.shape(shape_id).unwrap().aabb().unwrap();
    let body_bounds = world.body(body_id).unwrap().aabb().unwrap();
    assert_eq!(body_bounds, shape_bounds);
}

#[test]
fn body_capability_enforces_world_provenance_and_liveness() {
    let mut source = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let source_body = source
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut target = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    assert_eq!(target.body(source_body).err().unwrap(), Error::WrongWorld);
    source.body(source_body).unwrap().destroy().unwrap();
    assert_eq!(
        source.body(source_body).err().unwrap(),
        Error::InvalidBodyId
    );
}

#[test]
fn body_user_data_is_owned_by_the_world_registry() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();

    body.set_user_data(String::from("payload")).unwrap();
    assert_eq!(
        body.with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("payload")
    );
    assert_eq!(
        body.take_user_data::<String>().unwrap().as_deref(),
        Some("payload")
    );
    assert!(
        body.with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .is_none()
    );
}

#[test]
fn body_shape_convenience_constructors_validate_before_native_creation() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();

    assert_eq!(
        body.create_centered_circle(&ShapeDef::default(), f32::NAN),
        Err(Error::invalid_argument(
            "Circle::new",
            "circle",
            "finite center coordinates and a finite non-negative radius",
        ))
    );
    assert_eq!(
        body.create_box(&ShapeDef::default(), -1.0, 1.0),
        Err(Error::invalid_argument(
            "Polygon::box_polygon",
            "half_width",
            "a finite value greater than zero",
        ))
    );
    assert_eq!(body.shape_count().unwrap(), 0);

    let shape = body
        .create_polygon_from_points(
            &ShapeDef::default(),
            [[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]],
            0.0,
        )
        .unwrap();
    assert_eq!(body.shapes().unwrap(), vec![shape]);
}

#[test]
fn body_attachment_ids_remain_typed_values() {
    fn assert_shape(_: ShapeId) {}
    fn assert_joint(_: JointId) {}

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
                .body_def(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape = world
        .body(body_a)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle(Vec2::ZERO, 0.5).unwrap(),
        )
        .unwrap();
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();

    assert_shape(shape);
    assert_joint(joint);
}
