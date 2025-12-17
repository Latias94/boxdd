#![cfg(feature = "cgmath")]

use boxdd::{Aabb, Vec2};

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
