use boxdd::{
    DistanceInput, MAX_SHAPE_PROXY_POINTS, Rot, ShapeCastPairInput, ShapeProxy, SimplexCache,
    Sweep, ToiInput, ToiState, Transform, segment_distance, shape_cast, shape_distance,
    time_of_impact,
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
