use boxdd::{
    SimplexCache, Transform, collide_capsule_and_circle, collide_capsules,
    collide_chain_segment_and_capsule, collide_chain_segment_and_circle,
    collide_chain_segment_and_polygon, collide_circles, collide_polygon_and_capsule,
    collide_polygon_and_circle, collide_polygons, collide_segment_and_capsule,
    collide_segment_and_circle, collide_segment_and_polygon, shapes,
};

#[test]
fn safe_manifold_collision_helpers_smoke() {
    let identity = Transform::IDENTITY;
    let x_overlap = Transform::from_pos_angle([0.75_f32, 0.0], 0.0);
    let slight_up = Transform::from_pos_angle([0.0_f32, 0.25], 0.0);
    let slight_down = Transform::from_pos_angle([0.0_f32, -0.25], 0.0);
    let box_shift = Transform::from_pos_angle([1.25_f32, 0.0], 0.0);

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let capsule = shapes::capsule([-0.75_f32, 0.0], [0.75, 0.0], 0.35);
    let polygon = shapes::box_polygon(1.0, 1.0);
    let segment = shapes::segment([-1.5_f32, 0.0], [1.5, 0.0]);
    let chain_segment = shapes::chain_segment([-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]);

    let manifold = collide_circles(circle, identity, circle, x_overlap);
    assert_eq!(manifold.points().len(), 1);
    assert!(manifold.normal.x > 0.5);

    assert!(
        !collide_capsule_and_circle(capsule, identity, circle, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_circle(segment, identity, circle, slight_up)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygon_and_circle(polygon, identity, circle, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_capsules(capsule, identity, capsule, x_overlap)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_capsule(segment, identity, capsule, slight_up)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygon_and_capsule(polygon, identity, capsule, box_shift)
            .points()
            .is_empty()
    );
    assert!(
        !collide_polygons(polygon, identity, polygon, box_shift)
            .points()
            .is_empty()
    );
    assert!(
        !collide_segment_and_polygon(segment, identity, polygon, identity)
            .points()
            .is_empty()
    );
    assert!(
        !collide_chain_segment_and_circle(chain_segment, identity, circle, slight_down)
            .points()
            .is_empty()
    );

    let mut cache = SimplexCache::default();
    assert!(
        !collide_chain_segment_and_capsule(
            chain_segment,
            identity,
            capsule,
            slight_down,
            Some(&mut cache),
        )
        .points()
        .is_empty()
    );
    assert!(cache.count() <= 3);

    let mut cache = SimplexCache::default();
    assert!(
        !collide_chain_segment_and_polygon(
            chain_segment,
            identity,
            polygon,
            slight_down,
            Some(&mut cache),
        )
        .points()
        .is_empty()
    );
    assert!(cache.count() <= 3);
}

#[test]
fn safe_manifold_collision_helpers_report_separation() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let separated = Transform::from_pos_angle([2.0_f32, 0.0], 0.0);

    let manifold = collide_circles(circle, Transform::IDENTITY, circle, separated);
    assert!(manifold.points().is_empty());
}
