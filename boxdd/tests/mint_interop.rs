#![cfg(feature = "mint")]

use boxdd::{Aabb, Rot, Transform, TransformFromMintError, Vec2};

#[test]
fn vec2_converts_to_and_from_mint() {
    let v = Vec2::new(1.0, 2.0);
    let mv: mint::Vector2<f32> = v.into();
    assert_eq!(mv.x, 1.0);
    assert_eq!(mv.y, 2.0);

    let v2: Vec2 = mv.into();
    assert_eq!(v2, v);

    let mp: mint::Point2<f32> = v.into();
    let v3: Vec2 = mp.into();
    assert_eq!(v3, v);
}

#[test]
fn rot_converts_to_mint_matrices() {
    let r = Rot::from_radians(0.25);

    let row: mint::RowMatrix2<f32> = r.into();
    assert!((row.x.x - r.angle().cos()).abs() < 1.0e-6);
    assert!((row.x.y + r.angle().sin()).abs() < 1.0e-6);
    assert!((row.y.x - r.angle().sin()).abs() < 1.0e-6);
    assert!((row.y.y - r.angle().cos()).abs() < 1.0e-6);

    let col: mint::ColumnMatrix2<f32> = r.into();
    assert!((col.x.x - r.angle().cos()).abs() < 1.0e-6);
    assert!((col.x.y - r.angle().sin()).abs() < 1.0e-6);
    assert!((col.y.x + r.angle().sin()).abs() < 1.0e-6);
    assert!((col.y.y - r.angle().cos()).abs() < 1.0e-6);
}

#[test]
fn aabb_converts_to_and_from_mint_tuples() {
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]);

    let (lp, up): (mint::Point2<f32>, mint::Point2<f32>) = a.into();
    assert_eq!(lp.x, 1.0);
    assert_eq!(lp.y, 2.0);
    assert_eq!(up.x, 3.0);
    assert_eq!(up.y, 4.0);

    let a2 = Aabb::from((lp, up));
    assert_eq!(a2.lower, Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper, Vec2::new(3.0, 4.0));

    let (lv, uv): (mint::Vector2<f32>, mint::Vector2<f32>) = a.into();
    let a3 = Aabb::from((lv, uv));
    assert_eq!(a3, a2);
}

#[test]
fn transform_converts_to_mint_row_matrix3x2_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0);
    let m: mint::RowMatrix3x2<f32> = t.into();
    assert_eq!(m.z.x, 3.0);
    assert_eq!(m.z.y, 4.0);

    let t2 = Transform::try_from(m).unwrap();
    assert_eq!(t2.position(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_converts_to_mint_column_matrix3x2_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0);
    let m: mint::ColumnMatrix3x2<f32> = t.into();
    assert_eq!(m.x.z, 3.0);
    assert_eq!(m.y.z, 4.0);

    let t2 = Transform::try_from(m).unwrap();
    assert_eq!(t2.position(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_converts_to_mint_row_matrix2x3_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0);
    let m: mint::RowMatrix2x3<f32> = t.into();
    assert_eq!(m.x.z, 3.0);
    assert_eq!(m.y.z, 4.0);

    let t2 = Transform::try_from(m).unwrap();
    assert_eq!(t2.position(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_converts_to_mint_column_matrix2x3_translation_matches() {
    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.0);
    let m: mint::ColumnMatrix2x3<f32> = t.into();
    assert_eq!(m.z.x, 3.0);
    assert_eq!(m.z.y, 4.0);

    let t2 = Transform::try_from(m).unwrap();
    assert_eq!(t2.position(), Vec2::new(3.0, 4.0));
}

#[test]
fn transform_try_from_mint_rejects_scaled() {
    let m = mint::RowMatrix3x2::<f32> {
        x: mint::Vector2 { x: 2.0, y: 0.0 },
        y: mint::Vector2 { x: 0.0, y: 2.0 },
        z: mint::Vector2 { x: 0.0, y: 0.0 },
    };
    let err = Transform::try_from(m).unwrap_err();
    assert_eq!(err, TransformFromMintError::NotPureRotation);
}

#[test]
fn transform_try_from_mint_column_matrix_rejects_scaled() {
    let m = mint::ColumnMatrix2x3::<f32> {
        x: mint::Vector2 { x: 2.0, y: 0.0 },
        y: mint::Vector2 { x: 0.0, y: 2.0 },
        z: mint::Vector2 { x: 0.0, y: 0.0 },
    };
    let err = Transform::try_from(m).unwrap_err();
    assert_eq!(err, TransformFromMintError::NotPureRotation);
}
