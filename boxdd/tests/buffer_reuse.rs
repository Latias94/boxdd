use boxdd::{prelude::*, shapes};

#[test]
fn body_and_shape_contact_data_into_reuses_buffer() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(20.0, 0.5),
    );

    let body = world.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let shape = world.create_polygon_shape_for_owned(
        body.id(),
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    let mut body_contacts = Vec::with_capacity(8);
    let body_contacts_ptr = body_contacts.as_ptr();
    body.contact_data_into(&mut body_contacts);
    assert!(body_contacts.is_empty());
    assert_eq!(body_contacts.as_ptr(), body_contacts_ptr);

    let mut body_contacts_raw = Vec::with_capacity(8);
    let body_contacts_raw_ptr = body_contacts_raw.as_ptr();
    body.contact_data_into_raw(&mut body_contacts_raw);
    assert!(body_contacts_raw.is_empty());
    assert_eq!(body_contacts_raw.as_ptr(), body_contacts_raw_ptr);

    let mut shape_contacts = Vec::with_capacity(8);
    let shape_contacts_ptr = shape_contacts.as_ptr();
    shape.contact_data_into(&mut shape_contacts);
    assert!(shape_contacts.is_empty());
    assert_eq!(shape_contacts.as_ptr(), shape_contacts_ptr);

    let mut shape_contacts_raw = Vec::with_capacity(8);
    let shape_contacts_raw_ptr = shape_contacts_raw.as_ptr();
    shape.contact_data_into_raw(&mut shape_contacts_raw);
    assert!(shape_contacts_raw.is_empty());
    assert_eq!(shape_contacts_raw.as_ptr(), shape_contacts_raw_ptr);

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
        if !body.contact_data().is_empty() && !shape.contact_data().is_empty() {
            break;
        }
    }

    body.contact_data_into(&mut body_contacts);
    assert!(!body_contacts.is_empty());
    assert_eq!(body_contacts.as_ptr(), body_contacts_ptr);
    assert!(body_contacts[0].manifold.points().len() <= 2);
    body.try_contact_data_into(&mut body_contacts).unwrap();
    assert!(!body_contacts.is_empty());

    body.contact_data_into_raw(&mut body_contacts_raw);
    assert!(!body_contacts_raw.is_empty());
    assert_eq!(body_contacts_raw.as_ptr(), body_contacts_raw_ptr);
    body.try_contact_data_into_raw(&mut body_contacts_raw)
        .unwrap();
    assert!(!body_contacts_raw.is_empty());

    shape.contact_data_into(&mut shape_contacts);
    assert!(!shape_contacts.is_empty());
    assert_eq!(shape_contacts.as_ptr(), shape_contacts_ptr);
    assert!(shape_contacts[0].manifold.points().len() <= 2);
    shape.try_contact_data_into(&mut shape_contacts).unwrap();
    assert!(!shape_contacts.is_empty());

    shape.contact_data_into_raw(&mut shape_contacts_raw);
    assert!(!shape_contacts_raw.is_empty());
    assert_eq!(shape_contacts_raw.as_ptr(), shape_contacts_raw_ptr);
    shape
        .try_contact_data_into_raw(&mut shape_contacts_raw)
        .unwrap();
    assert!(!shape_contacts_raw.is_empty());
}

#[test]
fn sensor_overlap_into_reuses_buffer() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let sensor_body = world.create_body_id(BodyBuilder::new().position([0.0_f32, 1.5]).build());
    let sensor_shape = world.create_polygon_shape_for_owned(
        sensor_body,
        &ShapeDef::builder()
            .density(0.0)
            .sensor(true)
            .enable_sensor_events(true)
            .build(),
        &shapes::box_polygon(2.0, 0.3),
    );

    let visitor_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let _visitor_shape = world.create_circle_shape_for(
        visitor_body,
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.25),
    );

    let mut shape_overlaps = Vec::with_capacity(8);
    let shape_overlaps_ptr = shape_overlaps.as_ptr();
    sensor_shape.sensor_overlaps_into(&mut shape_overlaps);
    assert!(shape_overlaps.is_empty());
    assert_eq!(shape_overlaps.as_ptr(), shape_overlaps_ptr);

    let mut world_overlaps = Vec::with_capacity(8);
    let world_overlaps_ptr = world_overlaps.as_ptr();
    world.shape_sensor_overlaps_into(sensor_shape.id(), &mut world_overlaps);
    assert!(world_overlaps.is_empty());
    assert_eq!(world_overlaps.as_ptr(), world_overlaps_ptr);

    for _ in 0..240 {
        world.step(1.0 / 120.0, 8);
        if !world.shape_sensor_overlaps(sensor_shape.id()).is_empty() {
            break;
        }
    }

    sensor_shape.sensor_overlaps_into(&mut shape_overlaps);
    assert!(!shape_overlaps.is_empty());
    assert_eq!(shape_overlaps.as_ptr(), shape_overlaps_ptr);
    sensor_shape
        .try_sensor_overlaps_into(&mut shape_overlaps)
        .unwrap();
    assert!(!shape_overlaps.is_empty());

    let mut shape_overlaps_valid = Vec::with_capacity(8);
    let shape_overlaps_valid_ptr = shape_overlaps_valid.as_ptr();
    sensor_shape.sensor_overlaps_valid_into(&mut shape_overlaps_valid);
    assert!(!shape_overlaps_valid.is_empty());
    assert!(shape_overlaps_valid.len() <= shape_overlaps.len());
    assert_eq!(shape_overlaps_valid.as_ptr(), shape_overlaps_valid_ptr);
    sensor_shape
        .try_sensor_overlaps_valid_into(&mut shape_overlaps_valid)
        .unwrap();

    world.shape_sensor_overlaps_into(sensor_shape.id(), &mut world_overlaps);
    assert!(!world_overlaps.is_empty());
    assert_eq!(world_overlaps.as_ptr(), world_overlaps_ptr);
    world
        .try_shape_sensor_overlaps_into(sensor_shape.id(), &mut world_overlaps)
        .unwrap();

    let mut world_overlaps_valid = Vec::with_capacity(8);
    let world_overlaps_valid_ptr = world_overlaps_valid.as_ptr();
    world.shape_sensor_overlaps_valid_into(sensor_shape.id(), &mut world_overlaps_valid);
    assert!(!world_overlaps_valid.is_empty());
    assert!(world_overlaps_valid.len() <= world_overlaps.len());
    assert_eq!(world_overlaps_valid.as_ptr(), world_overlaps_valid_ptr);
    world
        .try_shape_sensor_overlaps_valid_into(sensor_shape.id(), &mut world_overlaps_valid)
        .unwrap();
}

#[test]
fn chain_segments_into_reuses_buffer() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let chain = world.create_chain_for_owned(
        body,
        &boxdd::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );

    let baseline = chain.segments();
    assert!(!baseline.is_empty());

    let mut segments = Vec::with_capacity(8);
    let segments_ptr = segments.as_ptr();
    chain.segments_into(&mut segments);
    assert_eq!(segments.len(), baseline.len());
    assert_eq!(segments.as_ptr(), segments_ptr);

    chain.try_segments_into(&mut segments).unwrap();
    assert_eq!(segments.len(), baseline.len());
    assert_eq!(segments.as_ptr(), segments_ptr);
}

#[test]
fn debug_draw_collect_into_reuses_command_and_vertex_buffers() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let _shape = world.create_polygon_shape_for_owned(
        body,
        &ShapeDef::default(),
        &shapes::box_polygon(0.75, 0.5),
    );

    let opts = DebugDrawOptions {
        draw_joints: false,
        draw_joint_extras: false,
        draw_bounds: false,
        draw_mass: false,
        draw_body_names: false,
        draw_contacts: false,
        draw_graph_colors: false,
        draw_contact_features: false,
        draw_contact_normals: false,
        draw_contact_forces: false,
        draw_friction_forces: false,
        draw_islands: false,
        ..DebugDrawOptions::default()
    };

    let baseline = world.debug_draw_collect(opts);
    assert!(!baseline.is_empty());

    let mut cmds = Vec::with_capacity(baseline.len() + 4);
    let cmds_ptr = cmds.as_ptr();
    world.debug_draw_collect_into(&mut cmds, opts);
    assert_eq!(cmds.len(), baseline.len());
    assert_eq!(cmds.as_ptr(), cmds_ptr);

    let vertices_ptr = cmds
        .iter()
        .find_map(|cmd| match cmd {
            DebugDrawCmd::Polygon { vertices, .. }
            | DebugDrawCmd::SolidPolygon { vertices, .. } => Some(vertices.as_ptr()),
            _ => None,
        })
        .expect("expected a polygon debug draw command");

    world.debug_draw_collect_into(&mut cmds, opts);
    assert_eq!(cmds.len(), baseline.len());
    assert_eq!(cmds.as_ptr(), cmds_ptr);

    let reused_vertices_ptr = cmds
        .iter()
        .find_map(|cmd| match cmd {
            DebugDrawCmd::Polygon { vertices, .. }
            | DebugDrawCmd::SolidPolygon { vertices, .. } => Some(vertices.as_ptr()),
            _ => None,
        })
        .expect("expected a polygon debug draw command");

    assert_eq!(reused_vertices_ptr, vertices_ptr);
}

#[test]
fn shape_type_uses_safe_enum_and_explicit_raw_escape_hatch() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let circle = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    assert_eq!(circle.shape_type(), ShapeType::Circle);
    assert_eq!(
        circle.shape_type_raw(),
        boxdd_sys::ffi::b2ShapeType_b2_circleShape
    );
}
