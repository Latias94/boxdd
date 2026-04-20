#![cfg(feature = "cgmath")]

use boxdd::{Aabb, Rot, Transform, TransformFromCgmathError, Vec2};

#[test]
fn vec2_converts_to_and_from_cgmath() {
    let v = Vec2::new(1.0, 2.0);
    let cv: cgmath::Vector2<f32> = v.into();
    assert_eq!(cv.x, 1.0);
    assert_eq!(cv.y, 2.0);

    let v2: Vec2 = cv.into();
    assert_eq!(v2, v);

    let cp: cgmath::Point2<f32> = v.into();
    let v3: Vec2 = cp.into();
    assert_eq!(v3, v);
}

#[test]
fn aabb_converts_to_and_from_cgmath_tuples() {
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]);

    let (lp, up): (cgmath::Point2<f32>, cgmath::Point2<f32>) = a.into();
    assert_eq!(lp.x, 1.0);
    assert_eq!(lp.y, 2.0);
    assert_eq!(up.x, 3.0);
    assert_eq!(up.y, 4.0);

    let a2 = Aabb::from((lp, up));
    assert_eq!(a2.lower, Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper, Vec2::new(3.0, 4.0));

    let (lv, uv): (cgmath::Vector2<f32>, cgmath::Vector2<f32>) = a.into();
    let a3 = Aabb::from((lv, uv));
    assert_eq!(a3, a2);
}

#[test]
fn rot_round_trips_through_cgmath_basis2() {
    let r = Rot::from_radians(0.25);
    let basis: cgmath::Basis2<f32> = r.into();
    let r2 = Rot::from(&basis);
    assert!((r2.angle() - r.angle()).abs() < 1.0e-6);
}

#[test]
fn transform_converts_to_cgmath_matrix3_translation_matches() {
    use cgmath::Vector3;
    let t = Transform::from_pos_angle([3.0, 4.0], 0.0);
    let m: cgmath::Matrix3<f32> = t.into();
    let p = m * Vector3::new(0.0, 0.0, 1.0);
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 4.0);
    assert_eq!(p.z, 1.0);

    let t2 = Transform::try_from(m).unwrap();
    assert_eq!(t2.position(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_try_from_cgmath_rejects_scaled() {
    use cgmath::Vector3;
    let m = cgmath::Matrix3 {
        x: Vector3::new(2.0, 0.0, 0.0),
        y: Vector3::new(0.0, 2.0, 0.0),
        z: Vector3::new(0.0, 0.0, 1.0),
    };
    let err = Transform::try_from(m).unwrap_err();
    assert_eq!(err, TransformFromCgmathError::NotPureRotation);
}
