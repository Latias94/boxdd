use boxdd::{BodyBuilder, QueryFilter, ShapeDef, Vec2, World, WorldDef, shapes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -9.8]).build())?;

    let solid = ShapeDef::builder().density(0.0).build();

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_polygon_shape_for(ground, &solid, &shapes::box_polygon(6.0, 0.5));

    let blocker = world.create_body_id(BodyBuilder::new().position([0.0_f32, 2.5]).build());
    let _ = world.create_polygon_shape_for(blocker, &solid, &shapes::box_polygon(0.5, 0.5));

    let wall = world.create_body_id(BodyBuilder::new().position([1.8_f32, 1.4]).build());
    let _ = world.create_polygon_shape_for(wall, &solid, &shapes::box_polygon(0.4, 0.9));

    let filter = QueryFilter::default();

    let closest = world.cast_ray_closest([0.0_f32, 5.0], [0.0, -8.0], filter);

    let mut ray_hits = Vec::with_capacity(8);
    world.cast_ray_all_into([0.0_f32, 5.0], [0.0, -8.0], filter, &mut ray_hits);

    let mut sweep_hits = Vec::with_capacity(8);
    world.cast_shape_points_into(
        [
            Vec2::new(-1.6, 1.0),
            Vec2::new(-0.8, 1.0),
            Vec2::new(-0.8, 1.8),
            Vec2::new(-1.6, 1.8),
        ],
        0.02,
        [3.6_f32, 0.0],
        filter,
        &mut sweep_hits,
    );

    let mut offset_hits = Vec::with_capacity(8);
    world.cast_shape_points_with_offset_into(
        [
            Vec2::new(-0.4, -0.3),
            Vec2::new(0.4, -0.3),
            Vec2::new(0.4, 0.3),
            Vec2::new(-0.4, 0.3),
        ],
        0.02,
        [-1.2_f32, 3.0],
        0.35_f32,
        [3.5_f32, -1.6],
        filter,
        &mut offset_hits,
    );

    let sweep_min_fraction = sweep_hits.iter().map(|h| h.fraction).fold(1.0, f32::min);
    let offset_min_fraction = offset_hits.iter().map(|h| h.fraction).fold(1.0, f32::min);

    println!(
        "cast_ray_closest: hit={} fraction={:.3}",
        closest.hit, closest.fraction
    );
    println!("cast_ray_all_into hits: {}", ray_hits.len());
    println!(
        "cast_shape_points_into hits: {} earliest_fraction={:.3}",
        sweep_hits.len(),
        sweep_min_fraction
    );
    println!(
        "cast_shape_points_with_offset_into hits: {} earliest_fraction={:.3}",
        offset_hits.len(),
        offset_min_fraction
    );

    Ok(())
}
