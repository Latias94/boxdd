use boxdd::prelude::*;

fn approx(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn approx_world(a: WorldScalar, b: WorldScalar, tolerance: WorldScalar) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn hello_world_box_settles_on_the_ground() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ground = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .position([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(ground)
        .unwrap()
        .create_polygon(
            &ShapeDef::default(),
            &shapes::box_polygon(50.0, 10.0).unwrap(),
        )
        .unwrap();

    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 4.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_polygon(
            &ShapeDef::builder()
                .density(1.0)
                .material(SurfaceMaterial::default().with_friction(0.3).unwrap())
                .build()
                .unwrap(),
            &shapes::box_polygon(1.0, 1.0).unwrap(),
        )
        .unwrap();

    for _ in 0..90 {
        drop(world.step(1.0 / 60.0, 4).unwrap());
    }

    let transform = world.body(body).unwrap().transform().unwrap();
    assert!(approx_world(transform.position().x, 0.0, 0.01));
    assert!(approx_world(transform.position().y, 1.0, 0.05));
    assert!(approx(transform.rotation().angle(), 0.0, 0.05));
}
