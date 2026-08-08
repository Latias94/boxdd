use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    // World
    let def = boxdd::WorldBuilder::from(foundation.world_def())
        .gravity(Vec2::new(0.0, -10.0))
        .build()?;
    let mut world = foundation.create_world(def)?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let sdef_ground = ShapeDef::builder().density(0.0).build()?;
    let gpoly = shapes::box_polygon(50.0, 1.0).expect("valid polygon geometry");
    let _gs = world.body(ground)?.create_polygon(&sdef_ground, &gpoly)?;

    // Pyramid of boxes
    let columns = 10usize;
    let rows = 10usize;
    let box_poly = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
    let sdef = ShapeDef::builder().density(1.0).build()?;
    let mut bodies: Vec<BodyId> = Vec::new();
    for i in 0..rows {
        // Avoid usize underflow when rows > columns
        let width = columns.saturating_sub(i);
        for j in 0..width {
            let x = (j as f32) * 1.1 - (width as f32) * 0.55;
            let y = 0.5 + (i as f32) * 1.05 + 2.0;
            let b = world.create_body(
                BodyBuilder::from(foundation.body_def())
                    .position([x, y])
                    .build()?,
            )?;
            let _s = world.body(b)?.create_polygon(&sdef, &box_poly)?;
            bodies.push(b);
        }
    }

    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    if let Some(&top) = bodies.last() {
        let p = world.body(top)?.position()?;
        println!("top box at: ({:.2}, {:.2})", p.x, p.y);
    }
    println!("pyramid: {} bodies", bodies.len());
    Ok(())
}
