use boxdd::prelude::*;

// Rough port of parts of "Shapes" sample: create a few primitive shapes with materials.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );

    // Materials
    let ice = SurfaceMaterial::default().friction(0.05).restitution(0.0);
    let rubber = SurfaceMaterial::default().friction(0.8).restitution(0.7);
    let metal = SurfaceMaterial::default().friction(0.4).restitution(0.1);

    // A few dynamic bodies each with a different shape
    let s_circle = ShapeDef::builder()
        .density(1.0)
        .material(rubber.clone())
        .build();
    let s_poly = ShapeDef::builder()
        .density(1.0)
        .material(metal.clone())
        .build();
    let s_caps = ShapeDef::builder()
        .density(1.0)
        .material(ice.clone())
        .build();

    let b_circle = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-4.0_f32, 6.0])
            .build(),
    );
    let b_poly = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 6.0])
            .build(),
    );
    let b_caps = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([4.0_f32, 6.0])
            .build(),
    );

    let _ = world.create_circle_shape_for(b_circle, &s_circle, &shapes::circle([0.0, 0.0], 0.5));
    let _ = world.create_polygon_shape_for(b_poly, &s_poly, &shapes::box_polygon(0.6, 0.4));
    let _ = world.create_capsule_shape_for(
        b_caps,
        &s_caps,
        &shapes::capsule([-0.6_f32, 0.0], [0.6, 0.0], 0.2),
    );

    // Step and print positions
    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }
    let pc = world.body_position(b_circle);
    let pp = world.body_position(b_poly);
    let pa = world.body_position(b_caps);
    println!(
        "shapes_variety: circle=({:.2},{:.2}) box=({:.2},{:.2}) capsule=({:.2},{:.2})",
        pc.x, pc.y, pp.x, pp.y, pa.x, pa.y
    );
    Ok(())
}
