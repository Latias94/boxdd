use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _g = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 1.0),
    );

    // Kinematic platform moving horizontally
    let platform = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Kinematic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let _ps = world.create_polygon_shape_for(
        platform,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(2.0, 0.2),
    );
    world.set_body_linear_velocity(platform, [1.5_f32, 0.0]);

    // Dynamic box on top
    let box_id = world.create_body_id(BodyBuilder::new().position([0.0_f32, 4.0]).build());
    let _bs = world.create_polygon_shape_for(
        box_id,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.3, 0.3),
    );

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }
    let p = world.body_position(box_id);
    println!("box after ride: ({:.2}, {:.2})", p.x, p.y);
    Ok(())
}
