use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground body with a chain walkway (sine wave)
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let mut pts: Vec<Vec2> = Vec::with_capacity(41);
    for i in -20..=20 {
        let x = i as f32 * 0.5;
        let y = (x * 0.6).sin() * 0.4;
        pts.push(Vec2::new(x, y));
    }
    let cdef = boxdd::shapes::chain::ChainDef::builder()
        .points(pts.iter().copied())
        .is_loop(false)
        .single_material(&SurfaceMaterial::default())
        .build()?;
    let _chain = world.body(ground)?.create_chain(&cdef)?;

    // Spawn some dynamic boxes that will roll along the walkway
    let sdef = ShapeDef::builder().density(1.0).build()?;
    let poly = shapes::box_polygon(0.2, 0.2).expect("valid polygon geometry");
    let mut ids = Vec::with_capacity(10);
    for i in 0..10 {
        let x = -4.0 + i as f32 * 0.8;
        let b = world.create_body(
            BodyBuilder::from(foundation.body_def())
                .position([x, 3.0_f32])
                .build()?,
        )?;
        let _s = world.body(b)?.create_polygon(&sdef, &poly)?;
        ids.push(b);
    }

    for _ in 0..300 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    let avg_y = if ids.is_empty() {
        0.0
    } else {
        let mut total = 0.0;
        for &id in &ids {
            total += world.body(id)?.position()?.y;
        }
        total / ids.len() as WorldScalar
    };
    println!("chain walkway: {} bodies, avg y={:.2}", ids.len(), avg_y);
    Ok(())
}
