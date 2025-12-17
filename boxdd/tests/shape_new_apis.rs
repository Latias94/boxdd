use boxdd::prelude::*;
use boxdd::shapes;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn shape_closest_point_and_apply_wind_smoke() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let mut shape = world.create_polygon_shape_for_owned(body, &sdef, &poly);

    let target = Vec2::new(10.0, 0.0);
    let cp1 = shape.closest_point(target);
    let cp2 = world.shape_closest_point(shape.id(), target);
    assert!(approx_eq(cp1.x, cp2.x, 1e-6) && approx_eq(cp1.y, cp2.y, 1e-6));
    assert!(approx_eq(cp1.x, 0.5, 1e-3) && approx_eq(cp1.y, 0.0, 1e-3));

    let cp3 = shape.try_closest_point(target).unwrap();
    assert!(approx_eq(cp1.x, cp3.x, 1e-6) && approx_eq(cp1.y, cp3.y, 1e-6));

    world
        .try_shape_closest_point(shape.id(), target)
        .expect("try_shape_closest_point should succeed");

    shape.apply_wind(Vec2::new(5.0, 0.0), 1.0, 0.5, true);
    shape
        .try_apply_wind(Vec2::new(5.0, 0.0), 1.0, 0.5, true)
        .unwrap();
    world.shape_apply_wind(shape.id(), Vec2::new(5.0, 0.0), 1.0, 0.5, true);
    world
        .try_shape_apply_wind(shape.id(), Vec2::new(5.0, 0.0), 1.0, 0.5, true)
        .unwrap();
}
