use boxdd::{BodyType, DistanceJointDef, Error, Foundation, FoundationConfig, RecordingLimits};

const LENGTH_UNITS_PER_METER: f32 = 2.0;

#[test]
fn scale_and_owner_provenance_are_issued_by_capabilities() {
    let foundation = Foundation::initialize(FoundationConfig::new(LENGTH_UNITS_PER_METER))
        .expect("custom-scale foundation should initialize");

    let mut world = foundation
        .create_world(
            foundation
                .world_builder()
                .gravity([0.0_f32, -9.8])
                .build()
                .unwrap(),
        )
        .expect("a foundation-issued world definition should be accepted");

    world
        .start_recording(RecordingLimits::DEFAULT)
        .unwrap()
        .finish()
        .expect("an empty custom-scale recording should pass output validation");

    let body_def = world
        .body_builder()
        .body_type(BodyType::Dynamic)
        .build()
        .unwrap();
    let body_a = world.create_body(body_def.clone()).unwrap();
    let body_b = world.create_body(body_def).unwrap();

    let draw_scale = 17.0;
    let matching = DistanceJointDef::new(
        world
            .joint_base(body_a, body_b)
            .unwrap()
            .with_draw_scale(draw_scale)
            .unwrap(),
    );
    let expected_maximum_length = if cfg!(feature = "double-precision") {
        1.0e9 * LENGTH_UNITS_PER_METER
    } else {
        1.0e5 * LENGTH_UNITS_PER_METER
    };
    let draw_scaled_maximum_length = if cfg!(feature = "double-precision") {
        1.0e9 * draw_scale
    } else {
        1.0e5 * draw_scale
    };
    assert_eq!(matching.maximum_length(), expected_maximum_length);
    assert_ne!(matching.maximum_length(), draw_scaled_maximum_length);
    world.create_distance_joint(&matching).unwrap();

    let mut foreign_world = foundation
        .create_world(foundation.world_def())
        .expect("another world from the same foundation should be accepted");
    let foreign_body = foreign_world.create_body(foreign_world.body_def()).unwrap();
    assert!(matches!(
        world.joint_base(body_a, foreign_body),
        Err(Error::WrongWorld)
    ));

    let mut session = world.start_recording(RecordingLimits::DEFAULT).unwrap();
    assert!(matches!(
        session.joint_base(body_a, foreign_body),
        Err(Error::WrongWorld)
    ));
    let recorded_body = session
        .body_builder()
        .body_type(BodyType::Dynamic)
        .build()
        .unwrap();
    let recorded_body_a = session.create_body(recorded_body.clone()).unwrap();
    let recorded_body_b = session.create_body(recorded_body).unwrap();
    let recorded_joint = DistanceJointDef::new(
        session
            .joint_base(recorded_body_a, recorded_body_b)
            .unwrap(),
    );
    session.create_distance_joint(&recorded_joint).unwrap();
    session.finish().unwrap();
}
