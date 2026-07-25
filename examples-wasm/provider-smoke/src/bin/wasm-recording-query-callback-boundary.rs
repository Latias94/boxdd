#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Aabb, Position, QueryFilter, RecordingCapacity, Vec2, World, WorldDef};

    let mut world = World::new(WorldDef::default()).unwrap();
    let session = world.start_recording(RecordingCapacity::default());
    let filter = QueryFilter::default();
    let unit_x = Vec2::new(1.0, 0.0);
    let unit_y = Vec2::new(0.0, 1.0);
    let aabb = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0));
    let _ = session.overlap_aabb(Position::ZERO, aabb, filter);
    let _ = session.try_overlap_aabb(Position::ZERO, aabb, filter);
    let _ = session.overlap_polygon_points(Position::ZERO, [Vec2::ZERO], 0.0, filter);
    let _ = session.try_overlap_polygon_points(Position::ZERO, [Vec2::ZERO], 0.0, filter);
    let _ = session.cast_ray_all(Position::ZERO, unit_x, filter);
    let _ = session.try_cast_ray_all(Position::ZERO, unit_x, filter);
    let _ = session.cast_shape_points(Position::ZERO, [Vec2::ZERO], 0.0, unit_x, filter);
    let _ = session.try_cast_shape_points(Position::ZERO, [Vec2::ZERO], 0.0, unit_x, filter);
    let _ = session.collide_mover(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter);
    let _ = session.try_collide_mover(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter);
}
