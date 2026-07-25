#![cfg(feature = "bytemuck")]

use boxdd::{Aabb, Position, Rot, Transform, Vec2, WorldScalar, WorldTransform};

#[cfg(not(feature = "double-precision"))]
const TEST_WORLD_X: WorldScalar = 10_000.125;
#[cfg(feature = "double-precision")]
const TEST_WORLD_X: WorldScalar = 10_000_000.001;

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

#[test]
fn bytemuck_bytes_roundtrip_for_world_precision_types() {
    let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
    let position_round_trip = *bytemuck::from_bytes::<Position>(bytemuck::bytes_of(&position));
    assert_eq!(position_round_trip, position);

    let transform = WorldTransform::new(position, Rot::from_radians(0.375));
    let transform_round_trip =
        *bytemuck::from_bytes::<WorldTransform>(bytemuck::bytes_of(&transform));
    assert_eq!(transform_round_trip.position(), position);
    assert_eq!(
        bytemuck::bytes_of(&transform_round_trip),
        bytemuck::bytes_of(&transform)
    );

    assert_eq!(
        core::mem::size_of::<Position>(),
        2 * core::mem::size_of::<WorldScalar>()
    );
    assert_eq!(
        core::mem::size_of::<WorldTransform>(),
        core::mem::size_of::<Position>() + core::mem::size_of::<Rot>()
    );

    #[cfg(feature = "double-precision")]
    assert_ne!(
        f64::from(position_round_trip.x as f32),
        position_round_trip.x
    );
}
