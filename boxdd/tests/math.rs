use boxdd::{
    Plane, Rot, Transform, Vec2, atan2, compute_cos_sin, rotation_between_unit_vectors, version,
};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn math_core_functions() {
    // Test deterministic cosine/sine and atan2 helpers against std.
    let atan_tol = 0.00004_f32; // ~0.0023 degrees
    for t in (-100..100).map(|i| i as f32 * 0.1) {
        let angle = core::f32::consts::PI * t;
        let cs = compute_cos_sin(angle);
        let (s, c) = angle.sin_cos();
        assert!(approx(cs.sine(), s, 0.002));
        assert!(approx(cs.cosine(), c, 0.002));

        let a = atan2(s, c);
        assert!(a.is_finite());
        let xn = (angle.sin()).atan2(angle.cos());
        let mut diff = (a - xn).abs();
        if diff > core::f32::consts::PI {
            diff -= 2.0 * core::f32::consts::PI;
        }
        assert!(diff.abs() <= atan_tol);
    }

    let atan_tol = 0.00004_f32;
    let mut y = -1.0_f32;
    while y <= 1.0 {
        let mut x = -1.0_f32;
        while x <= 1.0 {
            let a1 = atan2(y, x);
            let a2 = y.atan2(x);
            assert!(a1.is_finite());
            assert!((a1 - a2).abs() <= atan_tol);
            x += 0.1;
        }
        y += 0.1;
    }

    // Transform composition and inverse.
    let t1 = Transform::from_pos_angle([-2.0, 3.0], 1.0);
    let t2 = Transform::from_pos_angle([1.0, 0.0], -2.0);
    let r1 = t1.rotation();
    let p1 = t1.position();
    let r2 = t2.rotation();
    let p2 = t2.position();
    let composed_p = {
        let rotated = r2.rotate_vec(p1);
        Vec2::new(rotated.x + p2.x, rotated.y + p2.y)
    };
    let composed_r = Rot::from_radians(r2.angle() + r1.angle());

    let two = Vec2 { x: 2.0, y: 2.0 };
    let v1 = t1.transform_point(two);
    let v12 = t2.transform_point(v1);
    let vcomp_rotated = composed_r.rotate_vec(two);
    let vcomp = Vec2::new(
        vcomp_rotated.x + composed_p.x,
        vcomp_rotated.y + composed_p.y,
    );
    assert!(approx(v12.x, vcomp.x, 1e-5));
    assert!(approx(v12.y, vcomp.y, 1e-5));

    let v_back = t1.inv_transform_point(v1);
    assert!(approx(v_back.x, two.x, 1e-5));
    assert!(approx(v_back.y, two.y, 1e-5));
}

#[test]
fn public_math_helpers_cover_validity_rotation_and_version() {
    assert!(Vec2::new(1.0, 2.0).is_valid());
    assert!(!Vec2::new(f32::NAN, 2.0).is_valid());

    let rot = compute_cos_sin(core::f32::consts::FRAC_PI_2);
    assert!(rot.is_valid());
    assert!(approx(rot.rotate_vec(Vec2::new(1.0, 0.0)).x, 0.0, 0.002));
    assert!(approx(rot.rotate_vec(Vec2::new(1.0, 0.0)).y, 1.0, 0.002));

    let between = rotation_between_unit_vectors([1.0, 0.0], [0.0, 1.0]);
    assert!(between.is_valid());
    let turned = between.rotate_vec(Vec2::new(1.0, 0.0));
    assert!(approx(turned.x, 0.0, 1e-5));
    assert!(approx(turned.y, 1.0, 1e-5));

    let by_method = Rot::from_unit_vectors([0.0, 1.0], [-1.0, 0.0]);
    let turned = by_method.rotate_vec(Vec2::new(0.0, 1.0));
    assert!(approx(turned.x, -1.0, 1e-5));
    assert!(approx(turned.y, 0.0, 1e-5));

    assert!(Transform::from_pos_angle([0.0, 0.0], 0.25).is_valid());
    assert!(!Transform::from_pos_angle([f32::NAN, 0.0], 0.25).is_valid());

    assert!(Plane::new([0.0, 1.0], 0.5).is_valid());
    assert!(!Plane::new([0.0, 2.0], 0.5).is_valid());

    let v = version();
    assert!(v.major >= 3);
}
