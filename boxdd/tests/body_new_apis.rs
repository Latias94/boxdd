use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn same_shape_id(a: ShapeId, b: ShapeId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

fn same_joint_id(a: JointId, b: JointId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[test]
fn body_runtime_controls_and_enumeration_are_available_across_handle_and_world_apis() {
    let mut world = World::new(WorldDef::default()).unwrap();

    let body_id = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 1.0])
            .angle(0.5)
            .enable_sleep(true)
            .build(),
    );
    let other_body_id = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 1.0])
            .build(),
    );

    let shape_a = world.create_circle_shape_for(
        body_id,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let shape_b = world.create_polygon_shape_for(
        body_id,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.25, 0.75),
    );
    let joint = world
        .revolute(body_id, other_body_id)
        .anchor_world([0.5_f32, 1.0])
        .build_owned();
    let joint_id = joint.id();

    {
        let mut body = world.body(body_id).expect("body should still be valid");

        let rotation = body.rotation();
        assert!(approx_eq(rotation.angle(), 0.5, 1.0e-6));
        assert!(approx_eq(body.try_rotation().unwrap().angle(), 0.5, 1.0e-6));
        assert!(approx_eq(
            Rot::from(body.rotation_raw()).angle(),
            rotation.angle(),
            1.0e-6
        ));
        assert!(approx_eq(
            Rot::from(body.try_rotation_raw().unwrap()).angle(),
            rotation.angle(),
            1.0e-6
        ));

        assert!(body.is_sleep_enabled());
        assert!(body.try_is_sleep_enabled().unwrap());
        body.enable_sleep(false);
        assert!(!body.is_sleep_enabled());
        body.try_enable_sleep(true).unwrap();
        assert!(body.is_sleep_enabled());

        body.set_sleep_threshold(0.25);
        assert!(approx_eq(body.sleep_threshold(), 0.25, 1.0e-6));
        body.try_set_sleep_threshold(0.5).unwrap();
        assert!(approx_eq(body.try_sleep_threshold().unwrap(), 0.5, 1.0e-6));

        assert!(body.is_awake());
        assert!(body.try_is_awake().unwrap());
        body.set_awake(false);
        assert!(!body.is_awake());
        body.try_set_awake(true).unwrap();
        assert!(body.is_awake());

        assert!(body.is_enabled());
        assert!(body.try_is_enabled().unwrap());
        body.disable();
        assert!(!body.is_enabled());
        body.try_enable().unwrap();
        assert!(body.is_enabled());

        assert!(!body.is_bullet());
        assert!(!body.try_is_bullet().unwrap());
        body.set_bullet(true);
        assert!(body.is_bullet());
        body.try_set_bullet(false).unwrap();
        assert!(!body.is_bullet());

        assert_eq!(body.name().as_deref(), Some(""));
        body.set_name("runtime-body");
        assert_eq!(body.name().as_deref(), Some("runtime-body"));
        assert_eq!(body.try_name().unwrap().as_deref(), Some("runtime-body"));

        body.enable_contact_events(true);
        body.try_enable_contact_events(true).unwrap();
        body.enable_hit_events(true);
        body.try_enable_hit_events(true).unwrap();

        assert_eq!(body.shape_count(), 2);
        assert_eq!(body.try_shape_count().unwrap(), 2);
        let body_shapes = body.shapes();
        assert_eq!(body_shapes.len(), 2);
        assert!(
            body_shapes
                .iter()
                .copied()
                .any(|id| same_shape_id(id, shape_a))
        );
        assert!(
            body_shapes
                .iter()
                .copied()
                .any(|id| same_shape_id(id, shape_b))
        );

        let mut shape_buf = Vec::with_capacity(4);
        let shape_buf_ptr = shape_buf.as_ptr();
        body.shapes_into(&mut shape_buf);
        assert_eq!(shape_buf.as_ptr(), shape_buf_ptr);
        assert_eq!(shape_buf.len(), 2);
        body.try_shapes_into(&mut shape_buf).unwrap();
        assert_eq!(shape_buf.as_ptr(), shape_buf_ptr);
        assert_eq!(shape_buf.len(), 2);

        assert_eq!(body.joint_count(), 1);
        assert_eq!(body.try_joint_count().unwrap(), 1);
        let body_joints = body.joints();
        assert_eq!(body_joints.len(), 1);
        assert!(same_joint_id(body_joints[0], joint_id));

        let mut joint_buf = Vec::with_capacity(4);
        let joint_buf_ptr = joint_buf.as_ptr();
        body.joints_into(&mut joint_buf);
        assert_eq!(joint_buf.as_ptr(), joint_buf_ptr);
        assert_eq!(joint_buf.len(), 1);
        body.try_joints_into(&mut joint_buf).unwrap();
        assert_eq!(joint_buf.as_ptr(), joint_buf_ptr);
        assert_eq!(joint_buf.len(), 1);
    }

    assert!(approx_eq(world.body_rotation(body_id).angle(), 0.5, 1.0e-6));
    assert!(approx_eq(
        world.try_body_rotation(body_id).unwrap().angle(),
        0.5,
        1.0e-6
    ));

    assert!(world.body_is_sleep_enabled(body_id));
    assert!(world.try_body_is_sleep_enabled(body_id).unwrap());
    world.body_enable_sleep(body_id, false);
    assert!(!world.body_is_sleep_enabled(body_id));
    world.try_body_enable_sleep(body_id, true).unwrap();
    assert!(world.body_is_sleep_enabled(body_id));

    assert!(approx_eq(world.body_sleep_threshold(body_id), 0.5, 1.0e-6));
    world.set_body_sleep_threshold(body_id, 0.75);
    assert!(approx_eq(
        world.try_body_sleep_threshold(body_id).unwrap(),
        0.75,
        1.0e-6
    ));

    assert!(world.body_is_awake(body_id));
    assert!(world.try_body_is_awake(body_id).unwrap());
    world.set_body_awake(body_id, false);
    assert!(!world.body_is_awake(body_id));
    world.try_set_body_awake(body_id, true).unwrap();
    assert!(world.body_is_awake(body_id));

    assert!(world.body_is_enabled(body_id));
    assert!(world.try_body_is_enabled(body_id).unwrap());
    world.disable_body(body_id);
    assert!(!world.body_is_enabled(body_id));
    world.try_enable_body(body_id).unwrap();
    assert!(world.body_is_enabled(body_id));

    assert!(!world.body_is_bullet(body_id));
    assert!(!world.try_body_is_bullet(body_id).unwrap());
    world.set_body_bullet(body_id, true);
    assert!(world.body_is_bullet(body_id));
    world.try_set_body_bullet(body_id, false).unwrap();
    assert!(!world.body_is_bullet(body_id));

    assert_eq!(world.body_name(body_id).as_deref(), Some("runtime-body"));
    assert_eq!(
        world.try_body_name(body_id).unwrap().as_deref(),
        Some("runtime-body")
    );

    world.body_enable_contact_events(body_id, true);
    world.try_body_enable_contact_events(body_id, true).unwrap();
    world.body_enable_hit_events(body_id, true);
    world.try_body_enable_hit_events(body_id, true).unwrap();

    assert_eq!(world.body_shape_count(body_id), 2);
    assert_eq!(world.try_body_shape_count(body_id).unwrap(), 2);
    let world_shapes = world.body_shapes(body_id);
    assert_eq!(world_shapes.len(), 2);
    assert!(
        world_shapes
            .iter()
            .copied()
            .any(|id| same_shape_id(id, shape_a))
    );
    assert!(
        world_shapes
            .iter()
            .copied()
            .any(|id| same_shape_id(id, shape_b))
    );

    let mut world_shape_buf = Vec::with_capacity(4);
    let world_shape_buf_ptr = world_shape_buf.as_ptr();
    world.body_shapes_into(body_id, &mut world_shape_buf);
    assert_eq!(world_shape_buf.as_ptr(), world_shape_buf_ptr);
    assert_eq!(world_shape_buf.len(), 2);
    world
        .try_body_shapes_into(body_id, &mut world_shape_buf)
        .unwrap();
    assert_eq!(world_shape_buf.as_ptr(), world_shape_buf_ptr);
    assert_eq!(world_shape_buf.len(), 2);

    assert_eq!(world.body_joint_count(body_id), 1);
    assert_eq!(world.try_body_joint_count(body_id).unwrap(), 1);
    let world_joints = world.body_joints(body_id);
    assert_eq!(world_joints.len(), 1);
    assert!(same_joint_id(world_joints[0], joint_id));

    let mut world_joint_buf = Vec::with_capacity(4);
    let world_joint_buf_ptr = world_joint_buf.as_ptr();
    world.body_joints_into(body_id, &mut world_joint_buf);
    assert_eq!(world_joint_buf.as_ptr(), world_joint_buf_ptr);
    assert_eq!(world_joint_buf.len(), 1);
    world
        .try_body_joints_into(body_id, &mut world_joint_buf)
        .unwrap();
    assert_eq!(world_joint_buf.as_ptr(), world_joint_buf_ptr);
    assert_eq!(world_joint_buf.len(), 1);
}
