use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground body with a chain walkway (sine wave)
    let ground = world.create_body_id(BodyBuilder::new().build());
    let mut pts: Vec<Vec2> = Vec::new();
    for i in -20..=20 {
        let x = i as f32 * 0.5;
        let y = (x * 0.6).sin() * 0.4;
        pts.push(Vec2::new(x, y));
    }
    let cdef = boxdd::shapes::chain::ChainDef::builder()
        .points(pts.iter().copied())
        .is_loop(false)
        .single_material(&SurfaceMaterial::default())
        .build();
    let _chain = world.create_chain_for_id(ground, &cdef);

    // Spawn some dynamic boxes that will roll along the walkway
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.2, 0.2);
    let mut ids = Vec::new();
    for i in 0..10 {
        let x = -4.0 + i as f32 * 0.8;
        let b = world.create_body_id(BodyBuilder::new().position([x, 3.0_f32]).build());
        let _s = world.create_polygon_shape_for(b, &sdef, &poly);
        ids.push(b);
    }

    for _ in 0..300 {
        world.step(1.0 / 60.0, 4);
    }

    let avg_y = if ids.is_empty() {
        0.0
    } else {
        ids.iter().map(|&id| world.body_position(id).y).sum::<f32>() / ids.len() as f32
    };
    println!("chain walkway: {} bodies, avg y={:.2}", ids.len(), avg_y);
    Ok(())
}
