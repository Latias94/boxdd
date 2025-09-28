use boxdd_sys::ffi;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn aabb_ray_cast(
    aabb: ffi::b2AABB,
    p1: ffi::b2Vec2,
    p2: ffi::b2Vec2,
) -> (bool, f32, ffi::b2Vec2, ffi::b2Vec2) {
    // Slab method
    let d = ffi::b2Vec2 {
        x: p2.x - p1.x,
        y: p2.y - p1.y,
    };
    let mut tmin = 0.0_f32;
    let mut tmax = 1.0_f32;
    let mut normal = ffi::b2Vec2 { x: 0.0, y: 0.0 };

    // X slab
    if d.x.abs() < f32::EPSILON {
        if p1.x < aabb.lowerBound.x || p1.x > aabb.upperBound.x {
            return (false, 0.0, p1, normal);
        }
    } else {
        let inv_d = 1.0 / d.x;
        let mut t1 = (aabb.lowerBound.x - p1.x) * inv_d;
        let mut t2 = (aabb.upperBound.x - p1.x) * inv_d;
        let mut n1 = ffi::b2Vec2 { x: -1.0, y: 0.0 };
        let mut n2 = ffi::b2Vec2 { x: 1.0, y: 0.0 };
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
            core::mem::swap(&mut n1, &mut n2);
        }
        if t1 > tmin {
            tmin = t1;
            normal = n1;
        }
        if t2 < tmax {
            tmax = t2;
        }
        if tmin > tmax {
            return (false, 0.0, p1, normal);
        }
    }

    // Y slab
    if d.y.abs() < f32::EPSILON {
        if p1.y < aabb.lowerBound.y || p1.y > aabb.upperBound.y {
            return (false, 0.0, p1, normal);
        }
    } else {
        let inv_d = 1.0 / d.y;
        let mut t1 = (aabb.lowerBound.y - p1.y) * inv_d;
        let mut t2 = (aabb.upperBound.y - p1.y) * inv_d;
        let mut n1 = ffi::b2Vec2 { x: 0.0, y: -1.0 };
        let mut n2 = ffi::b2Vec2 { x: 0.0, y: 1.0 };
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
            core::mem::swap(&mut n1, &mut n2);
        }
        if t1 > tmin {
            tmin = t1;
            normal = n1;
        }
        if t2 < tmax {
            tmax = t2;
        }
        if tmin > tmax {
            return (false, 0.0, p1, normal);
        }
    }

    if !(0.0..=1.0).contains(&tmin) {
        return (false, 0.0, p1, normal);
    }
    let point = ffi::b2Vec2 {
        x: p1.x + tmin * d.x,
        y: p1.y + tmin * d.y,
    };
    (true, tmin, point, normal)
}

#[test]
fn aabb_valid_and_raycast() {
    // validity
    let mut aabb = ffi::b2AABB {
        lowerBound: ffi::b2Vec2 { x: -1.0, y: -1.0 },
        upperBound: ffi::b2Vec2 { x: -2.0, y: -2.0 },
    };
    assert!(!unsafe { ffi::b2IsValidAABB(aabb) });
    aabb.upperBound = ffi::b2Vec2 { x: 1.0, y: 1.0 };
    assert!(unsafe { ffi::b2IsValidAABB(aabb) });

    // raycast tests
    let aabb = ffi::b2AABB {
        lowerBound: ffi::b2Vec2 { x: -1.0, y: -1.0 },
        upperBound: ffi::b2Vec2 { x: 1.0, y: 1.0 },
    };
    // left side
    let (hit, frac, pt, n) = aabb_ray_cast(
        aabb,
        ffi::b2Vec2 { x: -3.0, y: 0.0 },
        ffi::b2Vec2 { x: 3.0, y: 0.0 },
    );
    assert!(
        hit && approx(frac, 1.0 / 3.0, f32::EPSILON)
            && approx(n.x, -1.0, f32::EPSILON)
            && approx(pt.x, -1.0, f32::EPSILON)
    );
    // right side
    let (hit, frac, pt, n) = aabb_ray_cast(
        aabb,
        ffi::b2Vec2 { x: 3.0, y: 0.0 },
        ffi::b2Vec2 { x: -3.0, y: 0.0 },
    );
    assert!(
        hit && approx(frac, 1.0 / 3.0, f32::EPSILON)
            && approx(n.x, 1.0, f32::EPSILON)
            && approx(pt.x, 1.0, f32::EPSILON)
    );
    // bottom
    let (hit, _, pt, n) = aabb_ray_cast(
        aabb,
        ffi::b2Vec2 { x: 0.0, y: -3.0 },
        ffi::b2Vec2 { x: 0.0, y: 3.0 },
    );
    assert!(hit && approx(n.y, -1.0, f32::EPSILON) && approx(pt.y, -1.0, f32::EPSILON));
    // top
    let (hit, _, pt, n) = aabb_ray_cast(
        aabb,
        ffi::b2Vec2 { x: 0.0, y: 3.0 },
        ffi::b2Vec2 { x: 0.0, y: -3.0 },
    );
    assert!(hit && approx(n.y, 1.0, f32::EPSILON) && approx(pt.y, 1.0, f32::EPSILON));
    // miss
    let (hit, _, _, _) = aabb_ray_cast(
        aabb,
        ffi::b2Vec2 { x: -3.0, y: 2.0 },
        ffi::b2Vec2 { x: 3.0, y: 2.0 },
    );
    assert!(!hit);
}
