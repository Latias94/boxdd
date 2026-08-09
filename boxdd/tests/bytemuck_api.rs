#![cfg(feature = "bytemuck")]

use boxdd::{Aabb, Position, Rot, Transform, Vec2, WorldScalar, WorldTransform};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(Aabb: bytemuck::Pod);
assert_not_impl_any!(Rot: bytemuck::Pod);
assert_not_impl_any!(Transform: bytemuck::Pod);
assert_not_impl_any!(WorldTransform: bytemuck::Pod);

#[cfg(not(feature = "double-precision"))]
const TEST_WORLD_X: WorldScalar = 10_000.125;
#[cfg(feature = "double-precision")]
const TEST_WORLD_X: WorldScalar = 10_000_000.001;

#[test]
fn bytemuck_bytes_roundtrip_for_plain_vector_type() {
    let v = Vec2::new(1.0, 2.0);
    let v2 = *bytemuck::from_bytes::<Vec2>(bytemuck::bytes_of(&v));
    assert_eq!(bytemuck::bytes_of(&v), bytemuck::bytes_of(&v2));
}

#[test]
fn bytemuck_bytes_roundtrip_for_world_precision_types() {
    let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
    let position_round_trip = *bytemuck::from_bytes::<Position>(bytemuck::bytes_of(&position));
    assert_eq!(position_round_trip, position);

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
