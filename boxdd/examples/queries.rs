use boxdd::{
    Aabb, BodyBuilder, Position, QueryFilter, ShapeDef, ShapeProxy, ShapeQueryBuffer, Transform,
    Vec2, shapes,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -9.8])
            .build()?,
    )?;

    let solid = ShapeDef::builder().density(0.0).build()?;
    for (x, y, hx, hy) in [
        (0.0_f32, 2.0, 0.6, 0.6),
        (1.3, 2.3, 0.5, 0.5),
        (3.5, 2.0, 0.5, 0.8),
    ] {
        let body = world.create_body(
            BodyBuilder::from(foundation.body_def())
                .position([x, y])
                .build()?,
        )?;
        let _ = world
            .body(body)?
            .create_polygon(&solid, &shapes::box_polygon(hx, hy)?)?;
    }

    let filter = QueryFilter::default();
    let aabb = Aabb::from_center_half_extents([0.8_f32, 2.1], [1.6, 0.9])?;

    let query = world.query()?;
    let owned_hits = query.overlap_aabb(Position::ZERO, aabb, filter)?;

    let mut reused_hits = ShapeQueryBuffer::new();
    query.overlap_aabb_into(Position::ZERO, aabb, filter, &mut reused_hits)?;

    let mut visited_hits = 0usize;
    let visited_all = query.visit_overlap_aabb(Position::ZERO, aabb, filter, |_| {
        visited_hits += 1;
        true
    })?;

    let mut stopped_early = false;
    let completed = query.visit_overlap_aabb(Position::ZERO, aabb, filter, |_| {
        stopped_early = true;
        false
    })?;

    let polygon_proxy = ShapeProxy::new(
        [
            Vec2::new(-0.8, 1.2),
            Vec2::new(0.8, 1.2),
            Vec2::new(0.8, 2.8),
            Vec2::new(-0.8, 2.8),
        ],
        0.01,
    )?;
    let polygon_hits = query.overlap_shape(Position::ZERO, polygon_proxy, filter)?;

    let offset_proxy = ShapeProxy::offset_from_points(
        [
            Vec2::new(-0.7, -0.7),
            Vec2::new(0.7, -0.7),
            Vec2::new(0.7, 0.7),
            Vec2::new(-0.7, 0.7),
        ],
        0.01,
        Transform::from_pos_angle([1.3_f32, 2.3], 0.0)?,
    )?;
    let offset_hits = query.overlap_shape(Position::ZERO, offset_proxy, filter)?;

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
