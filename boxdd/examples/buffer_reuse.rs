use boxdd::{
    Aabb, BodyBuilder, Position, QueryFilter, RayQueryBuffer, ShapeDef, ShapeQueryBuffer, shapes,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -9.8])
            .build()?,
    )?;

    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ground_shape = world
        .body(ground)?
        .create_polygon(&ShapeDef::default(), &shapes::box_polygon(10.0, 0.5)?)?;

    let dynamic = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(boxdd::BodyType::Dynamic)
            .position([0.0, 2.0])
            .build()?,
    )?;
    let _dynamic_shape = world.body(dynamic)?.create_polygon(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::box_polygon(0.5, 0.5)?,
    )?;

    for _ in 0..10 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    let query_aabb = Aabb::new([-1.0, -1.0], [1.0, 3.0])?;
    let filter = QueryFilter::default();

    // Pre-reserve once, then keep reusing the same buffers every frame.
    let mut overlap_hits = ShapeQueryBuffer::with_capacity(8)?;
    let mut ray_hits = RayQueryBuffer::with_capacity(8)?;
    let initial_overlap_capacity = overlap_hits.capacity();
    let initial_ray_capacity = ray_hits.capacity();
    let query = world.query()?;

    for frame in 0..3 {
        query.overlap_aabb_into(Position::ZERO, query_aabb, filter, &mut overlap_hits)?;
        query.cast_ray_all_into(Position::new(0.0, 5.0), [0.0, -10.0], filter, &mut ray_hits)?;

        println!(
            "frame {frame}: overlap_hits={}, ray_hits={}",
            overlap_hits.len(),
            ray_hits.len()
        );
    }

    assert_eq!(overlap_hits.capacity(), initial_overlap_capacity);
    assert_eq!(ray_hits.capacity(), initial_ray_capacity);

    let mut visited = 0usize;
    let visited_all = query.visit_overlap_aabb(Position::ZERO, query_aabb, filter, |_| {
        visited += 1;
        true
    })?;
    assert!(visited_all);

    println!("visited overlap hits without building another Vec: {visited}");

    Ok(())
}
