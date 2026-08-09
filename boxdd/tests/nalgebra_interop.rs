#![cfg(feature = "nalgebra")]

use boxdd::{
    Aabb, Position, Rot, Transform, Vec2, WorldScalar, WorldTransform,
    WorldTransformFromInteropError,
};

#[cfg(not(feature = "double-precision"))]
const TEST_WORLD_X: WorldScalar = 10_000.125;
#[cfg(feature = "double-precision")]
const TEST_WORLD_X: WorldScalar = 10_000_000.001;

#[test]
fn vec2_converts_to_and_from_nalgebra() {
    let v = Vec2::new(1.0, 2.0);
    let nv: nalgebra::Vector2<f32> = v.into();
    assert_eq!(nv.x, 1.0);
    assert_eq!(nv.y, 2.0);

    let v2: Vec2 = nv.into();
    assert_eq!(v2, v);

    let np: nalgebra::Point2<f32> = v.into();
    let v3: Vec2 = np.into();
    assert_eq!(v3, v);
}

#[test]
fn aabb_converts_to_and_from_nalgebra_tuples() {
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]).unwrap();

    let (lp, up): (nalgebra::Point2<f32>, nalgebra::Point2<f32>) = a.into();
    assert_eq!(lp.x, 1.0);
    assert_eq!(lp.y, 2.0);
    assert_eq!(up.x, 3.0);
    assert_eq!(up.y, 4.0);

    let a2 = Aabb::try_from((lp, up)).unwrap();
    assert_eq!(a2.lower(), Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper(), Vec2::new(3.0, 4.0));

    let (lv, uv): (nalgebra::Vector2<f32>, nalgebra::Vector2<f32>) = a.into();
    let a3 = Aabb::try_from((lv, uv)).unwrap();
    assert_eq!(a3, a2);
}

#[test]
fn rot_round_trips_through_nalgebra_unit_complex() {
    let r = Rot::from_radians(0.25).unwrap();
    let rot: nalgebra::UnitComplex<f32> = r.into();
    let r2 = Rot::try_from(&rot).unwrap();
    assert!((r2.angle() - r.angle()).abs() < 1.0e-6);
}

#[test]
fn transform_converts_to_nalgebra_isometry_translation_matches() {
    let t = Transform::from_pos_angle([3.0, 4.0], 0.0).unwrap();
    let i: nalgebra::Isometry2<f32> = (&t).into();
    let p = i.translation.vector;
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 4.0);
}

#[test]
fn world_types_round_trip_through_scalar_correct_nalgebra_representations() {
    let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
    let point: nalgebra::Point2<WorldScalar> = position.into();
    assert_eq!(Position::from(point), position);

    let transform = WorldTransform::new(position, Rot::from_radians(0.375).unwrap()).unwrap();
    let isometry: nalgebra::Isometry2<WorldScalar> = transform.into();
    let round_trip = WorldTransform::try_from(&isometry).unwrap();
    assert_eq!(round_trip.position(), position);
    assert!((round_trip.rotation().angle() - transform.rotation().angle()).abs() < 1.0e-6);

    #[cfg(feature = "double-precision")]
    assert_ne!(f64::from(TEST_WORLD_X as f32), TEST_WORLD_X);
}

#[test]
fn world_transform_try_from_nalgebra_rejects_non_finite_translation() {
    let isometry = nalgebra::Isometry2::<WorldScalar>::from_parts(
        nalgebra::Translation2::new(WorldScalar::NAN, WorldScalar::from(0.0_f32)),
        nalgebra::UnitComplex::identity(),
    );
    let error = WorldTransform::try_from(isometry).unwrap_err();
    assert_eq!(error, WorldTransformFromInteropError::NonFinite);
}
