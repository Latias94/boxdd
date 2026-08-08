use boxdd::{prelude::*, shapes};

const INVALID_WIND_PARAMETERS: [(Vec2, f32, f32, &str, &str); 10] = [
    (
        Vec2::new(f32::NAN, 0.0),
        1.0,
        0.5,
        "wind",
        "a finite vector",
    ),
    (
        Vec2::new(0.0, f32::INFINITY),
        1.0,
        0.5,
        "wind",
        "a finite vector",
    ),
    (
        Vec2::new(f32::NEG_INFINITY, 0.0),
        1.0,
        0.5,
        "wind",
        "a finite vector",
    ),
    (
        Vec2::new(5.0, 0.0),
        f32::NAN,
        0.5,
        "drag",
        "a finite value greater than or equal to zero",
    ),
    (
        Vec2::new(5.0, 0.0),
        f32::INFINITY,
        0.5,
        "drag",
        "a finite value greater than or equal to zero",
    ),
    (
        Vec2::new(5.0, 0.0),
        f32::NEG_INFINITY,
        0.5,
        "drag",
        "a finite value greater than or equal to zero",
    ),
    (
        Vec2::new(5.0, 0.0),
        -1.0,
        0.5,
        "drag",
        "a finite value greater than or equal to zero",
    ),
    (Vec2::new(5.0, 0.0), 1.0, f32::NAN, "lift", "a finite value"),
    (
        Vec2::new(5.0, 0.0),
        1.0,
        f32::INFINITY,
        "lift",
        "a finite value",
    ),
    (
        Vec2::new(5.0, 0.0),
        1.0,
        f32::NEG_INFINITY,
        "lift",
        "a finite value",
    ),
];

fn create_wind_test_shape(world: &mut World) -> (BodyId, ShapeId) {
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape = world
        .body(body)
        .unwrap()
        .create_polygon(
            &ShapeDef::builder().density(1.0).build().unwrap(),
            &shapes::box_polygon(0.5, 0.5).unwrap(),
        )
        .unwrap();
    (body, shape)
}

fn assert_invalid_wind_parameters(shape: &mut Shape<'_>) {
    for (wind, drag, lift, argument, constraint) in INVALID_WIND_PARAMETERS {
        assert_eq!(
            shape.apply_wind(wind, drag, lift, true),
            Err(Error::invalid_argument(
                "Shape::apply_wind",
                argument,
                constraint,
            ))
        );
    }
}

#[test]
fn shape_wind_validation_is_transactional_and_allows_signed_lift() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let (body, shape_id) = create_wind_test_shape(&mut world);

    let mut shape = world.shape(shape_id).unwrap();
    assert_invalid_wind_parameters(&mut shape);
    shape
        .apply_wind(Vec2::new(5.0, 0.0), 1.0, -0.5, true)
        .unwrap();
    drop(world.step(1.0 / 60.0, 4).unwrap());
    let velocity = world.body(body).unwrap().linear_velocity().unwrap();
    assert!(velocity.is_valid());
    assert!(velocity.x > 0.0);
}

#[test]
fn recording_uses_the_same_shape_wind_contract() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let (body, shape_id) = create_wind_test_shape(&mut world);

    let _recording = {
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        let mut shape = session.shape(shape_id).unwrap();
        assert_invalid_wind_parameters(&mut shape);
        shape
            .apply_wind(Vec2::new(5.0, 0.0), 1.0, -0.5, true)
            .unwrap();
        drop(session.step(1.0 / 60.0, 4).unwrap());
        session.finish().unwrap()
    };

    let velocity = world.body(body).unwrap().linear_velocity().unwrap();
    assert!(velocity.is_valid());
    assert!(velocity.x > 0.0);
}
