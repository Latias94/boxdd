use boxdd::{
    ApiError, DistanceInput, Rot, ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput,
    Transform, try_shape_cast, try_shape_distance, try_time_of_impact,
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
}
