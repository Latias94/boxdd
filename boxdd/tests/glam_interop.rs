#![cfg(feature = "glam")]

use boxdd::{Aabb, Rot, Transform, TransformFromGlamError, Vec2};

#[test]
fn rot_converts_to_glam_mat2() {
    let r = Rot::from_radians(0.25);
    let m: glam::Mat2 = r.into();
    let _ = m * glam::Vec2::new(1.0, 0.0);
}

#[test]
fn transform_converts_to_glam_affine2_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0);
    let a: glam::Affine2 = t.into();

    let p = a.transform_point2(glam::Vec2::ZERO);
    assert_eq!(p, glam::Vec2::new(3.0, 4.0));
}

#[test]
fn aabb_converts_to_glam_vec2_tuple() {
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]);
    let (l, u): (glam::Vec2, glam::Vec2) = a.into();
    assert_eq!(l, glam::Vec2::new(1.0, 2.0));
    assert_eq!(u, glam::Vec2::new(3.0, 4.0));

    let a2 = Aabb::from((l, u));
    assert_eq!(a2.lower, Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper, Vec2::new(3.0, 4.0));
}

#[test]
fn transform_try_from_glam_affine2_rejects_scaled() {
    let scaled =
        glam::Affine2::from_scale_angle_translation(glam::Vec2::splat(2.0), 0.0, glam::Vec2::ZERO);
    let err = Transform::try_from(scaled).unwrap_err();
    assert_eq!(err, TransformFromGlamError::NotPureRotation);
}
