use boxdd_sys::ffi;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn math_core_functions() {
    // Test b2ComputeCosSin and b2Atan2 accuracy vs std
    let atan_tol = 0.00004_f32; // ~0.0023 degrees
    for t in (-100..100).map(|i| i as f32 * 0.1) {
        let angle = core::f32::consts::PI * t;
        let cs = unsafe { ffi::b2ComputeCosSin(angle) };
        let (s, c) = angle.sin_cos();
        assert!(approx(cs.sine, s, 0.002));
        assert!(approx(cs.cosine, c, 0.002));

        let a = unsafe { ffi::b2Atan2(s, c) };
        assert!(a.is_finite());
        // Unwind angle to [-pi, pi] using atan2(sin, cos)
        let xn = (angle.sin()).atan2(angle.cos());
        let mut diff = (a - xn).abs();
        if diff > core::f32::consts::PI {
            diff -= 2.0 * core::f32::consts::PI;
        }
        assert!(diff.abs() <= atan_tol);
    }

    // Grid of atan2 comparisons
    let atan_tol = 0.00004_f32;
    let mut y = -1.0_f32;
    while y <= 1.0 {
        let mut x = -1.0_f32;
        while x <= 1.0 {
            let a1 = unsafe { ffi::b2Atan2(y, x) };
            let a2 = y.atan2(x);
            assert!(a1.is_finite());
            assert!((a1 - a2).abs() <= atan_tol);
            x += 0.1;
        }
        y += 0.1;
    }

    // Transform composition and inverse
    let t1 = ffi::b2Transform {
        p: ffi::b2Vec2 { x: -2.0, y: 3.0 },
        q: {
            let a = 1.0_f32;
            ffi::b2Rot {
                c: a.cos(),
                s: a.sin(),
            }
        },
    };
    let t2 = ffi::b2Transform {
        p: ffi::b2Vec2 { x: 1.0, y: 0.0 },
        q: {
            let a = -2.0_f32;
            ffi::b2Rot {
                c: a.cos(),
                s: a.sin(),
            }
        },
    };
    let comp = ffi::b2Transform {
        p: ffi::b2Vec2 {
            x: t2.p.x + t2.q.c * t1.p.x - t2.q.s * t1.p.y,
            y: t2.p.y + t2.q.s * t1.p.x + t2.q.c * t1.p.y,
        },
        q: ffi::b2Rot {
            c: t2.q.c * t1.q.c - t2.q.s * t1.q.s,
            s: t2.q.s * t1.q.c + t2.q.c * t1.q.s,
        },
    };
    // Compare transform point under composition equivalence
    let two = ffi::b2Vec2 { x: 2.0, y: 2.0 };
    fn tform(t: ffi::b2Transform, v: ffi::b2Vec2) -> ffi::b2Vec2 {
        let x = t.q.c * v.x - t.q.s * v.y + t.p.x;
        let y = t.q.s * v.x + t.q.c * v.y + t.p.y;
        ffi::b2Vec2 { x, y }
    }
    fn inv_tform(t: ffi::b2Transform, v: ffi::b2Vec2) -> ffi::b2Vec2 {
        let dx = v.x - t.p.x;
        let dy = v.y - t.p.y;
        let x = t.q.c * dx + t.q.s * dy;
        let y = -t.q.s * dx + t.q.c * dy;
        ffi::b2Vec2 { x, y }
    }
    let v1 = tform(t1, two);
    let v12 = tform(t2, v1);
    let vcomp = tform(comp, two);
    assert!(approx(v12.x, vcomp.x, 1e-5));
    assert!(approx(v12.y, vcomp.y, 1e-5));

    // Inverse transform
    let v_back = inv_tform(t1, v1);
    assert!(approx(v_back.x, two.x, 1e-5));
    assert!(approx(v_back.y, two.y, 1e-5));
}
