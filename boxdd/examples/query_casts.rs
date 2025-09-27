use boxdd::{shapes, BodyBuilder, QueryFilter, ShapeDef, Vec2, World, WorldDef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build())?;

    // One dynamic box
    let body = world.create_body_id(BodyBuilder::new().position(Vec2::new(0.0, 2.0)).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let _s = world.create_polygon_shape_for(body, &sdef, &shapes::box_polygon(0.5, 0.5));

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    // Ray casts
    let _closest = world.cast_ray_closest([0.0_f32, 5.0], [0.0, -10.0], QueryFilter::default());
    let _all = world.cast_ray_all([0.0_f32, 5.0], [0.0, -10.0], QueryFilter::default());

    // Polygon overlap via points (+radius)
    let square = [
        Vec2::new(-0.6, -0.6),
        Vec2::new(0.6, -0.6),
        Vec2::new(0.6, 0.6),
        Vec2::new(-0.6, 0.6),
    ];
    let _over_ids = world.overlap_polygon_points(square, 0.01, QueryFilter::default());

    // Offset proxy and cast
    let _over_off = world.overlap_polygon_points_with_offset(
        square,
        0.01,
        [0.0_f32, 2.0],
        0.0_f32,
        QueryFilter::default(),
    );
    let _cast_off = world.cast_shape_points_with_offset(
        square,
        0.01,
        [0.0_f32, 2.0],
        0.0_f32,
        [1.0_f32, 0.0],
        QueryFilter::default(),
    );

    // Cast mover (capsule)
    let _frac = world.cast_mover(
        [0.0_f32, 2.0],
        [0.0, 2.5],
        0.1,
        [0.5_f32, 0.0],
        QueryFilter::default(),
    );

    Ok(())
}
