use boxdd::{shapes, Aabb, BodyBuilder, QueryFilter, ShapeDef, Vec2, World, WorldDef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def)?;

    // Create a static ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let sdef = ShapeDef::builder().density(0.0).build();
    let gpoly = shapes::box_polygon(10.0, 0.5);
    let _gs = world.create_polygon_shape_for(ground, &sdef, &gpoly);

    // Create a dynamic box
    let body = world.create_body_id(BodyBuilder::new().position(Vec2::new(0.0, 2.0)).build());
    let sdef_dyn = ShapeDef::builder().density(1.0).build();
    let bpoly = shapes::box_polygon(0.5, 0.5);
    let _bs = world.create_polygon_shape_for(body, &sdef_dyn, &bpoly);

    // Step world a bit
    for _ in 0..5 {
        world.step(1.0 / 60.0, 4);
    }

    // AABB overlap around origin
    let ids = world.overlap_aabb(
        Aabb {
            lower: Vec2::new(-1.0, -1.0),
            upper: Vec2::new(1.0, 1.0),
        },
        QueryFilter::default(),
    );
    println!("overlap ids: {}", ids.len());

    // Ray cast down from y=10
    let hits = world.cast_ray_all([0.0_f32, 10.0], [0.0, -100.0], QueryFilter::default());
    println!("ray hits: {}", hits.len());

    Ok(())
}
