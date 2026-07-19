use boxdd::{
    SimplexCache, Transform, Vec2, collide_capsule_and_circle, collide_capsules,
    collide_chain_segment_and_capsule, collide_chain_segment_and_circle,
    collide_chain_segment_and_polygon, collide_circles, collide_polygon_and_capsule,
    collide_polygon_and_circle, collide_polygons, collide_segment_and_capsule,
    collide_segment_and_circle, collide_segment_and_polygon,
    collision::{LocalManifold, LocalManifoldPoint},
    shapes, try_collide_chain_segment_and_capsule, try_collide_chain_segment_and_polygon,
};

#[test]
fn safe_manifold_collision_helpers_smoke() {
    let x_overlap = Transform::from_pos_angle([0.75_f32, 0.0], 0.0);
    let slight_up = Transform::from_pos_angle([0.0_f32, 0.25], 0.0);
    let slight_down = Transform::from_pos_angle([0.0_f32, -0.25], 0.0);
    let box_shift = Transform::from_pos_angle([1.25_f32, 0.0], 0.0);

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let capsule = shapes::capsule([-0.75_f32, 0.0], [0.75, 0.0], 0.35);
    let polygon = shapes::box_polygon(1.0, 1.0);
    let segment = shapes::segment([-1.5_f32, 0.0], [1.5, 0.0]);
    let chain_segment = shapes::chain_segment([-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]);

    let manifold = collide_circles(circle, circle, x_overlap);
    assert_eq!(manifold.points().len(), 1);
    assert!(manifold.normal.x > 0.5);

    assert!(
        !collide_capsule_and_circle(capsule, circle, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_circle(segment, circle, slight_up)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygon_and_circle(polygon, circle, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_capsules(capsule, capsule, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_capsule(segment, capsule, slight_up)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygon_and_capsule(polygon, capsule, box_shift)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygons(polygon, polygon, box_shift)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_polygon(segment, polygon, Transform::IDENTITY)
            .points()
            .is_empty()
    );
    assert!(
        !collide_chain_segment_and_circle(chain_segment, circle, slight_down)
            .points()
            .is_empty()
    );

    let mut cache = SimplexCache::default();
    assert!(
        !collide_chain_segment_and_capsule(chain_segment, capsule, slight_down, Some(&mut cache),)
            .points()
            .is_empty()
    );
    assert!(cache.count() <= 3);

    let mut cache = SimplexCache::default();
    assert!(
        !collide_chain_segment_and_polygon(chain_segment, polygon, slight_down, Some(&mut cache),)
            .points()
            .is_empty()
    );
    assert!(cache.count() <= 3);
}

#[test]
fn chain_manifold_helpers_supply_a_cache_when_the_caller_omits_one() {
    let segment = shapes::chain_segment([-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]);
    let capsule = shapes::capsule([-0.75_f32, 0.0], [0.75, 0.0], 0.35);
    let polygon = shapes::box_polygon(1.0, 1.0);
    let transform = Transform::from_pos_angle([0.0_f32, -0.25], 0.0);

    assert!(!collide_chain_segment_and_capsule(segment, capsule, transform, None).is_empty());
    assert!(
        !try_collide_chain_segment_and_capsule(segment, capsule, transform, None)
            .unwrap()
            .is_empty()
    );
    assert!(!collide_chain_segment_and_polygon(segment, polygon, transform, None).is_empty());
    assert!(
        !try_collide_chain_segment_and_polygon(segment, polygon, transform, None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn safe_manifold_collision_helpers_report_separation() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let separated = Transform::from_pos_angle([2.0_f32, 0.0], 0.0);

    let manifold = collide_circles(circle, circle, separated);
    assert!(manifold.points().is_empty());
}

#[test]
fn local_manifold_uses_shape_a_coordinates() {
    let circle_a = shapes::circle([0.0_f32, 0.0], 0.5);
    let circle_b = shapes::circle([1.0_f32, 0.0], 0.5);
    let quarter_turn_b_in_a =
        Transform::from_pos_angle([0.0_f32, 0.0], core::f32::consts::FRAC_PI_2);

    let manifold = collide_circles(circle_a, circle_b, quarter_turn_b_in_a);
    assert_eq!(manifold.point_count(), 1);
    assert!(manifold.normal.x.abs() < 1.0e-5);
    assert!(manifold.normal.y > 0.99);
    assert!(manifold.points()[0].point.x.abs() < 1.0e-5);
    assert!((manifold.points()[0].point.y - 0.5).abs() < 1.0e-5);
}

#[test]
fn local_manifold_value_types_use_explicit_raw_conversions() {
    let point = LocalManifoldPoint {
        point: Vec2::new(1.0, 2.0),
        separation: -0.1,
        id: 7,
    };
    assert_eq!(LocalManifoldPoint::from_raw(point.into_raw()), point);

    let manifold = LocalManifold {
        normal: Vec2::new(0.0, 1.0),
        contact_points: [point, LocalManifoldPoint::default()],
        point_count: 1,
    };
    let raw = manifold.into_raw();
    assert_eq!(raw.pointCount, 1);
    assert_eq!(LocalManifold::from_raw(raw), manifold);
}
