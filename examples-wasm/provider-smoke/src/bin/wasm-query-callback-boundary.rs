#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Aabb, Foundation, Position, QueryFilter, ShapeProxy, Vec2};

    let foundation = Foundation::initialize_default().unwrap();
    let world = foundation.create_world(foundation.world_def()).unwrap();
    let query = world.query().unwrap();
    let filter = QueryFilter::default();
    let unit_x = Vec2::new(1.0, 0.0);
    let unit_y = Vec2::new(0.0, 1.0);
    let points = [Vec2::ZERO];
    let proxy = ShapeProxy::new(points, 0.0).unwrap();
    let aabb = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)).unwrap();
    let mut shapes = ();
    let mut rays = ();
    let mut planes = ();

    let _ = query.overlap_aabb(Position::ZERO, aabb, filter);
    let _ = query.overlap_aabb_into(Position::ZERO, aabb, filter, &mut shapes);
    let _ = query.visit_overlap_aabb(Position::ZERO, aabb, filter, |_| true);
    let _ =
        query.visit_overlap_aabb_with_buffer(Position::ZERO, aabb, filter, &mut shapes, |_| true);

    let _ = query.overlap_shape(Position::ZERO, proxy, filter);
    let _ = query.overlap_shape_into(Position::ZERO, proxy, filter, &mut shapes);
    let _ = query.visit_overlap_shape(Position::ZERO, proxy, filter, |_| true);
    let _ =
        query.visit_overlap_shape_with_buffer(Position::ZERO, proxy, filter, &mut shapes, |_| true);

    let _ = query.cast_ray_all(Position::ZERO, unit_x, filter);
    let _ = query.cast_ray_all_into(Position::ZERO, unit_x, filter, &mut rays);
    let _ = query.cast_shape(Position::ZERO, proxy, unit_x, filter);
    let _ = query.cast_shape_into(Position::ZERO, proxy, unit_x, filter, &mut rays);

    let _ = query.collide_mover(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter);
    let _ = query.collide_mover_into(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter, &mut planes);
}
