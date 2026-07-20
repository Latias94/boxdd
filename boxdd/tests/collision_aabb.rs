use boxdd::{
    Aabb, BodyBuilder, Position, QueryFilter, ShapeDef, Vec2, World, WorldDef, WorldScalar, shapes,
};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn approx_world(a: WorldScalar, b: WorldScalar, tol: WorldScalar) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn aabb_valid_and_raycast() {
    let mut aabb = Aabb::new([-1.0, -1.0], [-2.0, -2.0]);
    assert!(!aabb.is_valid());

    aabb.upper = Vec2::new(1.0, 1.0);
    assert!(aabb.is_valid());

    let aabb = Aabb::new([-1.0, -1.0], [1.0, 1.0]);

    let hit = aabb.ray_cast([-3.0, 0.0], [6.0, 0.0]);
    assert!(hit.hit);
    assert!(approx(hit.fraction, 1.0 / 3.0, f32::EPSILON));
    assert!(approx(hit.normal.x, -1.0, f32::EPSILON));
    assert!(approx(hit.point.x, -1.0, f32::EPSILON));

    let hit = aabb.ray_cast([3.0, 0.0], [-6.0, 0.0]);
    assert!(hit.hit);
    assert!(approx(hit.fraction, 1.0 / 3.0, f32::EPSILON));
    assert!(approx(hit.normal.x, 1.0, f32::EPSILON));
    assert!(approx(hit.point.x, 1.0, f32::EPSILON));

    let hit = aabb.ray_cast([0.0, -3.0], [0.0, 6.0]);
    assert!(hit.hit);
    assert!(approx(hit.normal.y, -1.0, f32::EPSILON));
    assert!(approx(hit.point.y, -1.0, f32::EPSILON));

    let hit = aabb.ray_cast([0.0, 3.0], [0.0, -6.0]);
    assert!(hit.hit);
    assert!(approx(hit.normal.y, 1.0, f32::EPSILON));
    assert!(approx(hit.point.y, 1.0, f32::EPSILON));

    let miss = aabb.ray_cast([-3.0, 2.0], [6.0, 0.0]);
    assert!(!miss.hit);
    assert!(approx(miss.fraction, 0.0, f32::EPSILON));

    let overlap = aabb.ray_cast([0.0, 0.0], [1.0, 0.0]);
    assert!(overlap.hit);
    assert!(approx(overlap.fraction, 0.0, f32::EPSILON));
    assert!(approx(overlap.normal.x, 0.0, f32::EPSILON));
    assert!(approx(overlap.normal.y, 0.0, f32::EPSILON));
    assert!(approx(overlap.point.x, 0.0, f32::EPSILON));
    assert!(approx(overlap.point.y, 0.0, f32::EPSILON));
}

#[test]
fn aabb_raw_conversion_and_world_bound_ray_result() {
    let aabb = Aabb::new([-2.0, -1.5], [3.0, 4.5]);
    let raw = aabb.into_raw();

    assert!(approx(raw.lowerBound.x, -2.0, f32::EPSILON));
    assert!(approx(raw.lowerBound.y, -1.5, f32::EPSILON));
    assert!(approx(raw.upperBound.x, 3.0, f32::EPSILON));
    assert!(approx(raw.upperBound.y, 4.5, f32::EPSILON));
    assert_eq!(Aabb::from_raw(raw), aabb);

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for_owned(
        body.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let hit = world
        .cast_ray_closest(Position::new(0.0, 2.0), [0.0, -4.0], QueryFilter::default())
        .expect("ray should hit the circle");

    assert_eq!(hit.shape_id, shape.id());
    assert!(approx_world(hit.point.x, 0.0, 1.0e-5));
    assert!(approx_world(hit.point.y, 0.5, 1.0e-5));
    assert!(approx(hit.normal.x, 0.0, f32::EPSILON));
    assert!(approx(hit.normal.y, 1.0, f32::EPSILON));
    assert!(approx(hit.fraction, 0.375, f32::EPSILON));
    assert!(hit.hit);
}
