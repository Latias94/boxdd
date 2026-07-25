#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{
        Aabb, MoverPlaneResult, Position, QueryFilter, RayResult, ShapeId, Vec2, World, WorldDef,
    };

    let world = World::new(WorldDef::default()).unwrap();
    let filter = QueryFilter::default();
    let unit_x = Vec2::new(1.0, 0.0);
    let unit_y = Vec2::new(0.0, 1.0);
    let points = [Vec2::ZERO];
    let aabb = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0));
    let mut shapes = Vec::<ShapeId>::new();
    let mut rays = Vec::<RayResult>::new();
    let mut planes = Vec::<MoverPlaneResult>::new();

    let _ = world.overlap_aabb(Position::ZERO, aabb, filter);
    world.overlap_aabb_into(Position::ZERO, aabb, filter, &mut shapes);
    let _ = world.visit_overlap_aabb(Position::ZERO, aabb, filter, |_| true);
    let _ = world.try_overlap_aabb(Position::ZERO, aabb, filter);
    let _ = world.try_overlap_aabb_into(Position::ZERO, aabb, filter, &mut shapes);
    let _ = world.try_visit_overlap_aabb(Position::ZERO, aabb, filter, |_| true);

    let _ = world.overlap_polygon_points(Position::ZERO, points, 0.0, filter);
    world.overlap_polygon_points_into(Position::ZERO, points, 0.0, filter, &mut shapes);
    let _ = world.visit_overlap_polygon_points(Position::ZERO, points, 0.0, filter, |_| true);
    let _ = world.try_overlap_polygon_points(Position::ZERO, points, 0.0, filter);
    let _ = world.try_overlap_polygon_points_into(Position::ZERO, points, 0.0, filter, &mut shapes);
    let _ = world.try_visit_overlap_polygon_points(Position::ZERO, points, 0.0, filter, |_| true);

    let _ = world.overlap_polygon_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
    );
    world.overlap_polygon_points_with_offset_into(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
        &mut shapes,
    );
    let _ = world.visit_overlap_polygon_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
        |_| true,
    );
    let _ = world.try_overlap_polygon_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
    );
    let _ = world.try_overlap_polygon_points_with_offset_into(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
        &mut shapes,
    );
    let _ = world.try_visit_overlap_polygon_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        filter,
        |_| true,
    );

    let _ = world.cast_ray_all(Position::ZERO, unit_x, filter);
    world.cast_ray_all_into(Position::ZERO, unit_x, filter, &mut rays);
    let _ = world.try_cast_ray_all(Position::ZERO, unit_x, filter);
    let _ = world.try_cast_ray_all_into(Position::ZERO, unit_x, filter, &mut rays);

    let _ = world.cast_shape_points(Position::ZERO, points, 0.0, unit_x, filter);
    world.cast_shape_points_into(Position::ZERO, points, 0.0, unit_x, filter, &mut rays);
    let _ = world.try_cast_shape_points(Position::ZERO, points, 0.0, unit_x, filter);
    let _ =
        world.try_cast_shape_points_into(Position::ZERO, points, 0.0, unit_x, filter, &mut rays);
    let _ = world.cast_shape_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        unit_x,
        filter,
    );
    world.cast_shape_points_with_offset_into(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        unit_x,
        filter,
        &mut rays,
    );
    let _ = world.try_cast_shape_points_with_offset(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        unit_x,
        filter,
    );
    let _ = world.try_cast_shape_points_with_offset_into(
        Position::ZERO,
        points,
        0.0,
        Vec2::ZERO,
        0.0,
        unit_x,
        filter,
        &mut rays,
    );

    let _ = world.collide_mover(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter);
    world.collide_mover_into(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter, &mut planes);
    let _ = world.try_collide_mover(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter);
    let _ =
        world.try_collide_mover_into(Position::ZERO, Vec2::ZERO, unit_y, 0.5, filter, &mut planes);
}
