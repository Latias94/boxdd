fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn geometry_runtime_paths_succeed() {
    initialize_foundation();

    let circle = boxdd::Circle::new([0.0_f32, 0.0], 0.5).unwrap();
    let capsule = boxdd::Capsule::new([-0.75_f32, 0.0], [0.75_f32, 0.0], 0.35).unwrap();
    let segment = boxdd::Segment::new([-1.0_f32, 0.0], [1.0_f32, 0.0]).unwrap();
    let polygon = boxdd::Polygon::square_polygon(1.0).unwrap();
    let proxy_a = boxdd::ShapeProxy::new([[-1.0_f32, -1.0], [1.0_f32, 1.0]], 0.0)
        .expect("shape proxy A should be valid");
    let proxy_b =
        boxdd::ShapeProxy::new([[0.0_f32, 0.0]], 0.1).expect("shape proxy B should be valid");
    let cast_proxy =
        boxdd::ShapeProxy::new([[-2.0_f32, 0.0]], 0.1).expect("cast proxy should be valid");
    let world_transform = boxdd::WorldTransform::IDENTITY;
    let local_transform = boxdd::Transform::from_pos_angle([0.25_f32, 0.0], 0.0).unwrap();
    let offset_proxy = boxdd::ShapeProxy::offset_from_points(
        [[-0.5_f32, 0.0], [0.5_f32, 0.0]],
        0.1,
        local_transform,
    )
    .expect("offset shape proxy should be valid");
    let shape_cast_input = boxdd::ShapeCastInput::new(cast_proxy, [4.0_f32, 0.0]).unwrap();
    let polygon_from_points = boxdd::Polygon::from_points(
        [
            [-1.0_f32, -1.0],
            [1.0_f32, -1.0],
            [1.0_f32, 1.0],
            [-1.0_f32, 1.0],
        ],
        0.0,
    )
    .unwrap();

    let circle_aabb = boxdd::Circle::aabb(circle, world_transform).unwrap();
    let circle_mass = boxdd::Circle::mass_data(circle, 1.0).unwrap();
    let circle_contains = boxdd::Circle::contains_point(circle, [0.0_f32, 0.0]).unwrap();
    let circle_ray = boxdd::Circle::ray_cast(circle, [-2.0_f32, 0.0], [4.0_f32, 0.0]).unwrap();
    let circle_cast = boxdd::Circle::shape_cast(circle, shape_cast_input).unwrap();

    let capsule_aabb = boxdd::Capsule::aabb(capsule, world_transform).unwrap();
    let capsule_mass = boxdd::Capsule::mass_data(capsule, 1.0).unwrap();
    let capsule_contains = boxdd::Capsule::contains_point(capsule, [0.0_f32, 0.0]).unwrap();
    let capsule_ray = boxdd::Capsule::ray_cast(capsule, [-2.0_f32, 0.0], [4.0_f32, 0.0]).unwrap();
    let capsule_cast = boxdd::Capsule::shape_cast(capsule, shape_cast_input).unwrap();

    let polygon_aabb = boxdd::Polygon::aabb(polygon, world_transform).unwrap();
    let polygon_mass = boxdd::Polygon::mass_data(polygon, 1.0).unwrap();
    let polygon_contains = boxdd::Polygon::contains_point(polygon, [0.0_f32, 0.0]).unwrap();
    let polygon_ray = boxdd::Polygon::ray_cast(polygon, [-2.0_f32, 0.0], [4.0_f32, 0.0]).unwrap();
    let polygon_cast = boxdd::Polygon::shape_cast(polygon, shape_cast_input).unwrap();
    let transformed_polygon = boxdd::Polygon::transformed(polygon, local_transform).unwrap();

    let segment_aabb = boxdd::Segment::aabb(segment, world_transform).unwrap();
    let segment_ray =
        boxdd::Segment::ray_cast(segment, [0.0_f32, 1.0], [0.0_f32, -2.0], false).unwrap();
    let segment_cast = boxdd::Segment::shape_cast(segment, shape_cast_input).unwrap();

    let mut cache = boxdd::SimplexCache::new();
    let distance_input =
        boxdd::DistanceInput::new(proxy_a, proxy_b, boxdd::Transform::IDENTITY).unwrap();
    let distance = boxdd::shape_distance(distance_input, &mut cache).unwrap();
    let pair_cast_input = boxdd::ShapeCastPairInput::new(
        proxy_a,
        proxy_b,
        boxdd::Transform::from_pos_angle([2.0_f32, 0.0], 0.0).unwrap(),
        [-2.0_f32, 0.0],
    )
    .unwrap();
    let pair_cast = boxdd::shape_cast(pair_cast_input).unwrap();
    let sweep_a = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        boxdd::Rot::IDENTITY,
        boxdd::Rot::IDENTITY,
    )
    .unwrap();
    let sweep_b = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [2.0_f32, 0.0],
        [0.0_f32, 0.0],
        boxdd::Rot::IDENTITY,
        boxdd::Rot::IDENTITY,
    )
    .unwrap();
    let toi_input = boxdd::ToiInput::new(proxy_a, proxy_b, sweep_a, sweep_b).unwrap();
    let toi = boxdd::time_of_impact(toi_input).unwrap();
    let plane = boxdd::Plane::new([0.0_f32, 1.0], 0.0).unwrap();
    let mut planes = [boxdd::CollisionPlane::rigid(plane).unwrap()];
    let solved = boxdd::solve_planes([1.0_f32, -1.0], &mut planes).unwrap();

    assert_eq!(polygon_from_points.count(), 4);
    assert_eq!(offset_proxy.count(), 2);
    assert_eq!(offset_proxy.radius(), 0.1);
    assert_eq!(offset_proxy.points()[0], boxdd::Vec2::new(-0.25, 0.0));
    assert_eq!(offset_proxy.points()[1], boxdd::Vec2::new(0.75, 0.0));
    assert!(circle_aabb.is_valid());
    assert!(capsule_aabb.is_valid());
    assert!(polygon_aabb.is_valid());
    assert!(segment_aabb.is_valid());
    assert!(circle_mass.mass() > 0.0);
    assert!(capsule_mass.mass() > 0.0);
    assert!(polygon_mass.mass() > 0.0);
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
    assert!(solved.translation().is_valid());
}

#[test]
fn foundation_runtime_paths_succeed() {
    initialize_foundation();

    let start_ticks = boxdd::ticks().unwrap();
    let version = boxdd::version().unwrap();
    let allocated_bytes = boxdd::allocated_byte_count().unwrap();
    let elapsed = boxdd::milliseconds_since(start_ticks).unwrap();
    let mut reset_ticks = start_ticks;
    let elapsed_and_reset = boxdd::milliseconds_and_reset(&mut reset_ticks).unwrap();
    let hash = boxdd::hash_bytes(boxdd::HASH_INIT, b"boxdd-api-contract").unwrap();
    boxdd::yield_now().unwrap();

    assert!(version.major > 0);
    assert!(allocated_bytes >= 0);
    assert!(elapsed >= 0.0);
    assert!(elapsed_and_reset >= 0.0);
    assert!(reset_ticks >= start_ticks);
    assert_ne!(hash, boxdd::HASH_INIT);
}
