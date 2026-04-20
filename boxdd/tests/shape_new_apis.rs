use boxdd::Filter;
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

#[test]
fn shape_geometry_roundtrip_uses_safe_value_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder()
        .density(1.0)
        .filter(Filter::default())
        .build();

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let mut circle_shape = world.create_circle_shape_for_owned(body, &sdef, &circle);
    assert_eq!(circle_shape.circle(), circle);

    let updated_circle = shapes::circle([0.0_f32, 0.0], 0.25);
    circle_shape.set_circle(&updated_circle);
    assert_eq!(circle_shape.circle(), updated_circle);

    let poly = shapes::box_polygon(0.5, 0.5);
    let poly_shape = world.create_polygon_shape_for_owned(body, &sdef, &poly);
    assert_eq!(poly_shape.polygon().count(), 4);
    assert!(approx_eq(
        poly_shape.polygon().radius(),
        poly.radius(),
        f32::EPSILON
    ));

    let wider = shapes::box_polygon(1.0, 0.25);
    world.shape_set_polygon(poly_shape.id(), &wider);
    let updated = poly_shape.polygon();
    assert_eq!(updated.count(), 4);
    assert!(approx_eq(updated.vertices()[0].x.abs(), 1.0, 1.0e-6));
}
