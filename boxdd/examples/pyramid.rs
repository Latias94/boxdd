use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // World
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -10.0)).build();
    let mut world = World::new(def)?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let sdef_ground = ShapeDef::builder().density(0.0).build();
    let gpoly = shapes::box_polygon(50.0, 1.0);
    let _gs = world.create_polygon_shape_for(ground, &sdef_ground, &gpoly);

    // Pyramid of boxes
    let columns = 10usize;
    let rows = 10usize;
    let box_poly = shapes::box_polygon(0.5, 0.5);
    let sdef = ShapeDef::builder().density(1.0).build();
    let mut bodies: Vec<BodyId> = Vec::new();
    for i in 0..rows {
        // Avoid usize underflow when rows > columns
        let width = columns.saturating_sub(i);
        for j in 0..width {
            let x = (j as f32) * 1.1 - (width as f32) * 0.55;
            let y = 0.5 + (i as f32) * 1.05 + 2.0;
            let b = world.create_body_id(BodyBuilder::new().position([x, y]).build());
            let _s = world.create_polygon_shape_for(b, &sdef, &box_poly);
            bodies.push(b);
        }
    }

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }

    if let Some(&top) = bodies.last() {
        let p = world.body_position(top);
        println!("top box at: ({:.2}, {:.2})", p.x, p.y);
    }
    println!("pyramid: {} bodies", bodies.len());
    Ok(())
}
