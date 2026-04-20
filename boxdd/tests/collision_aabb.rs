use boxdd::{Aabb, Vec2};

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
