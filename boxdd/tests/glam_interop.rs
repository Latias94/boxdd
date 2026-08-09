#![cfg(feature = "glam")]

use boxdd::{
    Aabb, Position, Rot, RotFromGlamError, Transform, TransformFromGlamError, Vec2, WorldScalar,
    WorldTransform, WorldTransformFromInteropError,
};

#[cfg(not(feature = "double-precision"))]
const TEST_WORLD_X: WorldScalar = 10_000.125;
#[cfg(feature = "double-precision")]
const TEST_WORLD_X: WorldScalar = 10_000_000.001;

#[test]
fn rot_converts_to_glam_mat2() {
    let r = Rot::from_radians(0.25).unwrap();
    let m: glam::Mat2 = r.into();
    let _ = m * glam::Vec2::new(1.0, 0.0);
}

#[test]
fn rot_round_trips_through_glam_mat2() {
    let r = Rot::from_radians(0.25).unwrap();
    let m: glam::Mat2 = r.into();
    let r2 = Rot::try_from(m).unwrap();
    assert!((r2.angle() - r.angle()).abs() < 1.0e-6);
}

#[test]
fn transform_converts_to_glam_affine2_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0).unwrap();
    let a: glam::Affine2 = t.into();

    let p = a.transform_point2(glam::Vec2::ZERO);
    assert_eq!(p, glam::Vec2::new(3.0, 4.0));
}

#[test]
fn aabb_converts_to_glam_vec2_tuple() {
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]).unwrap();
    let (l, u): (glam::Vec2, glam::Vec2) = a.into();
    assert_eq!(l, glam::Vec2::new(1.0, 2.0));
    assert_eq!(u, glam::Vec2::new(3.0, 4.0));

    let a2 = Aabb::try_from((l, u)).unwrap();
    assert_eq!(a2.lower(), Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_try_from_glam_affine2_rejects_scaled() {
    let scaled =
        glam::Affine2::from_scale_angle_translation(glam::Vec2::splat(2.0), 0.0, glam::Vec2::ZERO);
    let err = Transform::try_from(scaled).unwrap_err();
    assert_eq!(err, TransformFromGlamError::NotPureRotation);
}

#[test]
fn rot_try_from_glam_mat2_rejects_scaled() {
    let scaled = glam::Mat2::from_cols_array(&[2.0, 0.0, 0.0, 2.0]);
    let err = Rot::try_from(scaled).unwrap_err();
    assert_eq!(err, RotFromGlamError::NotPureRotation);
}

#[test]
fn rot_try_from_glam_mat2_rejects_non_finite() {
    let bad = glam::Mat2::from_cols(glam::Vec2::new(f32::NAN, 0.0), glam::Vec2::Y);
    let err = Rot::try_from(&bad).unwrap_err();
    assert_eq!(err, RotFromGlamError::NonFinite);
}

#[test]
fn world_types_round_trip_through_scalar_correct_glam_representations() {
    let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
    let transform = WorldTransform::new(position, Rot::from_radians(0.375).unwrap()).unwrap();

    #[cfg(not(feature = "double-precision"))]
    {
        let point: glam::Vec2 = position.into();
        assert_eq!(Position::from(point), position);

        let affine: glam::Affine2 = transform.into();
        let round_trip = WorldTransform::try_from(&affine).unwrap();
        assert_eq!(round_trip.position(), position);
        assert!((round_trip.rotation().angle() - transform.rotation().angle()).abs() < 1.0e-6);
    }

    #[cfg(feature = "double-precision")]
    {
        let point: glam::DVec2 = position.into();
        assert_eq!(Position::from(point), position);

        let affine: glam::DAffine2 = transform.into();
        let round_trip = WorldTransform::try_from(&affine).unwrap();
        assert_eq!(round_trip.position(), position);
        assert!((round_trip.rotation().angle() - transform.rotation().angle()).abs() < 1.0e-6);
        assert_ne!(f64::from(TEST_WORLD_X as f32), TEST_WORLD_X);
    }
}

#[test]
fn world_transform_try_from_glam_rejects_scaled_affines() {
    #[cfg(not(feature = "double-precision"))]
    let scaled = glam::Affine2::from_scale(glam::Vec2::splat(2.0));
    #[cfg(feature = "double-precision")]
    let scaled = glam::DAffine2::from_scale(glam::DVec2::splat(2.0));

    let error = WorldTransform::try_from(scaled).unwrap_err();
    assert_eq!(error, WorldTransformFromInteropError::NotPureRotation);
}
