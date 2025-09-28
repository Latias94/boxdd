use boxdd_sys::ffi;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn segment_and_shape_distance_and_toi() {
    // Segment distance
    let p1 = ffi::b2Vec2 { x: -1.0, y: -1.0 };
    let q1 = ffi::b2Vec2 { x: -1.0, y: 1.0 };
    let p2 = ffi::b2Vec2 { x: 2.0, y: 0.0 };
    let q2 = ffi::b2Vec2 { x: 1.0, y: 0.0 };
    let res = unsafe { ffi::b2SegmentDistance(p1, q1, p2, q2) };
    assert!(approx(res.fraction1, 0.5, f32::EPSILON));
    assert!(approx(res.fraction2, 1.0, f32::EPSILON));
    assert!(
        approx(res.closest1.x, -1.0, f32::EPSILON) && approx(res.closest1.y, 0.0, f32::EPSILON)
    );
    assert!(approx(res.closest2.x, 1.0, f32::EPSILON) && approx(res.closest2.y, 0.0, f32::EPSILON));
    assert!(approx(res.distanceSquared, 4.0, f32::EPSILON));

    // Shape distance
    let vas = [
        ffi::b2Vec2 { x: -1.0, y: -1.0 },
        ffi::b2Vec2 { x: 1.0, y: -1.0 },
        ffi::b2Vec2 { x: 1.0, y: 1.0 },
        ffi::b2Vec2 { x: -1.0, y: 1.0 },
    ];
    let vbs = [
        ffi::b2Vec2 { x: 2.0, y: -1.0 },
        ffi::b2Vec2 { x: 2.0, y: 1.0 },
    ];
    let input = ffi::b2DistanceInput {
        proxyA: unsafe { ffi::b2MakeProxy(vas.as_ptr(), vas.len() as i32, 0.0) },
        proxyB: unsafe { ffi::b2MakeProxy(vbs.as_ptr(), vbs.len() as i32, 0.0) },
        transformA: ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        },
        transformB: ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        },
        useRadii: false,
    };
    let mut cache = ffi::b2SimplexCache {
        count: 0,
        indexA: [0; 3],
        indexB: [0; 3],
    };
    let out = unsafe { ffi::b2ShapeDistance(&input, &mut cache, core::ptr::null_mut(), 0) };
    assert!(approx(out.distance, 1.0, f32::EPSILON));

    // Shape cast
    let input_sc = ffi::b2ShapeCastPairInput {
        proxyA: input.proxyA,
        proxyB: input.proxyB,
        transformA: input.transformA,
        transformB: input.transformB,
        translationB: ffi::b2Vec2 { x: -2.0, y: 0.0 },
        maxFraction: 1.0,
        canEncroach: false,
    };
    let outc = unsafe { ffi::b2ShapeCast(&input_sc) };
    assert!(outc.hit);
    assert!(approx(outc.fraction, 0.5, 0.005));

    // Time of impact
    let toi_in = ffi::b2TOIInput {
        proxyA: input.proxyA,
        proxyB: input.proxyB,
        sweepA: ffi::b2Sweep {
            localCenter: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            c1: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            c2: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q1: ffi::b2Rot { c: 1.0, s: 0.0 },
            q2: ffi::b2Rot { c: 1.0, s: 0.0 },
        },
        sweepB: ffi::b2Sweep {
            localCenter: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            c1: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            c2: ffi::b2Vec2 { x: -2.0, y: 0.0 },
            q1: ffi::b2Rot { c: 1.0, s: 0.0 },
            q2: ffi::b2Rot { c: 1.0, s: 0.0 },
        },
        maxFraction: 1.0,
    };
    let to = unsafe { ffi::b2TimeOfImpact(&toi_in) };
    assert_eq!(to.state, ffi::b2TOIState_b2_toiStateHit);
    assert!(approx(to.fraction, 0.5, 0.005));
}
