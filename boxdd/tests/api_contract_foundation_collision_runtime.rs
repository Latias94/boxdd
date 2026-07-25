#[test]
fn standalone_manifold_runtime_paths_succeed() {
    let circle = boxdd::shapes::circle([0.0_f32, 0.0], 0.5);
    let capsule = boxdd::shapes::capsule([-0.75_f32, 0.0], [0.75_f32, 0.0], 0.35);
    let polygon = boxdd::shapes::box_polygon(1.0, 1.0);
    let segment = boxdd::shapes::segment([-1.5_f32, 0.0], [1.5_f32, 0.0]);
    let chain_segment = boxdd::shapes::chain_segment(
        [-2.0_f32, 0.0],
        [-1.0_f32, 0.0],
        [1.0_f32, 0.0],
        [2.0_f32, 0.0],
    );
    let x_overlap = boxdd::Transform::from_pos_angle([0.75_f32, 0.0], 0.0);
    let slight_up = boxdd::Transform::from_pos_angle([0.0_f32, 0.25], 0.0);
    let slight_down = boxdd::Transform::from_pos_angle([0.0_f32, -0.25], 0.0);
    let box_shift = boxdd::Transform::from_pos_angle([1.25_f32, 0.0], 0.0);
    let mut capsule_cache = boxdd::SimplexCache::new();
    let mut polygon_cache = boxdd::SimplexCache::new();

    let circles = boxdd::collide_circles(circle, circle, x_overlap);
    let capsule_circle = boxdd::collide_capsule_and_circle(capsule, circle, x_overlap);
    let segment_circle = boxdd::collide_segment_and_circle(segment, circle, slight_up);
    let polygon_circle = boxdd::collide_polygon_and_circle(polygon, circle, x_overlap);
    let capsules = boxdd::collide_capsules(capsule, capsule, x_overlap);
    let segment_capsule = boxdd::collide_segment_and_capsule(segment, capsule, slight_up);
    let polygon_capsule = boxdd::collide_polygon_and_capsule(polygon, capsule, box_shift);
    let polygons = boxdd::collide_polygons(polygon, polygon, box_shift);
    let segment_polygon =
        boxdd::collide_segment_and_polygon(segment, polygon, boxdd::Transform::IDENTITY);
    let chain_circle = boxdd::collide_chain_segment_and_circle(chain_segment, circle, slight_down);
    let chain_capsule = boxdd::collide_chain_segment_and_capsule(
        chain_segment,
        capsule,
        slight_down,
        Some(&mut capsule_cache),
    );
    let chain_polygon = boxdd::collide_chain_segment_and_polygon(
        chain_segment,
        polygon,
        slight_down,
        Some(&mut polygon_cache),
    );

    assert_eq!(circles.point_count(), 1);
    assert!(!capsule_circle.is_empty());
    assert!(!segment_circle.is_empty());
    assert!(!polygon_circle.is_empty());
    assert!(!capsules.is_empty());
    assert!(!segment_capsule.is_empty());
    assert!(!polygon_capsule.is_empty());
    assert!(!polygons.is_empty());
    assert!(!segment_polygon.is_empty());
    assert!(!chain_circle.is_empty());
    assert!(!chain_capsule.is_empty());
    assert!(!chain_polygon.is_empty());
    assert!(capsule_cache.count() <= 3);
    assert!(polygon_cache.count() <= 3);
}

#[test]
fn geometry_runtime_paths_succeed() {
    let circle = boxdd::Circle::new([0.0_f32, 0.0], 0.5);
    let capsule = boxdd::Capsule::new([-0.75_f32, 0.0], [0.75_f32, 0.0], 0.35);
    let segment = boxdd::Segment::new([-1.0_f32, 0.0], [1.0_f32, 0.0]);
    let polygon = boxdd::Polygon::square_polygon(1.0);
    let proxy_a = boxdd::ShapeProxy::new([[-1.0_f32, -1.0], [1.0_f32, 1.0]], 0.0)
        .expect("shape proxy A should be valid");
    let proxy_b =
        boxdd::ShapeProxy::new([[0.0_f32, 0.0]], 0.1).expect("shape proxy B should be valid");
    let cast_proxy =
        boxdd::ShapeProxy::new([[-2.0_f32, 0.0]], 0.1).expect("cast proxy should be valid");
    let world_transform = boxdd::WorldTransform::IDENTITY;
    let local_transform = boxdd::Transform::from_pos_angle([0.25_f32, 0.0], 0.0);
    let shape_cast_input = boxdd::ShapeCastInput::new(cast_proxy, [4.0_f32, 0.0]);
    let polygon_from_points = boxdd::Polygon::from_points(
        [
            [-1.0_f32, -1.0],
            [1.0_f32, -1.0],
            [1.0_f32, 1.0],
            [-1.0_f32, 1.0],
        ],
        0.0,
    );

    let circle_aabb = boxdd::Circle::aabb(circle, world_transform);
    let circle_mass = boxdd::Circle::mass_data(circle, 1.0);
    let circle_contains = boxdd::Circle::contains_point(circle, [0.0_f32, 0.0]);
    let circle_ray = boxdd::Circle::ray_cast(circle, [-2.0_f32, 0.0], [4.0_f32, 0.0]);
    let circle_cast = boxdd::Circle::shape_cast(circle, shape_cast_input);

    let capsule_aabb = boxdd::Capsule::aabb(capsule, world_transform);
    let capsule_mass = boxdd::Capsule::mass_data(capsule, 1.0);
    let capsule_contains = boxdd::Capsule::contains_point(capsule, [0.0_f32, 0.0]);
    let capsule_ray = boxdd::Capsule::ray_cast(capsule, [-2.0_f32, 0.0], [4.0_f32, 0.0]);
    let capsule_cast = boxdd::Capsule::shape_cast(capsule, shape_cast_input);

    let polygon_aabb = boxdd::Polygon::aabb(polygon, world_transform);
    let polygon_mass = boxdd::Polygon::mass_data(polygon, 1.0);
    let polygon_contains = boxdd::Polygon::contains_point(polygon, [0.0_f32, 0.0]);
    let polygon_ray = boxdd::Polygon::ray_cast(polygon, [-2.0_f32, 0.0], [4.0_f32, 0.0]);
    let polygon_cast = boxdd::Polygon::shape_cast(polygon, shape_cast_input);
    let transformed_polygon = boxdd::Polygon::transformed(polygon, local_transform);

    let segment_aabb = boxdd::Segment::aabb(segment, world_transform);
    let segment_ray = boxdd::Segment::ray_cast(segment, [0.0_f32, 1.0], [0.0_f32, -2.0], false);
    let segment_cast = boxdd::Segment::shape_cast(segment, shape_cast_input);

    let mut cache = boxdd::SimplexCache::new();
    let distance_input = boxdd::DistanceInput::new(proxy_a, proxy_b, boxdd::Transform::IDENTITY);
    let distance = boxdd::shape_distance(distance_input, &mut cache);
    let pair_cast_input = boxdd::ShapeCastPairInput::new(
        proxy_a,
        proxy_b,
        boxdd::Transform::from_pos_angle([2.0_f32, 0.0], 0.0),
        [-2.0_f32, 0.0],
    );
    let pair_cast = boxdd::shape_cast(pair_cast_input);
    let sweep_a = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        boxdd::Rot::IDENTITY,
        boxdd::Rot::IDENTITY,
    );
    let sweep_b = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [2.0_f32, 0.0],
        [0.0_f32, 0.0],
        boxdd::Rot::IDENTITY,
        boxdd::Rot::IDENTITY,
    );
    let toi_input = boxdd::ToiInput::new(proxy_a, proxy_b, sweep_a, sweep_b);
    let toi = boxdd::time_of_impact(toi_input);
    let plane = boxdd::Plane::new([0.0_f32, 1.0], 0.0);
    let mut planes = [boxdd::CollisionPlane::rigid(plane)];
    let solved = boxdd::solve_planes([1.0_f32, -1.0], &mut planes);

    assert!(polygon_from_points.is_some());
    assert!(circle_aabb.is_valid());
    assert!(capsule_aabb.is_valid());
    assert!(polygon_aabb.is_valid());
    assert!(segment_aabb.is_valid());
    assert!(circle_mass.mass > 0.0);
    assert!(capsule_mass.mass > 0.0);
    assert!(polygon_mass.mass > 0.0);
    assert!(circle_contains);
    assert!(capsule_contains);
    assert!(polygon_contains);
    assert!(circle_ray.hit);
    assert!(capsule_ray.hit);
    assert!(polygon_ray.hit);
    assert!(segment_ray.hit);
    assert!(circle_cast.hit);
    assert!(capsule_cast.hit);
    assert!(polygon_cast.hit);
    assert!(segment_cast.hit);
    assert_eq!(transformed_polygon.count(), polygon.count());
    assert!(distance.distance.is_finite());
    assert!(pair_cast.fraction.is_finite());
    assert!(toi.fraction.is_finite());
    assert!(solved.translation.is_valid());
}

#[test]
fn foundation_runtime_paths_succeed() {
    let start_ticks = boxdd::ticks();
    let version = boxdd::version();
    let allocated_bytes = boxdd::allocated_byte_count();
    let elapsed = boxdd::milliseconds_since(start_ticks);
    let mut reset_ticks = start_ticks;
    let elapsed_and_reset = boxdd::milliseconds_and_reset(&mut reset_ticks);
    let hash = boxdd::hash_bytes(boxdd::HASH_INIT, b"boxdd-api-contract");
    boxdd::yield_now();

    assert!(version.major > 0);
    assert!(allocated_bytes >= 0);
    assert!(elapsed >= 0.0);
    assert!(elapsed_and_reset >= 0.0);
    assert!(reset_ticks >= start_ticks);
    assert_ne!(hash, boxdd::HASH_INIT);
}
