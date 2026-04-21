use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn shape_id_fields(id: ShapeId) -> (i32, u16, u16) {
    (id.index1, id.world0, id.generation)
}

#[test]
fn world_basics_and_queries() {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .build(),
    )
    .unwrap();

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _gs = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(20.0, 0.5),
    );

    // Dynamic body above ground
    let b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 5.0])
            .build(),
    );
    let _s = world.create_polygon_shape_for(
        b,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    // Step and ensure body moved downward due to gravity
    let y0 = world.body_position(b).y;
    for _ in 0..30 {
        world.step(1.0 / 60.0, 4);
    }
    let y1 = world.body_position(b).y;
    assert!(y1 < y0, "body should fall: y0={} y1={}", y0, y1);

    // AABB overlap near origin should at least find ground
    let ids = world.overlap_aabb(Aabb::new([-2.0, -2.0], [2.0, 2.0]), QueryFilter::default());
    assert!(!ids.is_empty());

    let mut reused_ids = Vec::with_capacity(16);
    let reused_ids_ptr = reused_ids.as_ptr();
    world.overlap_aabb_into(
        Aabb::new([-2.0, -2.0], [2.0, 2.0]),
        QueryFilter::default(),
        &mut reused_ids,
    );
    assert!(!reused_ids.is_empty());
    assert_eq!(reused_ids.as_ptr(), reused_ids_ptr);

    world.overlap_aabb_into(
        Aabb::new([50.0, 50.0], [51.0, 51.0]),
        QueryFilter::default(),
        &mut reused_ids,
    );
    assert!(reused_ids.is_empty());
    assert_eq!(reused_ids.as_ptr(), reused_ids_ptr);

    let handle = world.handle();
    let mut handle_ids = Vec::with_capacity(16);
    handle.overlap_aabb_into(
        Aabb::new([-2.0, -2.0], [2.0, 2.0]),
        QueryFilter::default(),
        &mut handle_ids,
    );
    assert_eq!(handle_ids.len(), ids.len());

    let mut visited_ids = Vec::new();
    let visited_complete = world.visit_overlap_aabb(
        Aabb::new([-2.0, -2.0], [2.0, 2.0]),
        QueryFilter::default(),
        |shape_id| {
            visited_ids.push(shape_id);
            true
        },
    );
    assert!(visited_complete);
    assert_eq!(
        visited_ids
            .iter()
            .copied()
            .map(shape_id_fields)
            .collect::<Vec<_>>(),
        ids.iter().copied().map(shape_id_fields).collect::<Vec<_>>()
    );

    let mut stopped_ids = Vec::new();
    let visited_complete = world.visit_overlap_aabb(
        Aabb::new([-2.0, -2.0], [2.0, 2.0]),
        QueryFilter::default(),
        |shape_id| {
            stopped_ids.push(shape_id);
            false
        },
    );
    assert!(!visited_complete);
    assert_eq!(stopped_ids.len(), 1);

    let mut empty_visit_count = 0;
    let visited_complete = world.visit_overlap_aabb(
        Aabb::new([50.0, 50.0], [51.0, 51.0]),
        QueryFilter::default(),
        |_| {
            empty_visit_count += 1;
            true
        },
    );
    assert!(visited_complete);
    assert_eq!(empty_visit_count, 0);

    let mut handle_visit_count = 0;
    let visited_complete = handle.visit_overlap_aabb(
        Aabb::new([-2.0, -2.0], [2.0, 2.0]),
        QueryFilter::default(),
        |_| {
            handle_visit_count += 1;
            true
        },
    );
    assert!(visited_complete);
    assert_eq!(handle_visit_count, ids.len());

    let mut try_handle_visit_count = 0;
    let visited_complete = handle
        .try_visit_overlap_aabb(
            Aabb::new([-2.0, -2.0], [2.0, 2.0]),
            QueryFilter::default(),
            |_| {
                try_handle_visit_count += 1;
                true
            },
        )
        .unwrap();
    assert!(visited_complete);
    assert_eq!(try_handle_visit_count, ids.len());

    // Raycast downward from above body should hit something
    let hit = world.cast_ray_closest([0.0_f32, 10.0], [0.0, -100.0], QueryFilter::default());
    assert!(hit.hit);
    assert!(hit.fraction >= 0.0 && hit.fraction <= 1.0);
    assert!(approx_eq(hit.normal.y.abs(), 1.0, 1e-3) || approx_eq(hit.normal.x.abs(), 1.0, 1e-3));

    let mut ray_hits = Vec::with_capacity(16);
    let ray_hits_ptr = ray_hits.as_ptr();
    world.cast_ray_all_into(
        [0.0_f32, 10.0],
        [0.0, -100.0],
        QueryFilter::default(),
        &mut ray_hits,
    );
    assert!(!ray_hits.is_empty());
    assert_eq!(ray_hits.as_ptr(), ray_hits_ptr);
    let world_ray_hit_count = ray_hits.len();

    world.cast_ray_all_into(
        [50.0_f32, 50.0],
        [1.0, 0.0],
        QueryFilter::default(),
        &mut ray_hits,
    );
    assert!(ray_hits.is_empty());
    assert_eq!(ray_hits.as_ptr(), ray_hits_ptr);

    let handle_hit =
        handle.cast_ray_closest([0.0_f32, 10.0], [0.0, -100.0], QueryFilter::default());
    assert_eq!(handle_hit.hit, hit.hit);
    assert!(approx_eq(handle_hit.fraction, hit.fraction, 1e-6));

    let handle_all = handle.cast_ray_all([0.0_f32, 10.0], [0.0, -100.0], QueryFilter::default());
    assert_eq!(handle_all.len(), world_ray_hit_count);
}

#[test]
fn world_handle_queries_match_world_queries() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(20.0, 0.5),
    );

    let wall = world.create_body_id(BodyBuilder::new().position([1.0_f32, 1.0]).build());
    let _wall_shape = world.create_polygon_shape_for(
        wall,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(0.25, 1.0),
    );

    let handle = world.handle();
    world.enable_sleeping(false);
    world.enable_continuous(false);
    world.enable_warm_starting(false);
    world.set_restitution_threshold(0.75);
    world.set_hit_event_threshold(1.25);
    world.set_maximum_linear_speed(18.0);
    world.step(1.0 / 60.0, 4);

    assert_eq!(handle.gravity(), world.gravity());
    assert_eq!(handle.try_gravity().unwrap(), world.gravity());

    let world_counters = world.counters();
    let handle_counters = handle.counters();
    let try_handle_counters = handle.try_counters().unwrap();
    assert_eq!(handle_counters.body_count, world_counters.body_count);
    assert_eq!(handle_counters.shape_count, world_counters.shape_count);
    assert_eq!(handle_counters.contact_count, world_counters.contact_count);
    assert_eq!(handle_counters.joint_count, world_counters.joint_count);
    assert_eq!(handle_counters.task_count, world_counters.task_count);
    assert_eq!(try_handle_counters.body_count, handle_counters.body_count);
    assert_eq!(try_handle_counters.shape_count, handle_counters.shape_count);

    let world_profile = world.profile();
    assert_eq!(handle.profile(), world_profile);
    assert_eq!(handle.try_profile().unwrap(), world_profile);
    assert_eq!(handle.awake_body_count(), world.awake_body_count());
    assert_eq!(
        handle.try_awake_body_count().unwrap(),
        world.try_awake_body_count().unwrap()
    );
    assert_eq!(handle.is_sleeping_enabled(), world.is_sleeping_enabled());
    assert_eq!(
        handle.try_is_sleeping_enabled().unwrap(),
        world.try_is_sleeping_enabled().unwrap()
    );
    assert_eq!(
        handle.is_continuous_enabled(),
        world.is_continuous_enabled()
    );
    assert_eq!(
        handle.try_is_continuous_enabled().unwrap(),
        world.try_is_continuous_enabled().unwrap()
    );
    assert_eq!(
        handle.is_warm_starting_enabled(),
        world.is_warm_starting_enabled()
    );
    assert_eq!(
        handle.try_is_warm_starting_enabled().unwrap(),
        world.try_is_warm_starting_enabled().unwrap()
    );
    assert!(approx_eq(
        handle.restitution_threshold(),
        world.restitution_threshold(),
        1e-6
    ));
    assert!(approx_eq(
        handle.try_restitution_threshold().unwrap(),
        world.try_restitution_threshold().unwrap(),
        1e-6
    ));
    assert!(approx_eq(
        handle.hit_event_threshold(),
        world.hit_event_threshold(),
        1e-6
    ));
    assert!(approx_eq(
        handle.try_hit_event_threshold().unwrap(),
        world.try_hit_event_threshold().unwrap(),
        1e-6
    ));
    assert!(approx_eq(
        handle.maximum_linear_speed(),
        world.maximum_linear_speed(),
        1e-6
    ));
    assert!(approx_eq(
        handle.try_maximum_linear_speed().unwrap(),
        world.try_maximum_linear_speed().unwrap(),
        1e-6
    ));

    let world_overlap = world.overlap_polygon_points_with_offset(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 0.2],
        0.0_f32,
        QueryFilter::default(),
    );
    assert!(!world_overlap.is_empty());
    let handle_overlap = handle.overlap_polygon_points_with_offset(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 0.2],
        0.0_f32,
        QueryFilter::default(),
    );
    assert_eq!(handle_overlap.len(), world_overlap.len());
    for (handle_shape, world_shape) in handle_overlap.iter().zip(world_overlap.iter()) {
        assert_eq!(handle_shape.index1, world_shape.index1);
        assert_eq!(handle_shape.generation, world_shape.generation);
    }

    let world_plain_overlap = world.overlap_polygon_points(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        QueryFilter::default(),
    );
    let mut visited_plain_overlap = Vec::new();
    let visited_complete = world.visit_overlap_polygon_points(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        QueryFilter::default(),
        |shape_id| {
            visited_plain_overlap.push(shape_id);
            true
        },
    );
    assert!(visited_complete);
    assert_eq!(
        visited_plain_overlap
            .iter()
            .copied()
            .map(shape_id_fields)
            .collect::<Vec<_>>(),
        world_plain_overlap
            .iter()
            .copied()
            .map(shape_id_fields)
            .collect::<Vec<_>>()
    );

    let mut visited_offset_overlap = Vec::new();
    let visited_complete = world.visit_overlap_polygon_points_with_offset(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 0.2],
        0.0_f32,
        QueryFilter::default(),
        |shape_id| {
            visited_offset_overlap.push(shape_id);
            true
        },
    );
    assert!(visited_complete);
    assert_eq!(
        visited_offset_overlap
            .iter()
            .copied()
            .map(shape_id_fields)
            .collect::<Vec<_>>(),
        world_overlap
            .iter()
            .copied()
            .map(shape_id_fields)
            .collect::<Vec<_>>()
    );

    let mut handle_offset_stop_count = 0;
    let visited_complete = handle.visit_overlap_polygon_points_with_offset(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 0.2],
        0.0_f32,
        QueryFilter::default(),
        |_| {
            handle_offset_stop_count += 1;
            false
        },
    );
    assert!(!visited_complete);
    assert_eq!(handle_offset_stop_count, 1);

    let mut world_cast_hits = Vec::with_capacity(8);
    world.cast_shape_points_with_offset_into(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 1.5],
        0.0_f32,
        [0.0_f32, -2.0],
        QueryFilter::default(),
        &mut world_cast_hits,
    );
    let mut handle_cast_hits = Vec::with_capacity(8);
    handle.cast_shape_points_with_offset_into(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
        [0.0_f32, 1.5],
        0.0_f32,
        [0.0_f32, -2.0],
        QueryFilter::default(),
        &mut handle_cast_hits,
    );
    assert_eq!(handle_cast_hits.len(), world_cast_hits.len());
    for (world_hit, handle_hit) in world_cast_hits.iter().zip(handle_cast_hits.iter()) {
        assert_eq!(handle_hit.hit, world_hit.hit);
        assert_eq!(handle_hit.shape_id.index1, world_hit.shape_id.index1);
        assert_eq!(
            handle_hit.shape_id.generation,
            world_hit.shape_id.generation
        );
        assert!(approx_eq(handle_hit.fraction, world_hit.fraction, 1e-6));
        assert!(approx_eq(handle_hit.point.x, world_hit.point.x, 1e-6));
        assert!(approx_eq(handle_hit.point.y, world_hit.point.y, 1e-6));
        assert!(approx_eq(handle_hit.normal.x, world_hit.normal.x, 1e-6));
        assert!(approx_eq(handle_hit.normal.y, world_hit.normal.y, 1e-6));
    }

    let c1 = Vec2::new(0.0, 0.7);
    let c2 = Vec2::new(0.0, 1.5);
    let mut world_planes = Vec::with_capacity(8);
    world.collide_mover_into(c1, c2, 0.25, QueryFilter::default(), &mut world_planes);
    let mut handle_planes = Vec::with_capacity(8);
    handle.collide_mover_into(c1, c2, 0.25, QueryFilter::default(), &mut handle_planes);
    assert_eq!(handle_planes.len(), world_planes.len());
    for (world_plane, handle_plane) in world_planes.iter().zip(handle_planes.iter()) {
        assert_eq!(handle_plane.hit, world_plane.hit);
        assert_eq!(handle_plane.shape_id.index1, world_plane.shape_id.index1);
        assert_eq!(
            handle_plane.shape_id.generation,
            world_plane.shape_id.generation
        );
        assert!(approx_eq(
            handle_plane.plane.normal.x,
            world_plane.plane.normal.x,
            1e-6
        ));
        assert!(approx_eq(
            handle_plane.plane.normal.y,
            world_plane.plane.normal.y,
            1e-6
        ));
        assert!(approx_eq(
            handle_plane.plane.offset,
            world_plane.plane.offset,
            1e-6
        ));
        assert!(approx_eq(handle_plane.point.x, world_plane.point.x, 1e-6));
        assert!(approx_eq(handle_plane.point.y, world_plane.point.y, 1e-6));
    }
}
