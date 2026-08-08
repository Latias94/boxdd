use boxdd::{
    BodyBuilder, Position, QueryFilter, RayQueryBuffer, ShapeDef, ShapeProxy, Transform, Vec2,
    shapes,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -9.8])
            .build()?,
    )?;

    let solid = ShapeDef::builder().density(0.0).build()?;

    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world
        .body(ground)?
        .create_polygon(&solid, &shapes::box_polygon(6.0, 0.5)?)?;

    let blocker = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0_f32, 2.5])
            .build()?,
    )?;
    let _ = world
        .body(blocker)?
        .create_polygon(&solid, &shapes::box_polygon(0.5, 0.5)?)?;

    let wall = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([1.8_f32, 1.4])
            .build()?,
    )?;
    let _ = world
        .body(wall)?
        .create_polygon(&solid, &shapes::box_polygon(0.4, 0.9)?)?;

    let filter = QueryFilter::default();
    let query = world.query()?;

    let closest = query.cast_ray_closest(Position::new(0.0, 5.0), [0.0, -8.0], filter)?;

    let mut ray_hits = RayQueryBuffer::with_capacity(8)?;
    query.cast_ray_all_into(Position::new(0.0, 5.0), [0.0, -8.0], filter, &mut ray_hits)?;

    let sweep_proxy = ShapeProxy::new(
        [
            Vec2::new(-1.6, 1.0),
            Vec2::new(-0.8, 1.0),
            Vec2::new(-0.8, 1.8),
            Vec2::new(-1.6, 1.8),
        ],
        0.02,
    )?;
    let mut sweep_hits = RayQueryBuffer::with_capacity(8)?;
    query.cast_shape_into(
        Position::ZERO,
        sweep_proxy,
        [3.6_f32, 0.0],
        filter,
        &mut sweep_hits,
    )?;

    let offset_proxy = ShapeProxy::offset_from_points(
        [
            Vec2::new(-0.4, -0.3),
            Vec2::new(0.4, -0.3),
            Vec2::new(0.4, 0.3),
            Vec2::new(-0.4, 0.3),
        ],
        0.02,
        Transform::from_pos_angle([-1.2_f32, 3.0], 0.35)?,
    )?;
    let mut offset_hits = RayQueryBuffer::with_capacity(8)?;
    query.cast_shape_into(
        Position::ZERO,
        offset_proxy,
        [3.5_f32, -1.6],
        filter,
        &mut offset_hits,
    )?;

    let sweep_min_fraction = sweep_hits.iter().map(|h| h.fraction).fold(1.0, f32::min);
    let offset_min_fraction = offset_hits.iter().map(|h| h.fraction).fold(1.0, f32::min);

    println!(
        "cast_ray_closest: hit={} fraction={:.3}",
        closest.is_some(),
        closest.map_or(1.0, |hit| hit.fraction)
    );
    println!("cast_ray_all_into hits: {}", ray_hits.len());
    println!(
        "cast_shape_into hits: {} earliest_fraction={:.3}",
        sweep_hits.len(),
        sweep_min_fraction
    );
    println!(
        "offset cast_shape_into hits: {} earliest_fraction={:.3}",
        offset_hits.len(),
        offset_min_fraction
    );

    Ok(())
}
