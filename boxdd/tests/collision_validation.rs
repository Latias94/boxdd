use boxdd::{
    Error, Polygon, Rot, ShapeCastPairInput, ShapeProxy, Sweep, ToiInput, Transform,
    segment_distance, shapes,
};

fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn shape_proxy_rejects_invalid_geometry_inputs() {
    assert_eq!(
        ShapeProxy::new(core::iter::empty::<[f32; 2]>(), 0.0).unwrap_err(),
        Error::invalid_argument("ShapeProxy::new", "points", "at least one point")
    );
    assert_eq!(
        ShapeProxy::new([[f32::NAN, 0.0]], 0.0).unwrap_err(),
        Error::invalid_argument("ShapeProxy::new", "points", "a finite vector")
    );
    assert_eq!(
        ShapeProxy::new([[0.0_f32, 0.0]], -1.0).unwrap_err(),
        Error::invalid_argument(
            "ShapeProxy::new",
            "radius",
            "a finite value greater than or equal to zero",
        )
    );

    let overflow_transform = Transform::from_pos_angle([f32::MAX, 0.0], 0.0).unwrap();
    assert_eq!(
        ShapeProxy::offset_from_points([[f32::MAX, 0.0]], 0.0, overflow_transform).unwrap_err(),
        Error::invalid_argument(
            "ShapeProxy::offset_from_points",
            "points/transform",
            "a transform whose proxy points remain finite",
        )
    );
}

#[test]
fn offset_shape_proxy_applies_the_transform_without_exposing_raw_storage() {
    let transform =
        Transform::from_pos_angle([2.0_f32, -1.0], core::f32::consts::FRAC_PI_2).unwrap();
    let proxy =
        ShapeProxy::offset_from_points([[1.0_f32, 0.0], [0.0, 2.0]], 0.25, transform).unwrap();

    assert_eq!(proxy.count(), 2);
    assert_eq!(proxy.radius(), 0.25);
    assert!((proxy.points()[0].x - 2.0).abs() <= 1.0e-6);
    assert!((proxy.points()[0].y - 0.0).abs() <= 1.0e-6);
    assert!((proxy.points()[1].x - 0.0).abs() <= 1.0e-6);
    assert!((proxy.points()[1].y + 1.0).abs() <= 1.0e-6);
}

#[test]
fn standalone_collision_apis_reject_invalid_inputs() {
    initialize_foundation();

    let proxy = ShapeProxy::new([[0.0_f32, 0.0]], 0.0).unwrap();
    assert_eq!(
        Transform::from_pos_angle([f32::NAN, 0.0], 0.0).unwrap_err(),
        Error::invalid_argument("Transform::from_pos_angle", "position", "a finite vector",)
    );
    let valid_cast =
        ShapeCastPairInput::new(proxy, proxy, Transform::IDENTITY, [1.0, 0.0]).unwrap();
    let invalid_max_fraction = Error::invalid_argument(
        "ShapeCastPairInput::with_max_fraction",
        "max_fraction",
        "a finite value in 0.0..=1.0",
    );
    assert_eq!(
        valid_cast.with_max_fraction(1.5).unwrap_err(),
        invalid_max_fraction
    );

    assert_eq!(
        Rot::from_raw(boxdd_sys::ffi::b2Rot { c: 2.0, s: 0.0 }).unwrap_err(),
        Error::invalid_argument("Rot::from_raw", "raw", "a normalized finite rotation",)
    );

    let valid_sweep = Sweep::new(
        [0.0_f32, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    )
    .unwrap();
    let toi = ToiInput::new(proxy, proxy, valid_sweep, valid_sweep).unwrap();
    assert_eq!(
        toi.with_max_fraction(1.5).unwrap_err(),
        Error::invalid_argument(
            "ToiInput::with_max_fraction",
            "max_fraction",
            "a finite value in 0.0..=1.0",
        )
    );

    assert_eq!(
        segment_distance(
            [f32::NAN, 0.0],
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [1.0_f32, 0.0]
        )
        .unwrap_err(),
        Error::invalid_argument("segment_distance", "p1", "a finite vector")
    );

    assert_eq!(
        shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]).unwrap_err(),
        Error::invalid_argument(
            "Segment::new",
            "segment",
            "finite endpoints separated by Box2D's minimum segment length",
        )
    );

    assert_eq!(
        shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25).unwrap_err(),
        Error::invalid_argument(
            "Capsule::new",
            "capsule",
            "finite geometry with endpoints separated by Box2D's minimum length and a non-negative radius",
        )
    );
}

#[test]
fn sweep_transform_rejects_invalid_sweeps_and_times() {
    initialize_foundation();

    let valid_sweep = Sweep::new(
        [0.0_f32, 0.0],
        [1.0_f32, 2.0],
        [3.0_f32, 4.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    )
    .unwrap();
    let start = Sweep::transform_at(valid_sweep, 0.0).unwrap();
    let end = Sweep::transform_at(valid_sweep, 1.0).unwrap();

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
            Sweep::transform_at(valid_sweep, invalid_time).unwrap_err(),
            Error::invalid_argument("Sweep::transform_at", "time", "a finite value in 0.0..=1.0",)
        );
    }

    assert!(Rot::from_raw(boxdd_sys::ffi::b2Rot { c: 2.0, s: 0.0 }).is_err());
    assert!(
        Rot::from_raw(boxdd_sys::ffi::b2Rot {
            c: f32::NAN,
            s: 0.0,
        })
        .is_err()
    );
    for (invalid_sweep, argument, constraint) in [
        (
            Sweep::new(
                [f32::NAN, 0.0],
                [1.0_f32, 2.0],
                [3.0_f32, 4.0],
                Rot::IDENTITY,
                Rot::IDENTITY,
            ),
            "local_center",
            "a finite vector",
        ),
        (
            Sweep::new(
                [0.0_f32, 0.0],
                [f32::INFINITY, 2.0],
                [3.0_f32, 4.0],
                Rot::IDENTITY,
                Rot::IDENTITY,
            ),
            "c1",
            "a finite vector",
        ),
        (
            Sweep::new(
                [0.0_f32, 0.0],
                [1.0_f32, 2.0],
                [3.0_f32, f32::NEG_INFINITY],
                Rot::IDENTITY,
                Rot::IDENTITY,
            ),
            "c2",
            "a finite vector",
        ),
    ] {
        assert_eq!(
            invalid_sweep.unwrap_err(),
            Error::invalid_argument("Sweep::validate", argument, constraint)
        );
    }
}

#[test]
fn geometry_constructors_reject_invalid_inputs() {
    initialize_foundation();

    assert_eq!(
        shapes::circle([f32::NAN, 0.0], 0.5).unwrap_err(),
        Error::invalid_argument(
            "Circle::new",
            "circle",
            "finite center coordinates and a finite non-negative radius",
        )
    );
    assert_eq!(
        shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]).unwrap_err(),
        Error::invalid_argument(
            "Segment::new",
            "segment",
            "finite endpoints separated by Box2D's minimum segment length",
        )
    );
    assert_eq!(
        shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25).unwrap_err(),
        Error::invalid_argument(
            "Capsule::new",
            "capsule",
            "finite geometry with endpoints separated by Box2D's minimum length and a non-negative radius",
        )
    );
    assert_eq!(
        shapes::chain_segment(
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [0.0_f32, 0.0],
            [1.0_f32, 0.0]
        )
        .unwrap_err(),
        Error::invalid_argument(
            "ChainSegment::new",
            "chain_segment",
            "finite ghost points and segment endpoints separated by Box2D's minimum length",
        )
    );

    let mut raw_polygon = shapes::box_polygon(1.0, 1.0).unwrap().into_raw();
    raw_polygon.radius = -1.0;
    assert_eq!(
        Polygon::from_raw(raw_polygon).unwrap_err(),
        Error::invalid_argument(
            "Polygon::from_raw",
            "polygon",
            "a valid convex Box2D polygon",
        )
    );
    assert_eq!(
        Polygon::from_points([[f32::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]], 0.0).unwrap_err(),
        Error::invalid_argument("Polygon::from_points", "points", "finite point coordinates",)
    );
}

#[test]
fn safe_manifold_geometry_is_rejected_before_collision() {
    initialize_foundation();

    assert_eq!(
        shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]).unwrap_err(),
        Error::invalid_argument(
            "Segment::new",
            "segment",
            "finite endpoints separated by Box2D's minimum segment length",
        )
    );
}
