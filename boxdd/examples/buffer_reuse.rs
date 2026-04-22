use boxdd::{Aabb, BodyBuilder, QueryFilter, ShapeDef, World, WorldDef, shapes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build())?;

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::box_polygon(10.0, 0.5),
    );

    let dynamic = world.create_body_id(
        BodyBuilder::new()
            .body_type(boxdd::BodyType::Dynamic)
            .position([0.0, 2.0])
            .build(),
    );
    let _dynamic_shape = world.create_polygon_shape_for(
        dynamic,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    let query_aabb = Aabb::new([-1.0, -1.0], [1.0, 3.0]);
    let filter = QueryFilter::default();

    // Pre-reserve once, then keep reusing the same buffers every frame.
    let mut overlap_hits = Vec::with_capacity(8);
    let mut ray_hits = Vec::with_capacity(8);
    let initial_overlap_capacity = overlap_hits.capacity();
    let initial_ray_capacity = ray_hits.capacity();

    for frame in 0..3 {
        world.overlap_aabb_into(query_aabb, filter, &mut overlap_hits);
        world.cast_ray_all_into([0.0, 5.0], [0.0, -10.0], filter, &mut ray_hits);

        println!(
            "frame {frame}: overlap_hits={}, ray_hits={}",
            overlap_hits.len(),
            ray_hits.len()
        );
    }

    assert_eq!(overlap_hits.capacity(), initial_overlap_capacity);
    assert_eq!(ray_hits.capacity(), initial_ray_capacity);

    let mut visited = 0usize;
    let visited_all = world.visit_overlap_aabb(query_aabb, filter, |_| {
        visited += 1;
        true
    });
    assert!(visited_all);

    println!("visited overlap hits without building another Vec: {visited}");

    Ok(())
}
