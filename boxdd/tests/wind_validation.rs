use std::panic::{AssertUnwindSafe, catch_unwind};

use boxdd::prelude::*;
use boxdd::shapes;

const INVALID_WIND_PARAMETERS: [(Vec2, f32, f32); 10] = [
    (Vec2::new(f32::NAN, 0.0), 1.0, 0.5),
    (Vec2::new(0.0, f32::INFINITY), 1.0, 0.5),
    (Vec2::new(f32::NEG_INFINITY, 0.0), 1.0, 0.5),
    (Vec2::new(5.0, 0.0), f32::NAN, 0.5),
    (Vec2::new(5.0, 0.0), f32::INFINITY, 0.5),
    (Vec2::new(5.0, 0.0), f32::NEG_INFINITY, 0.5),
    (Vec2::new(5.0, 0.0), -1.0, 0.5),
    (Vec2::new(5.0, 0.0), 1.0, f32::NAN),
    (Vec2::new(5.0, 0.0), 1.0, f32::INFINITY),
    (Vec2::new(5.0, 0.0), 1.0, f32::NEG_INFINITY),
];

fn collect_try_results(
    mut apply: impl FnMut(Vec2, f32, f32) -> ApiResult<()>,
) -> Vec<ApiResult<()>> {
    INVALID_WIND_PARAMETERS
        .into_iter()
        .map(|(wind, drag, lift)| apply(wind, drag, lift))
        .collect()
}

fn collect_panics(mut apply: impl FnMut(Vec2, f32, f32)) -> Vec<bool> {
    INVALID_WIND_PARAMETERS
        .into_iter()
        .map(|(wind, drag, lift)| {
            catch_unwind(AssertUnwindSafe(|| apply(wind, drag, lift))).is_err()
        })
        .collect()
}

fn assert_invalid_parameters_rejected(results: &[ApiResult<()>], panics: &[bool]) {
    assert!(
        results
            .iter()
            .all(|result| *result == Err(ApiError::InvalidArgument))
    );
    assert!(panics.iter().all(|panicked| *panicked));
}

fn create_wind_test_shape(world: &mut World) -> (BodyId, ShapeId) {
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let shape = world.create_polygon_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );
    (body, shape)
}

fn assert_finite_motion_after_step(world: &mut World, body: BodyId) {
    world.step(1.0 / 60.0, 4);
    let velocity = world.body_linear_velocity(body);
    assert!(velocity.is_valid());
    assert!(velocity.x > 0.0);
}

#[test]
fn world_shape_apply_wind_rejects_invalid_parameters_before_native_mutation() {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).unwrap();
    let (body, shape) = create_wind_test_shape(&mut world);

    let results = collect_try_results(|wind, drag, lift| {
        world.try_shape_apply_wind(shape, wind, drag, lift, true)
    });
    let panics =
        collect_panics(|wind, drag, lift| world.shape_apply_wind(shape, wind, drag, lift, true));

    // Lift is signed because its sign selects the perpendicular force direction.
    world.shape_apply_wind(shape, Vec2::new(5.0, 0.0), 1.0, -0.5, true);
    assert_finite_motion_after_step(&mut world, body);
    assert_invalid_parameters_rejected(&results, &panics);
}

#[test]
fn owned_shape_apply_wind_rejects_invalid_parameters_before_native_mutation() {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let mut shape = world.create_polygon_shape_for_owned(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    let results =
        collect_try_results(|wind, drag, lift| shape.try_apply_wind(wind, drag, lift, true));
    let panics = collect_panics(|wind, drag, lift| shape.apply_wind(wind, drag, lift, true));

    shape.apply_wind(Vec2::new(5.0, 0.0), 1.0, -0.5, true);
    assert_finite_motion_after_step(&mut world, body);
    assert_invalid_parameters_rejected(&results, &panics);
}

#[test]
fn scoped_shape_apply_wind_rejects_invalid_parameters_before_native_mutation() {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).unwrap();
    let (body, shape_id) = create_wind_test_shape(&mut world);

    let (results, panics) = {
        let mut shape = world.try_shape(shape_id).unwrap();
        let results =
            collect_try_results(|wind, drag, lift| shape.try_apply_wind(wind, drag, lift, true));
        let panics = collect_panics(|wind, drag, lift| shape.apply_wind(wind, drag, lift, true));
        shape.apply_wind(Vec2::new(5.0, 0.0), 1.0, -0.5, true);
        (results, panics)
    };

    assert_finite_motion_after_step(&mut world, body);
    assert_invalid_parameters_rejected(&results, &panics);
}

#[test]
fn recording_shape_apply_wind_rejects_invalid_parameters_before_native_mutation() {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).unwrap();
    let (body, shape) = create_wind_test_shape(&mut world);

    let (results, panics, recording) = {
        let mut session = world.start_recording(RecordingCapacity::default());
        let results = collect_try_results(|wind, drag, lift| {
            session.try_shape_apply_wind(shape, wind, drag, lift, true)
        });
        let panics = collect_panics(|wind, drag, lift| {
            session.shape_apply_wind(shape, wind, drag, lift, true)
        });
        session.shape_apply_wind(shape, Vec2::new(5.0, 0.0), 1.0, -0.5, true);
        session.step(1.0 / 60.0, 4);
        (results, panics, session.finish())
    };

    let velocity = world.body_linear_velocity(body);
    assert!(velocity.is_valid());
    assert!(velocity.x > 0.0);
    assert!(!recording.is_empty());
    assert_invalid_parameters_rejected(&results, &panics);
}
