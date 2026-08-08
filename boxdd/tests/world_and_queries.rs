use boxdd::{prelude::*, shapes};

fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

fn create_circle(world: &mut World, position: Position, radius: f32) -> ShapeId {
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .position(position)
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle(Vec2::ZERO, radius).unwrap(),
        )
        .unwrap()
}

fn create_box(world: &mut World, position: Position, half_width: f32, half_height: f32) -> ShapeId {
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .position(position)
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_polygon(
            &ShapeDef::default(),
            &shapes::box_polygon(half_width, half_height).unwrap(),
        )
        .unwrap()
}

#[cfg(feature = "double-precision")]
#[test]
fn large_world_paths_preserve_a_millimeter_separation() {
    const METERS: f64 = 10_000_000.0;
    const MILLIMETER: f64 = 0.001;
    const EXPECTED_ADVANCE: f64 = 0.125;

    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(
            foundation
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let initial_a = Position::new(METERS, -METERS);
    let initial_b = Position::new(METERS + MILLIMETER, -METERS);

    let mut create_body = |position| {
        let body = world
            .create_body(
                foundation
                    .body_builder()
                    .body_type(BodyType::Kinematic)
                    .position(position)
                    .linear_velocity([0.25_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape = world
            .body(body)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.0002)
            .unwrap();
        (body, shape)
    };
    let (body_a, shape_a) = create_body(initial_a);
    let (body_b, shape_b) = create_body(initial_b);

    let completed = world.step(0.5, 1).unwrap();
    let body_events = completed.body_events().unwrap().to_owned().unwrap();
    drop(completed);

    let position_a = world.body(body_a).unwrap().position().unwrap();
    let position_b = world.body(body_b).unwrap().position().unwrap();
    let expected_delta = initial_b.x - initial_a.x;
    assert!((expected_delta - MILLIMETER).abs() < 5.0e-10);
    assert_eq!(position_a.x - initial_a.x, EXPECTED_ADVANCE);
    assert_eq!(position_b.x - initial_b.x, EXPECTED_ADVANCE);
    assert_eq!(position_b.x - position_a.x, expected_delta);
    assert_eq!(position_a.y, initial_a.y);
    assert_eq!(position_b.y, initial_b.y);

    let event_position = |body| {
        body_events
            .iter()
            .find(|event| event.body_id == body)
            .expect("each moving kinematic body must publish an event")
            .transform
            .position()
    };
    let event_position_a = event_position(body_a);
    let event_position_b = event_position(body_b);
    assert_eq!(event_position_a, position_a);
    assert_eq!(event_position_b, position_b);
    assert_eq!(event_position_b.x - event_position_a.x, expected_delta);

    let local_delta = position_b.checked_relative_to(position_a).unwrap();
    assert!((local_delta.x - MILLIMETER as f32).abs() <= f32::EPSILON);
    assert_eq!(local_delta.y, 0.0);

    {
        let query = world.query().unwrap();
        let ray_hit = |position: Position| {
            query
                .cast_ray_closest(
                    position.offset(Vec2::new(0.0, 0.01)),
                    [0.0_f32, -0.02],
                    QueryFilter::default(),
                )
                .unwrap()
                .expect("each narrow ray must hit its corresponding sub-millimeter circle")
        };
        let hit_a = ray_hit(position_a);
        let hit_b = ray_hit(position_b);
        assert_eq!(hit_a.shape_id, shape_a);
        assert_eq!(hit_b.shape_id, shape_b);
        assert_eq!(hit_b.point.x - hit_a.point.x, expected_delta);
        assert_eq!(hit_a.point.checked_relative_to(position_a).unwrap().x, 0.0);
        assert_eq!(hit_b.point.checked_relative_to(position_b).unwrap().x, 0.0);
    }

    let shape = world.shape(shape_a).unwrap();
    assert!(shape.test_point(position_a).unwrap());
    assert!(
        !shape
            .test_point(position_a.offset(Vec2::new(0.0003, 0.0)))
            .unwrap()
    );

    let shape_ray = shape
        .ray_cast(position_a.offset(Vec2::new(-0.01, 0.0)), [0.02_f32, 0.0])
        .unwrap();
    assert!(shape_ray.hit);
    let local_shape_hit = shape_ray.point.checked_relative_to(position_a).unwrap();
    assert!(approx_eq(local_shape_hit.x, -0.0002, 1.0e-6));
    assert!(approx_eq(local_shape_hit.y, 0.0, 1.0e-6));

    let mut drawn_positions = world
        .debug_draw_collect(DebugDrawOptions::default())
        .unwrap()
        .into_iter()
        .filter_map(|command| match command {
            DebugDrawCmd::SolidCircle { transform, .. } => Some(transform.position()),
            _ => None,
        })
        .collect::<Vec<_>>();
    drawn_positions.sort_by(|left, right| left.x.total_cmp(&right.x));
    assert_eq!(drawn_positions.len(), 2);
    assert_eq!(drawn_positions[0], position_a);
    assert_eq!(drawn_positions[1], position_b);
    assert_eq!(drawn_positions[1].x - drawn_positions[0].x, expected_delta);
}

#[test]
fn closest_ray_stats_preserve_hit_and_miss_traversal_results() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let triangle =
        shapes::polygon_from_points([[0.0_f32, 0.0], [2.0, 0.0], [0.0, 2.0]], 0.0).unwrap();
    let shape = world
        .body(body)
        .unwrap()
        .create_polygon(&ShapeDef::default(), &triangle)
        .unwrap();
    let query = world.query().unwrap();

    let hit = query
        .cast_ray_closest_with_stats(
            Position::new(-1.0, 0.25),
            [4.0_f32, 0.0],
            QueryFilter::default(),
        )
        .unwrap();
    assert_eq!(hit.hit.map(|result| result.shape_id), Some(shape));
    assert!(hit.node_visits > 0);
    assert!(hit.leaf_visits > 0);
    assert_eq!(
        query
            .cast_ray_closest(
                Position::new(-1.0, 0.25),
                [4.0_f32, 0.0],
                QueryFilter::default(),
            )
            .unwrap()
            .map(|result| result.shape_id),
        Some(shape)
    );

    let miss = query
        .cast_ray_closest_with_stats(
            Position::new(1.7, 1.7),
            [0.2_f32, 0.0],
            QueryFilter::default(),
        )
        .unwrap();
    assert!(miss.hit.is_none());
    assert!(miss.node_visits > 0);
    assert!(miss.leaf_visits > 0);
}

#[test]
fn queries_preserve_local_geometry_around_explicit_origin() {
    #[cfg(feature = "double-precision")]
    let origin = Position::new(1.0e7, -1.0e7);
    #[cfg(not(feature = "double-precision"))]
    let origin = Position::new(10_000.0, -10_000.0);

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let shape = create_circle(&mut world, origin, 0.5);
    create_box(&mut world, origin.offset(Vec2::new(0.0, -0.01)), 20.0, 0.5);
    create_box(&mut world, origin.offset(Vec2::new(3.0, 1.0)), 0.25, 1.0);
    let query = world.query().unwrap();

    let overlaps = query
        .overlap_aabb(
            origin,
            Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]).unwrap(),
            QueryFilter::default(),
        )
        .unwrap();
    assert!(overlaps.contains(&shape));

    let proxy = ShapeProxy::new(
        [
            Vec2::new(-0.25, -0.25),
            Vec2::new(0.25, -0.25),
            Vec2::new(0.25, 0.25),
            Vec2::new(-0.25, 0.25),
        ],
        0.0,
    )
    .unwrap();
    assert!(
        query
            .overlap_shape(origin, proxy, QueryFilter::default())
            .unwrap()
            .contains(&shape)
    );

    let ray = query
        .cast_ray_closest(
            origin.offset(Vec2::new(0.0, 2.0)),
            [0.0_f32, -4.0],
            QueryFilter::default(),
        )
        .unwrap()
        .expect("ray should hit the circle at the explicit origin");
    assert_eq!(ray.shape_id, shape);
    let local_hit = ray.point.checked_relative_to(origin).unwrap();
    assert!(approx_eq(local_hit.x, 0.0, 1.0e-4));
    assert!(approx_eq(local_hit.y, 0.5, 1.0e-3));

    let cast_proxy = ShapeProxy::new(
        [
            Vec2::new(-0.1, 1.9),
            Vec2::new(0.1, 1.9),
            Vec2::new(0.1, 2.1),
            Vec2::new(-0.1, 2.1),
        ],
        0.0,
    )
    .unwrap();
    let cast_hits = query
        .cast_shape(origin, cast_proxy, [0.0_f32, -4.0], QueryFilter::default())
        .unwrap();
    let cast_hit = cast_hits
        .iter()
        .find(|hit| hit.shape_id == shape)
        .expect("shape cast should hit the circle at the explicit origin");
    assert!(cast_hit.hit);
    let local_cast_hit = cast_hit.point.checked_relative_to(origin).unwrap();
    assert!(local_cast_hit.is_valid());
    assert!(local_cast_hit.x.abs() < 1.0);
    assert!(local_cast_hit.y.abs() < 1.0);

    let c1 = Vec2::new(2.0, 0.7);
    let c2 = Vec2::new(2.0, 1.5);
    let mover_fraction = query
        .cast_mover(origin, c1, c2, 0.25, [2.0_f32, 0.0], QueryFilter::default())
        .unwrap();
    assert!((0.0..1.0).contains(&mover_fraction));

    let mover_planes = query
        .collide_mover(origin, c1, c2, 0.25, QueryFilter::default())
        .unwrap();
    assert!(mover_planes.iter().any(|result| result.hit));
    assert!(
        mover_planes
            .iter()
            .filter(|result| result.hit)
            .all(|result| result.point.is_valid()
                && result.point.x.abs() < 10.0
                && result.point.y.abs() < 10.0)
    );
}

#[test]
fn reusable_buffers_cover_hits_misses_and_visitor_short_circuit() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let shape = create_box(&mut world, Position::ZERO, 0.5, 0.5);
    let query = world.query().unwrap();
    let hit_bounds = Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]).unwrap();
    let miss_bounds = Aabb::new([50.0_f32, 50.0], [51.0, 51.0]).unwrap();

    let mut shapes = ShapeQueryBuffer::with_capacity(8).unwrap();
    let shape_capacity = shapes.capacity();
    query
        .overlap_aabb_into(
            Position::ZERO,
            hit_bounds,
            QueryFilter::default(),
            &mut shapes,
        )
        .unwrap();
    assert_eq!(shapes.as_slice(), &[shape]);
    assert_eq!(shapes.capacity(), shape_capacity);

    query
        .overlap_aabb_into(
            Position::ZERO,
            miss_bounds,
            QueryFilter::default(),
            &mut shapes,
        )
        .unwrap();
    assert!(shapes.is_empty());
    assert_eq!(shapes.capacity(), shape_capacity);

    let mut rays = RayQueryBuffer::with_capacity(8).unwrap();
    let ray_capacity = rays.capacity();
    query
        .cast_ray_all_into(
            Position::new(0.0, 2.0),
            [0.0_f32, -4.0],
            QueryFilter::default(),
            &mut rays,
        )
        .unwrap();
    assert!(rays.iter().any(|hit| hit.shape_id == shape));
    assert_eq!(rays.capacity(), ray_capacity);
    query
        .cast_ray_all_into(
            Position::new(50.0, 50.0),
            [1.0_f32, 0.0],
            QueryFilter::default(),
            &mut rays,
        )
        .unwrap();
    assert!(rays.is_empty());
    assert_eq!(rays.capacity(), ray_capacity);

    let mut visited = Vec::new();
    assert!(
        query
            .visit_overlap_aabb(
                Position::ZERO,
                hit_bounds,
                QueryFilter::default(),
                |shape_id| {
                    visited.push(shape_id);
                    true
                },
            )
            .unwrap()
    );
    assert_eq!(visited, [shape]);

    let mut stopped = 0;
    assert!(
        !query
            .visit_overlap_aabb(Position::ZERO, hit_bounds, QueryFilter::default(), |_| {
                stopped += 1;
                false
            },)
            .unwrap()
    );
    assert_eq!(stopped, 1);

    let mut empty_visits = 0;
    assert!(
        query
            .visit_overlap_aabb(Position::ZERO, miss_bounds, QueryFilter::default(), |_| {
                empty_visits += 1;
                true
            },)
            .unwrap()
    );
    assert_eq!(empty_visits, 0);
}

#[test]
fn offset_proxy_queries_share_the_same_capability_and_buffers() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let ground = create_box(&mut world, Position::ZERO, 20.0, 0.5);
    let query = world.query().unwrap();
    let points = [
        Vec2::new(-0.25, -0.25),
        Vec2::new(0.25, -0.25),
        Vec2::new(0.25, 0.25),
        Vec2::new(-0.25, 0.25),
    ];
    let overlap_proxy = ShapeProxy::offset_from_points(
        points,
        0.0,
        Transform::from_pos_angle([0.0_f32, 0.2], 0.0).unwrap(),
    )
    .unwrap();

    let overlap = query
        .overlap_shape(Position::ZERO, overlap_proxy, QueryFilter::default())
        .unwrap();
    assert!(overlap.contains(&ground));

    let mut visited = Vec::new();
    assert!(
        query
            .visit_overlap_shape(
                Position::ZERO,
                overlap_proxy,
                QueryFilter::default(),
                |shape_id| {
                    visited.push(shape_id);
                    true
                },
            )
            .unwrap()
    );
    assert_eq!(visited, overlap);

    let cast_proxy = ShapeProxy::offset_from_points(
        points,
        0.0,
        Transform::from_pos_angle([0.0_f32, 1.5], 0.0).unwrap(),
    )
    .unwrap();
    let mut casts = RayQueryBuffer::with_capacity(8).unwrap();
    query
        .cast_shape_into(
            Position::ZERO,
            cast_proxy,
            [0.0_f32, -2.0],
            QueryFilter::default(),
            &mut casts,
        )
        .unwrap();
    assert!(casts.iter().any(|hit| hit.shape_id == ground));
}

#[test]
fn visitor_panic_resumes_after_query_cleanup_and_later_queries_still_work() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    create_box(&mut world, Position::ZERO, 0.5, 0.5);
    let query = world.query().unwrap();
    let bounds = Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = query.visit_overlap_aabb(
            Position::ZERO,
            bounds,
            QueryFilter::default(),
            |_| -> bool { panic!("query visitor panic") },
        );
    }));
    assert!(result.is_err());

    let mut visited = 0;
    assert!(
        query
            .visit_overlap_aabb(Position::ZERO, bounds, QueryFilter::default(), |_| {
                visited += 1;
                true
            },)
            .unwrap()
    );
    assert_eq!(visited, 1);
}

#[test]
fn post_native_query_visitor_can_use_an_independent_dynamic_tree() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    create_box(&mut world, Position::ZERO, 0.5, 0.5);
    let query = world.query().unwrap();
    let bounds = Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]).unwrap();
    let mut tree = boxdd::DynamicTree::new().unwrap();
    tree.create_proxy(bounds, u64::MAX, 7).unwrap();
    let mut tree_hits = 0;

    assert!(
        query
            .visit_overlap_aabb(Position::ZERO, bounds, QueryFilter::default(), |_| {
                tree.query_all(bounds, &mut |_, user_data| {
                    assert_eq!(user_data, 7);
                    tree_hits += 1;
                    true
                })
                .unwrap();
                true
            })
            .unwrap()
    );
    assert_eq!(tree_hits, 1);
}
