use boxdd::{Transform, Vec2, shapes};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn shape_mass_aabb_point_raycast() {
    let circle = shapes::circle([1.0_f32, 0.0], 1.0);
    let boxp = shapes::box_polygon(1.0, 1.0);
    let segment = shapes::segment([0.0_f32, 1.0], [0.0, -1.0]);

    // Mass: circle
    let md = circle.mass_data(1.0);
    assert!(approx(md.mass, core::f32::consts::PI, f32::EPSILON));
    assert!(approx(md.center.x, 1.0, f32::EPSILON) && approx(md.center.y, 0.0, f32::EPSILON));
    assert!(approx(
        md.rotationalInertia,
        0.5 * core::f32::consts::PI,
        f32::EPSILON
    ));

    // Mass: capsule sandwich between hull and box bound
    let capsule = shapes::capsule([-1.0_f32, 0.0], [1.0, 0.0], 1.0);
    let m_cap = capsule.mass_data(1.0);
    let dx = capsule.center2.x - capsule.center1.x;
    let dy = capsule.center2.y - capsule.center1.y;
    let length = (dx * dx + dy * dy).sqrt();
    let r = shapes::box_polygon(capsule.radius, capsule.radius + 0.5 * length);
    let m_box = r.mass_data(1.0);

    const N: usize = 4;
    let mut pts = [Vec2::ZERO; 2 * N];
    let mut angle = -0.5 * core::f32::consts::PI;
    let d = core::f32::consts::PI / (N as f32 - 1.0);
    for point in pts.iter_mut().take(N) {
        point.x = 1.0 + capsule.radius * angle.cos();
        point.y = capsule.radius * angle.sin();
        angle += d;
    }
    angle = 0.5 * core::f32::consts::PI;
    for point in pts.iter_mut().skip(N).take(N) {
        point.x = -1.0 + capsule.radius * angle.cos();
        point.y = capsule.radius * angle.sin();
        angle += d;
    }
    let ac = shapes::polygon_from_points(pts, 0.0).expect("valid capsule hull polygon");
    let m_hull = ac.mass_data(1.0);
    assert!(m_hull.mass < m_cap.mass && m_cap.mass < m_box.mass);
    assert!(
        m_hull.rotationalInertia < m_cap.rotationalInertia
            && m_cap.rotationalInertia < m_box.rotationalInertia
    );

    // Mass: box
    let m = boxp.mass_data(1.0);
    assert!(approx(m.mass, 4.0, f32::EPSILON));
    assert!(approx(m.center.x, 0.0, f32::EPSILON));
    assert!(approx(m.center.y, 0.0, f32::EPSILON));
    assert!(approx(m.rotationalInertia, 8.0 / 3.0, 2.0 * f32::EPSILON));

    // AABB
    let a_circle = circle.aabb(Transform::IDENTITY);
    assert!(approx(a_circle.lower.x, 0.0, f32::EPSILON));
    assert!(approx(a_circle.lower.y, -1.0, f32::EPSILON));
    assert!(approx(a_circle.upper.x, 2.0, f32::EPSILON));
    assert!(approx(a_circle.upper.y, 1.0, f32::EPSILON));

    let a_box = boxp.aabb(Transform::IDENTITY);
    assert!(approx(a_box.lower.x, -1.0, f32::EPSILON));
    assert!(approx(a_box.lower.y, -1.0, f32::EPSILON));
    assert!(approx(a_box.upper.x, 1.0, f32::EPSILON));
    assert!(approx(a_box.upper.y, 1.0, f32::EPSILON));

    let a_seg = segment.aabb(Transform::IDENTITY);
    assert!(approx(a_seg.lower.x, 0.0, f32::EPSILON));
    assert!(approx(a_seg.lower.y, -1.0, f32::EPSILON));
    assert!(approx(a_seg.upper.x, 0.0, f32::EPSILON));
    assert!(approx(a_seg.upper.y, 1.0, f32::EPSILON));

    // Point in shape
    let p1 = [0.5_f32, 0.5];
    let p2 = [4.0_f32, -4.0];
    assert!(circle.contains_point(p1));
    assert!(!circle.contains_point(p2));
    assert!(boxp.contains_point(p1));
    assert!(!boxp.contains_point(p2));

    // Ray casts
    let out_c = circle.ray_cast([-4.0_f32, 0.0], [8.0, 0.0]);
    assert!(out_c.hit);
    assert!(approx(out_c.normal.x, -1.0, f32::EPSILON));
    assert!(approx(out_c.normal.y, 0.0, f32::EPSILON));
    assert!(approx(out_c.fraction, 0.5, f32::EPSILON));

    let out_p = boxp.ray_cast([-4.0_f32, 0.0], [8.0, 0.0]);
    assert!(out_p.hit);
    assert!(approx(out_p.normal.x, -1.0, f32::EPSILON));
    assert!(approx(out_p.normal.y, 0.0, f32::EPSILON));
    assert!(approx(out_p.fraction, 3.0 / 8.0, f32::EPSILON));
}
