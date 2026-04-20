use boxdd::{
    CastOutput, DistanceInput, DistanceOutput, MAX_SHAPE_PROXY_POINTS, Rot, SegmentDistanceResult,
    ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput, ToiOutput, ToiState, Transform,
    segment_distance, shape_cast, shape_distance, time_of_impact,
};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn shape_proxy_rejects_invalid_point_counts() {
    assert!(ShapeProxy::new(core::iter::empty::<[f32; 2]>(), 0.0).is_none());

    let too_many = (0..=MAX_SHAPE_PROXY_POINTS).map(|i| [i as f32, 0.0]);
    assert!(ShapeProxy::new(too_many, 0.0).is_none());
}

#[test]
fn segment_and_shape_distance_and_toi() {
    let res = segment_distance([-1.0, -1.0], [-1.0, 1.0], [2.0, 0.0], [1.0, 0.0]);
    assert!(approx(res.fraction1, 0.5, f32::EPSILON));
    assert!(approx(res.fraction2, 1.0, f32::EPSILON));
    assert!(approx(res.closest1.x, -1.0, f32::EPSILON));
    assert!(approx(res.closest1.y, 0.0, f32::EPSILON));
    assert!(approx(res.closest2.x, 1.0, f32::EPSILON));
    assert!(approx(res.closest2.y, 0.0, f32::EPSILON));
    assert!(approx(res.distance_squared, 4.0, f32::EPSILON));

    let proxy_a =
        ShapeProxy::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]], 0.0).unwrap();
    let proxy_b = ShapeProxy::new([[2.0, -1.0], [2.0, 1.0]], 0.0).unwrap();

    let mut cache = SimplexCache::default();
    let out = shape_distance(
        DistanceInput::new(proxy_a, proxy_b, Transform::IDENTITY, Transform::IDENTITY),
        &mut cache,
    );
    assert!(approx(out.distance, 1.0, f32::EPSILON));
    assert!(cache.count() > 0);

    let outc = shape_cast(ShapeCastPairInput::new(
        proxy_a,
        proxy_b,
        Transform::IDENTITY,
        Transform::IDENTITY,
        [-2.0, 0.0],
    ));
    assert!(outc.hit);
    assert!(approx(outc.fraction, 0.5, 0.005));

    let toi = time_of_impact(ToiInput::new(
        proxy_a,
        proxy_b,
        Sweep::new(
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        ),
        Sweep::new(
            [0.0, 0.0],
            [0.0, 0.0],
            [-2.0, 0.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        ),
    ));
    assert_eq!(toi.state, ToiState::Hit);
    assert!(approx(toi.fraction, 0.5, 0.005));
}

#[test]
fn collision_result_types_use_explicit_raw_conversions() {
    let segment = SegmentDistanceResult::from_raw(boxdd_sys::ffi::b2SegmentDistanceResult {
        closest1: boxdd_sys::ffi::b2Vec2 { x: -1.0, y: 2.0 },
        closest2: boxdd_sys::ffi::b2Vec2 { x: 3.0, y: -4.0 },
        fraction1: 0.25,
        fraction2: 0.75,
        distanceSquared: 6.5,
    });
    assert!(approx(segment.closest1.x, -1.0, f32::EPSILON));
    assert!(approx(segment.closest1.y, 2.0, f32::EPSILON));
    assert!(approx(segment.closest2.x, 3.0, f32::EPSILON));
    assert!(approx(segment.closest2.y, -4.0, f32::EPSILON));
    assert!(approx(segment.fraction1, 0.25, f32::EPSILON));
    assert!(approx(segment.fraction2, 0.75, f32::EPSILON));
    assert!(approx(segment.distance_squared, 6.5, f32::EPSILON));

    let cast = CastOutput::from_raw(boxdd_sys::ffi::b2CastOutput {
        normal: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 1.0 },
        point: boxdd_sys::ffi::b2Vec2 { x: 4.0, y: -2.0 },
        fraction: 0.4,
        iterations: 7,
        hit: true,
    });
    assert!(approx(cast.normal.y, 1.0, f32::EPSILON));
    assert!(approx(cast.point.x, 4.0, f32::EPSILON));
    assert!(approx(cast.point.y, -2.0, f32::EPSILON));
    assert!(approx(cast.fraction, 0.4, f32::EPSILON));
    assert_eq!(cast.iterations, 7);
    assert!(cast.hit);

    let distance = DistanceOutput::from_raw(boxdd_sys::ffi::b2DistanceOutput {
        pointA: boxdd_sys::ffi::b2Vec2 { x: -2.0, y: 1.0 },
        pointB: boxdd_sys::ffi::b2Vec2 { x: 3.0, y: 5.0 },
        normal: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        distance: 5.25,
        iterations: 3,
        simplexCount: 2,
    });
    assert!(approx(distance.point_a.x, -2.0, f32::EPSILON));
    assert!(approx(distance.point_b.y, 5.0, f32::EPSILON));
    assert!(approx(distance.normal.x, 1.0, f32::EPSILON));
    assert!(approx(distance.distance, 5.25, f32::EPSILON));
    assert_eq!(distance.iterations, 3);
    assert_eq!(distance.simplex_count, 2);

    assert_eq!(
        ToiState::from_raw(boxdd_sys::ffi::b2TOIState_b2_toiStateSeparated),
        ToiState::Separated
    );
    assert_eq!(
        ToiState::from_raw(boxdd_sys::ffi::b2TOIState_b2_toiStateHit),
        ToiState::Hit
    );

    let toi = ToiOutput::from_raw(boxdd_sys::ffi::b2TOIOutput {
        state: boxdd_sys::ffi::b2TOIState_b2_toiStateOverlapped,
        point: boxdd_sys::ffi::b2Vec2 { x: 0.5, y: -0.5 },
        normal: boxdd_sys::ffi::b2Vec2 { x: -1.0, y: 0.0 },
        fraction: 0.6,
    });
    assert_eq!(toi.state, ToiState::Overlapped);
    assert!(approx(toi.point.x, 0.5, f32::EPSILON));
    assert!(approx(toi.point.y, -0.5, f32::EPSILON));
    assert!(approx(toi.normal.x, -1.0, f32::EPSILON));
    assert!(approx(toi.fraction, 0.6, f32::EPSILON));
}
