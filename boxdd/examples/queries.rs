use boxdd::{Aabb, BodyBuilder, QueryFilter, ShapeDef, Vec2, World, WorldDef, shapes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -9.8]).build())?;

    let solid = ShapeDef::builder().density(0.0).build();
    for (x, y, hx, hy) in [
        (0.0_f32, 2.0, 0.6, 0.6),
        (1.3, 2.3, 0.5, 0.5),
        (3.5, 2.0, 0.5, 0.8),
    ] {
        let body = world.create_body_id(BodyBuilder::new().position([x, y]).build());
        let _ = world.create_polygon_shape_for(body, &solid, &shapes::box_polygon(hx, hy));
    }

    let filter = QueryFilter::default();
    let aabb = Aabb::from_center_half_extents([0.8_f32, 2.1], [1.6, 0.9]);

    let owned_hits = world.overlap_aabb(aabb, filter);

    let mut reused_hits = Vec::new();
    world.overlap_aabb_into(aabb, filter, &mut reused_hits);

    let mut visited_hits = 0usize;
    let visited_all = world.visit_overlap_aabb(aabb, filter, |_| {
        visited_hits += 1;
        true
    });

    let mut stopped_early = false;
    let completed = world.visit_overlap_aabb(aabb, filter, |_| {
        stopped_early = true;
        false
    });

    let polygon_hits = world.overlap_polygon_points(
        [
            Vec2::new(-0.8, 1.2),
            Vec2::new(0.8, 1.2),
            Vec2::new(0.8, 2.8),
            Vec2::new(-0.8, 2.8),
        ],
        0.01,
        filter,
    );

    let offset_hits = world.overlap_polygon_points_with_offset(
        [
            Vec2::new(-0.7, -0.7),
            Vec2::new(0.7, -0.7),
            Vec2::new(0.7, 0.7),
            Vec2::new(-0.7, 0.7),
        ],
        0.01,
        [1.3_f32, 2.3],
        0.0_f32,
        filter,
    );

    println!(
        "overlap_aabb: owned={} reused={} visited={} completed={}",
        owned_hits.len(),
        reused_hits.len(),
        visited_hits,
        visited_all
    );
    println!(
        "visit_overlap_aabb early-exit: stopped={} completed={}",
        stopped_early, completed
    );
    println!(
        "polygon overlap hits: direct={} offset={}",
        polygon_hits.len(),
        offset_hits.len()
    );

    Ok(())
}
