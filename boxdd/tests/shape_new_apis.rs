use boxdd::Filter;
use boxdd::prelude::*;
use boxdd::shapes;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn same_chain_id(a: ChainId, b: ChainId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
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

    let chain = world.create_chain_for_owned(
        body,
        &boxdd::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    let segment_shape_id = chain.segments()[0];
    let segment_shape = world
        .shape(segment_shape_id)
        .expect("chain segment shape should exist");
    assert_eq!(segment_shape.shape_type(), ShapeType::ChainSegment);
    assert!(
        segment_shape
            .parent_chain_id()
            .is_some_and(|parent| same_chain_id(parent, chain.id()))
    );

    let chain_segment = segment_shape.chain_segment();
    assert!(chain_segment.ghost1.x <= chain_segment.segment.point1.x);
    assert!(chain_segment.segment.point1.x < chain_segment.segment.point2.x);
    assert!(chain_segment.segment.point2.x <= chain_segment.ghost2.x);
}

#[test]
fn geometry_value_types_round_trip_through_explicit_raw_conversions() {
    let circle = shapes::circle([1.0_f32, -2.0], 0.5);
    assert_eq!(Circle::from_raw(circle.into_raw()), circle);

    let segment = shapes::segment([-1.0_f32, 2.0], [3.0, 4.0]);
    assert_eq!(Segment::from_raw(segment.into_raw()), segment);

    let chain_segment = shapes::chain_segment([-3.0_f32, 0.0], [-1.0, 0.0], [2.0, 0.0], [4.0, 0.0]);
    assert_eq!(
        ChainSegment::from_raw(chain_segment.into_raw()),
        chain_segment
    );

    let capsule = shapes::capsule([-1.0_f32, -1.0], [1.0, 1.0], 0.25);
    assert_eq!(Capsule::from_raw(capsule.into_raw()), capsule);

    let polygon = shapes::box_polygon(1.5, 0.75);
    let polygon_roundtrip = Polygon::from_raw(polygon.into_raw());
    assert_eq!(polygon_roundtrip.count(), polygon.count());
    assert!(approx_eq(
        polygon_roundtrip.radius(),
        polygon.radius(),
        f32::EPSILON
    ));
    assert!(approx_eq(
        polygon_roundtrip.centroid().x,
        polygon.centroid().x,
        f32::EPSILON
    ));
    assert!(approx_eq(
        polygon_roundtrip.centroid().y,
        polygon.centroid().y,
        f32::EPSILON
    ));
    for (lhs, rhs) in polygon_roundtrip.vertices().iter().zip(polygon.vertices()) {
        assert!(approx_eq(lhs.x, rhs.x, f32::EPSILON));
        assert!(approx_eq(lhs.y, rhs.y, f32::EPSILON));
    }
}
