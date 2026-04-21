use boxdd::{
    ApiError, DistanceInput, Polygon, Rot, ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep,
    ToiInput, Transform, collide_segment_and_polygon, shapes, try_collide_capsules,
    try_collide_segment_and_polygon, try_segment_distance, try_shape_cast, try_shape_distance,
    try_time_of_impact,
};

#[test]
fn shape_proxy_rejects_invalid_geometry_inputs() {
    assert_eq!(
        ShapeProxy::try_new(core::iter::empty::<[f32; 2]>(), 0.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        ShapeProxy::try_new([[f32::NAN, 0.0]], 0.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        ShapeProxy::try_new([[0.0_f32, 0.0]], -1.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert!(ShapeProxy::new([[f32::NAN, 0.0]], 0.0).is_none());
    assert!(ShapeProxy::new([[0.0_f32, 0.0]], -1.0).is_none());
}

#[test]
fn standalone_collision_try_apis_reject_invalid_inputs() {
    let proxy = ShapeProxy::new([[0.0_f32, 0.0]], 0.0).unwrap();
    let invalid_transform = Transform::from_pos_angle([f32::NAN, 0.0], 0.0);
    let mut cache = SimplexCache::default();

    let invalid_distance = DistanceInput::new(proxy, proxy, invalid_transform, Transform::IDENTITY);
    assert_eq!(
        invalid_distance.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        try_shape_distance(invalid_distance, &mut cache).unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_cast = ShapeCastPairInput::new(
        proxy,
        proxy,
        Transform::IDENTITY,
        Transform::IDENTITY,
        [1.0, 0.0],
    )
    .with_max_fraction(1.5);
    assert_eq!(
        invalid_cast.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        try_shape_cast(invalid_cast).unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_sweep = Sweep::new(
        [0.0_f32, 0.0],
        [0.0, 0.0],
        [1.0, 0.0],
        Rot::from_raw(boxdd_sys::ffi::b2Rot { c: 2.0, s: 0.0 }),
        Rot::IDENTITY,
    );
    assert_eq!(
        invalid_sweep.validate().unwrap_err(),
        ApiError::InvalidArgument
    );

    let valid_sweep = Sweep::new(
        [0.0_f32, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    );
    let invalid_toi =
        ToiInput::new(proxy, proxy, invalid_sweep, valid_sweep).with_max_fraction(1.0);
    assert_eq!(
        invalid_toi.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        try_time_of_impact(invalid_toi).unwrap_err(),
        ApiError::InvalidArgument
    );

    assert_eq!(
        try_segment_distance(
            [f32::NAN, 0.0],
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [1.0_f32, 0.0]
        )
        .unwrap_err(),
        ApiError::InvalidArgument
    );

    let polygon = shapes::box_polygon(1.0, 1.0);
    let invalid_segment = shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]);
    assert_eq!(
        try_collide_segment_and_polygon(
            invalid_segment,
            Transform::IDENTITY,
            polygon,
            Transform::IDENTITY,
        )
        .unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_capsule = shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25);
    assert_eq!(
        try_collide_capsules(
            invalid_capsule,
            Transform::IDENTITY,
            invalid_capsule,
            Transform::IDENTITY,
        )
        .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn geometry_values_expose_validation_for_invalid_inputs() {
    assert_eq!(
        shapes::circle([f32::NAN, 0.0], 0.5).validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0])
            .validate()
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25)
            .validate()
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shapes::chain_segment(
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [1.0_f32, 0.0]
        )
        .validate()
        .unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut raw_polygon = shapes::box_polygon(1.0, 1.0).into_raw();
    raw_polygon.radius = -1.0;
    assert_eq!(
        Polygon::from_raw(raw_polygon).validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert!(Polygon::from_points([[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]], 0.0).is_none());
}

#[test]
fn safe_manifold_collision_helpers_panic_on_invalid_geometry() {
    let result = std::panic::catch_unwind(|| {
        collide_segment_and_polygon(
            shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]),
            Transform::IDENTITY,
            shapes::box_polygon(1.0, 1.0),
            Transform::IDENTITY,
        );
    });
    assert!(result.is_err());
}
