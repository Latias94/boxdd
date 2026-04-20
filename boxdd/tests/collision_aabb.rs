use boxdd::{Aabb, BodyBuilder, QueryFilter, RayResult, ShapeDef, Vec2, World, WorldDef, shapes};

fn approx(a: f32, b: f32, tol: f32) -> bool {
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
fn aabb_and_ray_result_use_explicit_raw_conversions() {
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

    let raw = boxdd_sys::ffi::b2RayResult {
        shapeId: shape.id(),
        point: boxdd_sys::ffi::b2Vec2 { x: 1.25, y: -2.5 },
        normal: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 1.0 },
        fraction: 0.375,
        hit: true,
        leafVisits: 0,
        nodeVisits: 0,
    };
    let hit = RayResult::from_raw(raw);

    assert_eq!(hit.shape_id.index1, shape.id().index1);
    assert_eq!(hit.shape_id.generation, shape.id().generation);
    assert!(approx(hit.point.x, 1.25, f32::EPSILON));
    assert!(approx(hit.point.y, -2.5, f32::EPSILON));
    assert!(approx(hit.normal.x, 0.0, f32::EPSILON));
    assert!(approx(hit.normal.y, 1.0, f32::EPSILON));
    assert!(approx(hit.fraction, 0.375, f32::EPSILON));
    assert!(hit.hit);

    let closest = world.cast_ray_closest([0.0_f32, 2.0], [0.0, -4.0], QueryFilter::default());
    assert!(closest.hit);
}
