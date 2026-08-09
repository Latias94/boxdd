use boxdd::{
    Aabb, DistanceInput, Position, Rot, ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep,
    ToiInput, Transform, WorldTransform, collide_polygon_and_circle, segment_distance, shape_cast,
    shape_distance, shapes, time_of_impact,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let _foundation = boxdd::Foundation::initialize_default()?;
    let proxy_a = ShapeProxy::new(
        [[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]],
        0.0,
    )?;
    let proxy_b = ShapeProxy::new(
        [[-0.5_f32, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
        0.0,
    )?;

    let segment = segment_distance([-1.0_f32, 0.0], [1.0, 0.0], [0.5, -1.0], [0.5, 1.0])?;

    let mut cache = SimplexCache::default();
    let distance = shape_distance(
        DistanceInput::new(
            proxy_a,
            proxy_b,
            Transform::from_pos_angle([2.2_f32, 0.0], 0.0)?,
        )?,
        &mut cache,
    )?;

    let cast = shape_cast(ShapeCastPairInput::new(
        proxy_a,
        proxy_b,
        Transform::from_pos_angle([2.8_f32, 0.0], 0.0)?,
        [-2.2_f32, 0.0],
    )?)?;

    let toi = time_of_impact(ToiInput::new(
        proxy_a,
        proxy_b,
        Sweep::new(
            [0.0_f32, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        )?,
        Sweep::new(
            [0.0_f32, 0.0],
            [2.8, 0.0],
            [0.2, 0.0],
            Rot::IDENTITY,
            Rot::IDENTITY,
        )?,
    )?)?;

    let manifold = collide_polygon_and_circle(
        shapes::box_polygon(1.0, 0.5)?,
        shapes::circle([0.7_f32, 0.1], 0.35)?,
        Transform::IDENTITY,
    )?;
    // Standalone collision results are in shape A's local frame. Choose an absolute pose only at
    // the world/presentation boundary.
    let shape_a_world =
        WorldTransform::from_pos_angle(Position::new(1_000_000.0, -2_000_000.0), 0.25)?;
    let first_world_point = manifold
        .points()
        .first()
        .map(|point| shape_a_world.transform_point(point.point));

    let aabb_hit = Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0])?
        .ray_cast([-2.0_f32, 0.2], [4.0, 0.0])?;

    println!("segment_distance squared: {:.3}", segment.distance_squared);
    println!(
        "shape_distance: distance={:.3} cache_points={}",
        distance.distance,
        cache.count()
    );
    println!("shape_cast: hit={} fraction={:.3}", cast.hit, cast.fraction);
    println!(
        "time_of_impact: state={:?} fraction={:.3}",
        toi.state, toi.fraction
    );
    println!(
        "collide_polygon_and_circle: local_contacts={} local_normal=({:.3}, {:.3}) world_point={:?}",
        manifold.points().len(),
        manifold.normal.x,
        manifold.normal.y,
        first_world_point
    );
    println!(
        "aabb.ray_cast: hit={} fraction={:.3}",
        aabb_hit.hit, aabb_hit.fraction
    );

    Ok(())
}
