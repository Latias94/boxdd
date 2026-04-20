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

    world.cast_ray_all_into(
        [50.0_f32, 50.0],
        [1.0, 0.0],
        QueryFilter::default(),
        &mut ray_hits,
    );
    assert!(ray_hits.is_empty());
    assert_eq!(ray_hits.as_ptr(), ray_hits_ptr);
}
