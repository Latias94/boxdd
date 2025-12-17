#![cfg(feature = "bytemuck")]

use boxdd::{Aabb, Rot, Transform, Vec2};

#[test]
fn bytemuck_bytes_roundtrip_for_core_types() {
    let v = Vec2::new(1.0, 2.0);
    let v2 = *bytemuck::from_bytes::<Vec2>(bytemuck::bytes_of(&v));
    assert_eq!(bytemuck::bytes_of(&v), bytemuck::bytes_of(&v2));

    let r = Rot::from_radians(1.25);
    let r2 = *bytemuck::from_bytes::<Rot>(bytemuck::bytes_of(&r));
    assert_eq!(bytemuck::bytes_of(&r), bytemuck::bytes_of(&r2));

    let t = Transform::from_pos_angle(Vec2::new(3.0, 4.0), 0.5);
    let t2 = *bytemuck::from_bytes::<Transform>(bytemuck::bytes_of(&t));
    assert_eq!(bytemuck::bytes_of(&t), bytemuck::bytes_of(&t2));

    let a = Aabb::from_center_half_extents([0.0, 1.0], [2.0, 3.0]);
    let a2 = *bytemuck::from_bytes::<Aabb>(bytemuck::bytes_of(&a));
    assert_eq!(bytemuck::bytes_of(&a), bytemuck::bytes_of(&a2));
}
