use boxdd::prelude::*;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn hello_world_final_pose() {
    // Replicate upstream HelloWorld: final y ~ 1.0, small x and angle
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().position([0.0_f32, -10.0]).build());
    let _gs = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 10.0),
    );

    // Dynamic box at y=4
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 4.0])
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .material(SurfaceMaterial::default().friction(0.3))
        .build();
    let _bx = world.create_polygon_shape_for(body, &sdef, &shapes::box_polygon(1.0, 1.0));

    for _ in 0..90 {
        world.step(1.0 / 60.0, 4);
    }

    let pos = world.body_position(body);
    let angle = world.body_transform(body).rotation().angle();
    assert!(approx(pos.x, 0.0, 0.01));
    assert!(approx(pos.y, 1.00, 0.05));
    assert!(approx(angle, 0.0, 0.05));
}
