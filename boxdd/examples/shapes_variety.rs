use boxdd::prelude::*;

// Rough port of parts of "Shapes" sample: create a few primitive shapes with materials.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ground_shape = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;

    // Materials
    let ice = SurfaceMaterial::default()
        .with_friction(0.05)?
        .with_restitution(0.0)?;
    let rubber = SurfaceMaterial::default()
        .with_friction(0.8)?
        .with_restitution(0.7)?;
    let metal = SurfaceMaterial::default()
        .with_friction(0.4)?
        .with_restitution(0.1)?;

    // A few dynamic bodies each with a different shape
    let s_circle = ShapeDef::builder().density(1.0).material(rubber).build()?;
    let s_poly = ShapeDef::builder().density(1.0).material(metal).build()?;
    let s_caps = ShapeDef::builder().density(1.0).material(ice).build()?;

    let b_circle = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-4.0_f32, 6.0])
            .build()?,
    )?;
    let b_poly = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 6.0])
            .build()?,
    )?;
    let b_caps = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([4.0_f32, 6.0])
            .build()?,
    )?;

    let _ = world
        .body(b_circle)?
        .create_circle(&s_circle, &shapes::circle([0.0, 0.0], 0.5)?)?;
    let _ = world.body(b_poly)?.create_polygon(
        &s_poly,
        &shapes::box_polygon(0.6, 0.4).expect("valid polygon geometry"),
    )?;
    let _ = world
        .body(b_caps)?
        .create_capsule(&s_caps, &shapes::capsule([-0.6_f32, 0.0], [0.6, 0.0], 0.2)?)?;

    // Step and print positions
    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    let pc = world.body(b_circle)?.position()?;
    let pp = world.body(b_poly)?.position()?;
    let pa = world.body(b_caps)?.position()?;
    println!(
        "shapes_variety: circle=({:.2},{:.2}) box=({:.2},{:.2}) capsule=({:.2},{:.2})",
        pc.x, pc.y, pp.x, pp.y, pa.x, pa.y
    );
    Ok(())
}
