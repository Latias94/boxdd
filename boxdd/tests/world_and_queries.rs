use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
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
