use boxdd::{
    CastOutput, DistanceInput, DistanceOutput, MAX_SHAPE_PROXY_POINTS, Rot, SegmentDistanceResult,
    ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput, ToiOutput, ToiState, Transform,
    segment_distance, shape_cast, shape_distance, time_of_impact,
};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn shape_proxy_rejects_invalid_point_counts() {
    assert!(ShapeProxy::new(core::iter::empty::<[f32; 2]>(), 0.0).is_err());

    let too_many = (0..=MAX_SHAPE_PROXY_POINTS).map(|i| [i as f32, 0.0]);
    assert!(ShapeProxy::new(too_many, 0.0).is_err());
}

#[test]
fn segment_and_shape_distance_and_toi() {
    initialize_foundation();

    let res = segment_distance([-1.0, -1.0], [-1.0, 1.0], [2.0, 0.0], [1.0, 0.0]).unwrap();
    assert!(approx(res.fraction1, 0.5, f32::EPSILON));
    assert!(approx(res.fraction2, 1.0, f32::EPSILON));
    assert!(approx(res.closest1.x, -1.0, f32::EPSILON));
    assert!(approx(res.closest1.y, 0.0, f32::EPSILON));
    assert!(approx(res.closest2.x, 1.0, f32::EPSILON));
    assert!(approx(res.closest2.y, 0.0, f32::EPSILON));
    assert!(approx(res.distance_squared, 4.0, f32::EPSILON));

    let proxy_a =
        ShapeProxy::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]], 0.0).unwrap();
    let proxy_b = ShapeProxy::new([[0.0, -1.0], [0.0, 1.0]], 0.0).unwrap();
    let transform_b_in_a = Transform::from_pos_angle([2.0_f32, 0.0], 0.0).unwrap();

    let mut cache = SimplexCache::default();
    let out = shape_distance(
        DistanceInput::new(proxy_a, proxy_b, transform_b_in_a).unwrap(),
        &mut cache,
    )
    .unwrap();
    assert!(approx(out.distance, 1.0, f32::EPSILON));
    assert!(cache.count() > 0);

    let outc = shape_cast(
        ShapeCastPairInput::new(proxy_a, proxy_b, transform_b_in_a, [-2.0, 0.0]).unwrap(),
    )
    .unwrap();
    assert!(outc.hit);
    assert!(approx(outc.fraction, 0.5, 0.005));

    let toi = time_of_impact(
        ToiInput::new(
            proxy_a,
            proxy_b,
            Sweep::new(
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                Rot::IDENTITY,
                Rot::IDENTITY,
            )
            .unwrap(),
            Sweep::new(
                [0.0, 0.0],
                [2.0, 0.0],
                [0.0, 0.0],
                Rot::IDENTITY,
                Rot::IDENTITY,
            )
            .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
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
    })
    .unwrap();
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
    })
    .unwrap();
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
    })
    .unwrap();
    assert!(approx(distance.point_a.x, -2.0, f32::EPSILON));
    assert!(approx(distance.point_b.y, 5.0, f32::EPSILON));
    assert!(approx(distance.normal.x, 1.0, f32::EPSILON));
    assert!(approx(distance.distance, 5.25, f32::EPSILON));
    assert_eq!(distance.iterations, 3);
    assert_eq!(distance.simplex_count, 2);

    assert_eq!(
        ToiState::from_raw(boxdd_sys::ffi::b2TOIState_b2_toiStateSeparated),
        Some(ToiState::Separated)
    );
    assert_eq!(
        ToiState::from_raw(boxdd_sys::ffi::b2TOIState_b2_toiStateHit),
        Some(ToiState::Hit)
    );

    let toi = ToiOutput::from_raw(boxdd_sys::ffi::b2TOIOutput {
        state: boxdd_sys::ffi::b2TOIState_b2_toiStateOverlapped,
        point: boxdd_sys::ffi::b2Vec2 { x: 0.5, y: -0.5 },
        normal: boxdd_sys::ffi::b2Vec2 { x: -1.0, y: 0.0 },
        fraction: 0.6,
    })
    .unwrap();
    assert_eq!(toi.state, ToiState::Overlapped);
    assert!(approx(toi.point.x, 0.5, f32::EPSILON));
    assert!(approx(toi.point.y, -0.5, f32::EPSILON));
    assert!(approx(toi.normal.x, -1.0, f32::EPSILON));
    assert!(approx(toi.fraction, 0.6, f32::EPSILON));
}

#[test]
fn collision_result_raw_conversions_reject_invalid_values() {
    let raw_segment = boxdd_sys::ffi::b2SegmentDistanceResult {
        closest1: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 0.0 },
        closest2: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        fraction1: 0.25,
        fraction2: 0.75,
        distanceSquared: 1.0,
    };
    assert!(
        SegmentDistanceResult::from_raw(boxdd_sys::ffi::b2SegmentDistanceResult {
            distanceSquared: -1.0,
            ..raw_segment
        })
        .is_err()
    );
    assert!(
        SegmentDistanceResult::from_raw(boxdd_sys::ffi::b2SegmentDistanceResult {
            fraction1: f32::NAN,
            ..raw_segment
        })
        .is_err()
    );

    let raw_cast = boxdd_sys::ffi::b2CastOutput {
        normal: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        point: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 0.0 },
        fraction: 0.5,
        iterations: 1,
        hit: true,
    };
    assert!(
        CastOutput::from_raw(boxdd_sys::ffi::b2CastOutput {
            normal: boxdd_sys::ffi::b2Vec2 { x: 2.0, y: 0.0 },
            ..raw_cast
        })
        .is_err()
    );
    assert!(
        CastOutput::from_raw(boxdd_sys::ffi::b2CastOutput {
            iterations: -1,
            ..raw_cast
        })
        .is_err()
    );

    let raw_distance = boxdd_sys::ffi::b2DistanceOutput {
        pointA: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 0.0 },
        pointB: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        normal: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        distance: 1.0,
        iterations: 1,
        simplexCount: 0,
    };
    assert!(
        DistanceOutput::from_raw(boxdd_sys::ffi::b2DistanceOutput {
            distance: -1.0,
            ..raw_distance
        })
        .is_err()
    );
    assert!(
        DistanceOutput::from_raw(boxdd_sys::ffi::b2DistanceOutput {
            normal: boxdd_sys::ffi::b2Vec2 {
                x: f32::NAN,
                y: 0.0,
            },
            ..raw_distance
        })
        .is_err()
    );

    assert_eq!(ToiState::from_raw(u32::MAX), None);
    let raw_toi = boxdd_sys::ffi::b2TOIOutput {
        state: boxdd_sys::ffi::b2TOIState_b2_toiStateHit,
        point: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 0.0 },
        normal: boxdd_sys::ffi::b2Vec2 { x: 1.0, y: 0.0 },
        fraction: 0.5,
    };
    assert!(
        ToiOutput::from_raw(boxdd_sys::ffi::b2TOIOutput {
            state: u32::MAX,
            ..raw_toi
        })
        .is_err()
    );
    assert!(
        ToiOutput::from_raw(boxdd_sys::ffi::b2TOIOutput {
            normal: boxdd_sys::ffi::b2Vec2 { x: 0.5, y: 0.0 },
            ..raw_toi
        })
        .is_err()
    );
    assert!(
        ToiOutput::from_raw(boxdd_sys::ffi::b2TOIOutput {
            fraction: 1.5,
            ..raw_toi
        })
        .is_err()
    );
}

#[test]
fn collision_input_types_use_explicit_raw_conversions() {
    initialize_foundation();

    let proxy_a =
        ShapeProxy::new([[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]], 0.25).unwrap();
    let proxy_b = ShapeProxy::new([[2.0, -1.0], [2.0, 1.0]], 0.5).unwrap();
    let transform_b_in_a = Transform::from_pos_angle([3.0_f32, -2.0], 0.25).unwrap();

    let distance_input = DistanceInput::new(proxy_a, proxy_b, transform_b_in_a)
        .unwrap()
        .with_radii(true);
    let raw_distance = distance_input.into_raw();
    assert_eq!(raw_distance.proxyA.count, 4);
    assert_eq!(raw_distance.proxyB.count, 2);
    assert!(raw_distance.useRadii);
    assert!(approx(raw_distance.proxyA.points[0].x, -1.0, f32::EPSILON));
    assert!(approx(raw_distance.proxyA.radius, 0.25, f32::EPSILON));
    assert!(approx(raw_distance.proxyB.points[0].x, 2.0, f32::EPSILON));
    assert!(approx(raw_distance.proxyB.radius, 0.5, f32::EPSILON));
    assert!(approx(raw_distance.transform.p.x, 3.0, f32::EPSILON));
    assert!(approx(raw_distance.transform.p.y, -2.0, f32::EPSILON));

    let cast_input = ShapeCastPairInput::new(proxy_a, proxy_b, transform_b_in_a, [-2.0, 0.5])
        .unwrap()
        .with_max_fraction(0.75)
        .unwrap()
        .with_can_encroach(true);
    let raw_cast = cast_input.into_raw();
    assert_eq!(raw_cast.proxyA.count, 4);
    assert_eq!(raw_cast.proxyB.count, 2);
    assert!(approx(raw_cast.transform.p.x, 3.0, f32::EPSILON));
    assert!(approx(raw_cast.transform.p.y, -2.0, f32::EPSILON));
    assert!(approx(raw_cast.translationB.x, -2.0, f32::EPSILON));
    assert!(approx(raw_cast.translationB.y, 0.5, f32::EPSILON));
    assert!(approx(raw_cast.maxFraction, 0.75, f32::EPSILON));
    assert!(raw_cast.canEncroach);

    let sweep = Sweep::new(
        [1.0, 2.0],
        [3.0, 4.0],
        [5.0, 6.0],
        Rot::from_degrees(10.0).unwrap(),
        Rot::from_degrees(20.0).unwrap(),
    )
    .unwrap();
    let raw_sweep = sweep.into_raw();
    assert!(approx(raw_sweep.localCenter.x, 1.0, f32::EPSILON));
    assert!(approx(raw_sweep.localCenter.y, 2.0, f32::EPSILON));
    assert!(approx(raw_sweep.c1.x, 3.0, f32::EPSILON));
    assert!(approx(raw_sweep.c2.y, 6.0, f32::EPSILON));

    let sweep_roundtrip = Sweep::from_raw(raw_sweep).unwrap();
    assert!(approx(sweep_roundtrip.local_center().x, 1.0, f32::EPSILON));
    assert!(approx(sweep_roundtrip.local_center().y, 2.0, f32::EPSILON));
    assert!(approx(sweep_roundtrip.start_center().x, 3.0, f32::EPSILON));
    assert!(approx(sweep_roundtrip.start_center().y, 4.0, f32::EPSILON));
    assert!(approx(sweep_roundtrip.end_center().x, 5.0, f32::EPSILON));
    assert!(approx(sweep_roundtrip.end_center().y, 6.0, f32::EPSILON));
    assert!(approx(
        sweep_roundtrip.start_rotation().angle(),
        Rot::from_degrees(10.0).unwrap().angle(),
        1.0e-5
    ));
    assert!(approx(
        sweep_roundtrip.end_rotation().angle(),
        Rot::from_degrees(20.0).unwrap().angle(),
        1.0e-5
    ));

    let toi_input = ToiInput::new(proxy_a, proxy_b, sweep, sweep_roundtrip)
        .unwrap()
        .with_max_fraction(0.9)
        .unwrap();
    let raw_toi = toi_input.into_raw();
    assert_eq!(raw_toi.proxyA.count, 4);
    assert_eq!(raw_toi.proxyB.count, 2);
    assert!(approx(raw_toi.maxFraction, 0.9, f32::EPSILON));
    assert!(approx(raw_toi.sweepA.localCenter.x, 1.0, f32::EPSILON));
    assert!(approx(raw_toi.sweepB.c2.y, 6.0, f32::EPSILON));
}
