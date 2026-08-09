use boxdd::{prelude::*, shapes};

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shape_query_buffer_reuses_raw_and_mapped_storage_transactionally() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut expected = Vec::new();
    for index in 0..12 {
        let body = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .position([index as f32 - 6.0, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        expected.push(
            world
                .body(body)
                .unwrap()
                .create_centered_circle(&ShapeDef::default(), 0.25)
                .unwrap(),
        );
    }

    let query = world.query().unwrap();
    let mut buffer = ShapeQueryBuffer::with_capacity(16).unwrap();
    let capacity = buffer.capacity();
    query
        .overlap_aabb_into(
            Position::ZERO,
            Aabb::new([-10.0_f32, -2.0], [10.0, 2.0]).unwrap(),
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert_eq!(buffer.len(), expected.len());
    assert!(expected.iter().all(|id| buffer.as_slice().contains(id)));
    assert_eq!(buffer.capacity(), capacity);
    let mapped_ptr = buffer.as_slice().as_ptr();

    query
        .overlap_aabb_into(
            Position::ZERO,
            Aabb::new([-10.0_f32, -2.0], [10.0, 2.0]).unwrap(),
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.as_slice().as_ptr(), mapped_ptr);

    assert_eq!(
        Aabb::new([1.0_f32, 1.0], [-1.0, -1.0]).unwrap_err(),
        Error::invalid_argument("Aabb::new", "aabb", "finite ordered lower and upper bounds",)
    );
    assert_eq!(buffer.len(), expected.len());
    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.as_slice().as_ptr(), mapped_ptr);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ray_query_buffer_reuses_storage_for_repeated_casts() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    for index in 0..8 {
        let body = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .position([index as f32 * 1.5, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        world
            .body(body)
            .unwrap()
            .create_box(&ShapeDef::default(), 0.4, 0.4)
            .unwrap();
    }

    let query = world.query().unwrap();
    let mut buffer = RayQueryBuffer::with_capacity(16).unwrap();
    let capacity = buffer.capacity();
    query
        .cast_ray_all_into(
            Position::new(-2.0, 0.0),
            [16.0_f32, 0.0],
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert_eq!(buffer.len(), 8);
    assert_eq!(buffer.capacity(), capacity);
    let mapped_ptr = buffer.as_slice().as_ptr();

    query
        .cast_ray_all_into(
            Position::new(-2.0, 0.0),
            [16.0_f32, 0.0],
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert_eq!(buffer.len(), 8);
    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.as_slice().as_ptr(), mapped_ptr);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn mover_query_buffer_reuses_storage_for_collision_planes() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let ground = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
        .body(ground)
        .unwrap()
        .create_polygon(
            &ShapeDef::default(),
            &shapes::box_polygon(20.0, 0.5).unwrap(),
        )
        .unwrap();

    let query = world.query().unwrap();
    let mut buffer = MoverQueryBuffer::with_capacity(8).unwrap();
    let capacity = buffer.capacity();
    query
        .collide_mover_into(
            Position::ZERO,
            [0.0_f32, 0.7],
            [0.0_f32, 1.5],
            0.25,
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert!(!buffer.is_empty());
    assert_eq!(buffer.capacity(), capacity);
    let mapped_ptr = buffer.as_slice().as_ptr();

    query
        .collide_mover_into(
            Position::ZERO,
            [0.0_f32, 0.7],
            [0.0_f32, 1.5],
            0.25,
            QueryFilter::default(),
            &mut buffer,
        )
        .unwrap();
    assert!(!buffer.is_empty());
    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.as_slice().as_ptr(), mapped_ptr);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn debug_draw_collect_into_reuses_command_and_vertex_buffers() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_box(&ShapeDef::default(), 0.75, 0.5)
        .unwrap();

    let options = DebugDrawOptions {
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

    let baseline = world.debug_draw_collect(options).unwrap();
    assert!(!baseline.is_empty());
    let mut commands = Vec::with_capacity(baseline.len() + 4);
    let commands_ptr = commands.as_ptr();
    world
        .debug_draw_collect_into(&mut commands, options)
        .unwrap();
    assert_eq!(commands.len(), baseline.len());
    assert_eq!(commands.as_ptr(), commands_ptr);

    let vertices_ptr = commands
        .iter()
        .find_map(|command| match command {
            DebugDrawCmd::Polygon { vertices, .. }
            | DebugDrawCmd::SolidPolygon { vertices, .. } => Some(vertices.as_ptr()),
            _ => None,
        })
        .expect("expected a polygon debug draw command");

    world
        .debug_draw_collect_into(&mut commands, options)
        .unwrap();
    let reused_vertices_ptr = commands
        .iter()
        .find_map(|command| match command {
            DebugDrawCmd::Polygon { vertices, .. }
            | DebugDrawCmd::SolidPolygon { vertices, .. } => Some(vertices.as_ptr()),
            _ => None,
        })
        .expect("expected a polygon debug draw command");
    assert_eq!(commands.as_ptr(), commands_ptr);
    assert_eq!(reused_vertices_ptr, vertices_ptr);
}

#[test]
fn completed_step_event_views_clone_into_caller_owned_storage() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let moving_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 4.0])
                .linear_velocity([1.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(moving_body)
        .unwrap()
        .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 0.35)
        .unwrap();

    let completed = world.step(1.0 / 60.0, 4).unwrap();
    let events = completed.body_events().unwrap();
    assert!(!events.is_empty());

    let mut owned = Vec::with_capacity(events.len() + 4);
    let owned_ptr = owned.as_ptr();
    events.clone_into(&mut owned).unwrap();
    assert_eq!(owned.len(), events.len());
    assert_eq!(owned.as_ptr(), owned_ptr);

    events.clone_into(&mut owned).unwrap();
    assert_eq!(owned.len(), events.len());
    assert_eq!(owned.as_ptr(), owned_ptr);
}
