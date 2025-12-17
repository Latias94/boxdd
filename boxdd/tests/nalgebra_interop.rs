#![cfg(feature = "nalgebra")]

use boxdd::{Aabb, Transform, Vec2};

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
    let a = Aabb::new([1.0, 2.0], [3.0, 4.0]);

    let (lp, up): (nalgebra::Point2<f32>, nalgebra::Point2<f32>) = a.into();
    assert_eq!(lp.x, 1.0);
    assert_eq!(lp.y, 2.0);
    assert_eq!(up.x, 3.0);
    assert_eq!(up.y, 4.0);

    let a2 = Aabb::from((lp, up));
    assert_eq!(a2.lower, Vec2::new(1.0, 2.0));
    assert_eq!(a2.upper, Vec2::new(3.0, 4.0));

    let (lv, uv): (nalgebra::Vector2<f32>, nalgebra::Vector2<f32>) = a.into();
    let a3 = Aabb::from((lv, uv));
    assert_eq!(a3, a2);
}

#[test]
fn transform_converts_to_nalgebra_isometry_translation_matches() {
    let t = Transform::from_pos_angle([3.0, 4.0], 0.0);
    let i: nalgebra::Isometry2<f32> = (&t).into();
    let p = i.translation.vector;
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 4.0);
}
