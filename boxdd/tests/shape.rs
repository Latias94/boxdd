use boxdd_sys::ffi;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn shape_mass_aabb_point_raycast() {
    let circle = ffi::b2Circle {
        center: ffi::b2Vec2 { x: 1.0, y: 0.0 },
        radius: 1.0,
    };
    let boxp = unsafe { ffi::b2MakeBox(1.0, 1.0) };
    let segment = ffi::b2Segment {
        point1: ffi::b2Vec2 { x: 0.0, y: 1.0 },
        point2: ffi::b2Vec2 { x: 0.0, y: -1.0 },
    };

    // Mass: circle
    let md = unsafe { ffi::b2ComputeCircleMass(&circle, 1.0) };
    assert!(approx(md.mass, core::f32::consts::PI, f32::EPSILON));
    assert!(approx(md.center.x, 1.0, f32::EPSILON) && approx(md.center.y, 0.0, f32::EPSILON));
    assert!(approx(
        md.rotationalInertia,
        0.5 * core::f32::consts::PI,
        f32::EPSILON
    ));

    // Mass: capsule sandwich between hull and box bound
    let capsule = ffi::b2Capsule {
        center1: ffi::b2Vec2 { x: -1.0, y: 0.0 },
        center2: ffi::b2Vec2 { x: 1.0, y: 0.0 },
        radius: 1.0,
    };
    let m_cap = unsafe { ffi::b2ComputeCapsuleMass(&capsule, 1.0) };
    let dx = capsule.center2.x - capsule.center1.x;
    let dy = capsule.center2.y - capsule.center1.y;
    let length = (dx * dx + dy * dy).sqrt();
    let r = unsafe { ffi::b2MakeBox(capsule.radius, capsule.radius + 0.5 * length) };
    let m_box = unsafe { ffi::b2ComputePolygonMass(&r, 1.0) };
    // approximate capsule via hull
    const N: usize = 4;
    let mut pts = [ffi::b2Vec2 { x: 0.0, y: 0.0 }; 2 * N];
    let mut angle = -0.5 * core::f32::consts::PI;
    let d = core::f32::consts::PI / (N as f32 - 1.0);
    for p in pts.iter_mut().take(N) {
        p.x = 1.0 + capsule.radius * angle.cos();
        p.y = capsule.radius * angle.sin();
        angle += d;
    }
    angle = 0.5 * core::f32::consts::PI;
    for p in pts.iter_mut().skip(N).take(N) {
        p.x = -1.0 + capsule.radius * angle.cos();
        p.y = capsule.radius * angle.sin();
        angle += d;
    }
    let hull = unsafe { ffi::b2ComputeHull(pts.as_ptr(), (2 * N) as i32) };
    let ac = unsafe { ffi::b2MakePolygon(&hull, 0.0) };
    let m_hull = unsafe { ffi::b2ComputePolygonMass(&ac, 1.0) };
    assert!(m_hull.mass < m_cap.mass && m_cap.mass < m_box.mass);
    assert!(
        m_hull.rotationalInertia < m_cap.rotationalInertia
            && m_cap.rotationalInertia < m_box.rotationalInertia
    );

    // Mass: box
    let m = unsafe { ffi::b2ComputePolygonMass(&boxp, 1.0) };
    assert!(approx(m.mass, 4.0, f32::EPSILON));
    assert!(approx(m.center.x, 0.0, f32::EPSILON));
    assert!(approx(m.center.y, 0.0, f32::EPSILON));
    assert!(approx(m.rotationalInertia, 8.0 / 3.0, 2.0 * f32::EPSILON));

    // AABB
    let a_circle = unsafe {
        ffi::b2ComputeCircleAABB(
            &circle,
            ffi::b2Transform {
                p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
                q: ffi::b2Rot { c: 1.0, s: 0.0 },
            },
        )
    };
    assert!(approx(a_circle.lowerBound.x, 0.0, f32::EPSILON));
    assert!(approx(a_circle.lowerBound.y, -1.0, f32::EPSILON));
    assert!(approx(a_circle.upperBound.x, 2.0, f32::EPSILON));
    assert!(approx(a_circle.upperBound.y, 1.0, f32::EPSILON));

    let a_box = unsafe {
        ffi::b2ComputePolygonAABB(
            &boxp,
            ffi::b2Transform {
                p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
                q: ffi::b2Rot { c: 1.0, s: 0.0 },
            },
        )
    };
    assert!(approx(a_box.lowerBound.x, -1.0, f32::EPSILON));
    assert!(approx(a_box.lowerBound.y, -1.0, f32::EPSILON));
    assert!(approx(a_box.upperBound.x, 1.0, f32::EPSILON));
    assert!(approx(a_box.upperBound.y, 1.0, f32::EPSILON));

    let a_seg = unsafe {
        ffi::b2ComputeSegmentAABB(
            &segment,
            ffi::b2Transform {
                p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
                q: ffi::b2Rot { c: 1.0, s: 0.0 },
            },
        )
    };
    assert!(approx(a_seg.lowerBound.x, 0.0, f32::EPSILON));
    assert!(approx(a_seg.lowerBound.y, -1.0, f32::EPSILON));
    assert!(approx(a_seg.upperBound.x, 0.0, f32::EPSILON));
    assert!(approx(a_seg.upperBound.y, 1.0, f32::EPSILON));

    // Point in shape
    let p1 = ffi::b2Vec2 { x: 0.5, y: 0.5 };
    let p2 = ffi::b2Vec2 { x: 4.0, y: -4.0 };
    assert!(unsafe { ffi::b2PointInCircle(&circle, p1) });
    assert!(!unsafe { ffi::b2PointInCircle(&circle, p2) });
    assert!(unsafe { ffi::b2PointInPolygon(&boxp, p1) });
    assert!(!unsafe { ffi::b2PointInPolygon(&boxp, p2) });

    // Ray casts
    let input = ffi::b2RayCastInput {
        origin: ffi::b2Vec2 { x: -4.0, y: 0.0 },
        translation: ffi::b2Vec2 { x: 8.0, y: 0.0 },
        maxFraction: 1.0,
    };
    let out_c = unsafe { ffi::b2RayCastCircle(&circle, &input) };
    assert!(out_c.hit);
    assert!(
        approx(out_c.normal.x, -1.0, f32::EPSILON) && approx(out_c.normal.y, 0.0, f32::EPSILON)
    );
    assert!(approx(out_c.fraction, 0.5, f32::EPSILON));

    let out_p = unsafe { ffi::b2RayCastPolygon(&boxp, &input) };
    assert!(out_p.hit);
    assert!(
        approx(out_p.normal.x, -1.0, f32::EPSILON) && approx(out_p.normal.y, 0.0, f32::EPSILON)
    );
    assert!(approx(out_p.fraction, 3.0 / 8.0, f32::EPSILON));
}
