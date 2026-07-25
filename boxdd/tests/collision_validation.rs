use boxdd::{
    ApiError, DistanceInput, Polygon, Rot, ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep,
    ToiInput, Transform, collide_segment_and_polygon, shapes, try_collide_capsules,
    try_collide_circles, try_collide_segment_and_polygon, try_segment_distance, try_shape_cast,
    try_shape_distance, try_time_of_impact,
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

    let invalid_distance = DistanceInput::new(proxy, proxy, invalid_transform);
    assert_eq!(
        invalid_distance.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        try_shape_distance(invalid_distance, &mut cache).unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_cast = ShapeCastPairInput::new(proxy, proxy, Transform::IDENTITY, [1.0, 0.0])
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
        try_collide_segment_and_polygon(invalid_segment, polygon, Transform::IDENTITY,)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_capsule = shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25);
    assert_eq!(
        try_collide_capsules(invalid_capsule, invalid_capsule, Transform::IDENTITY,).unwrap_err(),
        ApiError::InvalidArgument
    );

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    assert_eq!(
        try_collide_circles(circle, circle, invalid_transform).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn sweep_transform_rejects_invalid_sweeps_and_times() {
    let valid_sweep = Sweep::new(
        [0.0_f32, 0.0],
        [1.0_f32, 2.0],
        [3.0_f32, 4.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    );
    let start = Sweep::try_transform_at(valid_sweep, 0.0).unwrap();
    let end = Sweep::try_transform_at(valid_sweep, 1.0).unwrap();

    assert_eq!(start.position(), boxdd::Vec2::new(1.0, 2.0));
    assert_eq!(end.position(), boxdd::Vec2::new(3.0, 4.0));

    for invalid_time in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -f32::EPSILON,
        1.0 + f32::EPSILON,
    ] {
        assert_eq!(
            Sweep::try_transform_at(valid_sweep, invalid_time).unwrap_err(),
            ApiError::InvalidArgument
        );
    }

    let invalid_rotation = Rot::from_raw(boxdd_sys::ffi::b2Rot { c: 2.0, s: 0.0 });
    let non_finite_rotation = Rot::from_raw(boxdd_sys::ffi::b2Rot {
        c: f32::NAN,
        s: 0.0,
    });
    for invalid_sweep in [
        Sweep::new(
            [f32::NAN, 0.0],
            [1.0_f32, 2.0],
            [3.0_f32, 4.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        ),
        Sweep::new(
            [0.0_f32, 0.0],
            [f32::INFINITY, 2.0],
            [3.0_f32, 4.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        ),
        Sweep::new(
            [0.0_f32, 0.0],
            [1.0_f32, 2.0],
            [3.0_f32, f32::NEG_INFINITY],
            Rot::IDENTITY,
            Rot::IDENTITY,
        ),
        Sweep::new(
            [0.0_f32, 0.0],
            [1.0_f32, 2.0],
            [3.0_f32, 4.0],
            invalid_rotation,
            Rot::IDENTITY,
        ),
        Sweep::new(
            [0.0_f32, 0.0],
            [1.0_f32, 2.0],
            [3.0_f32, 4.0],
            Rot::IDENTITY,
            non_finite_rotation,
        ),
    ] {
        assert_eq!(
            Sweep::try_transform_at(invalid_sweep, 0.5).unwrap_err(),
            ApiError::InvalidArgument
        );
    }

    assert!(std::panic::catch_unwind(|| valid_sweep.transform_at(f32::NAN)).is_err());
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
        // SAFETY: this intentionally violates the radius invariant to test validation.
        unsafe { Polygon::from_raw(raw_polygon) }
            .validate()
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert!(Polygon::from_points([[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]], 0.0).is_none());
}

#[test]
fn safe_manifold_collision_helpers_panic_on_invalid_geometry() {
    let result = std::panic::catch_unwind(|| {
        collide_segment_and_polygon(
            shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]),
            shapes::box_polygon(1.0, 1.0),
            Transform::IDENTITY,
        );
    });
    assert!(result.is_err());
}
