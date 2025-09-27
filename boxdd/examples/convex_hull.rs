use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Random-ish cloud of points (star shape)
    let pts = [
        Vec2::new(-1.0, 0.0),
        Vec2::new(-0.5, 0.8),
        Vec2::new(0.0, 0.2),
        Vec2::new(0.5, 0.9),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.6, -0.6),
        Vec2::new(0.0, -0.2),
        Vec2::new(-0.7, -0.5),
    ];
    let poly = boxdd::shapes::polygon_from_points(pts, 0.02).expect("valid hull");

    let body = world.create_body_id(BodyBuilder::new().position([0.0_f32, 3.0]).build());
    let _s = world.create_polygon_shape_for(body, &ShapeDef::builder().density(1.0).build(), &poly);

    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }

    // Query hull vertex count via shape getter
    // Note: polygon().count is in b2Polygon; we can fetch polygon from the shape via id -> not exposed directly.
    // So we just print that we built a hull from N points.
    println!("convex hull from {} points built and simulated", pts.len());
    Ok(())
}
