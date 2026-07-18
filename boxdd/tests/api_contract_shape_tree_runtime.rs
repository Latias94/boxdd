use boxdd::{Aabb, Vec2, prelude::*, shapes};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
}

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

#[test]
fn body_shape_creation_runtime_paths_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let mut body = boxdd::World::create_body(
        &mut world,
        BodyBuilder::new().body_type(BodyType::Dynamic).build(),
    );
    let shape_def = ShapeDef::builder().density(1.0).build();

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let circle_shape = boxdd::Body::create_circle_shape(&mut body, &shape_def, &circle);
    let circle_valid = boxdd::Shape::is_valid(&circle_shape);
    assert!(circle_valid);
    boxdd::Shape::destroy(circle_shape, true);

    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let capsule_shape = boxdd::Body::create_capsule_shape(&mut body, &shape_def, &capsule);
    let capsule_valid = boxdd::Shape::is_valid(&capsule_shape);
    assert!(capsule_valid);
    boxdd::Shape::destroy(capsule_shape, true);

    let segment = shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]);
    let segment_shape = boxdd::Body::create_segment_shape(&mut body, &shape_def, &segment);
    let segment_valid = boxdd::Shape::is_valid(&segment_shape);
    assert!(segment_valid);
    boxdd::Shape::destroy(segment_shape, true);

    let polygon_shape = boxdd::Body::create_box(&mut body, &shape_def, 0.75, 0.5);
    let polygon_valid = boxdd::Shape::is_valid(&polygon_shape);
    assert!(polygon_valid);
    boxdd::Shape::destroy(polygon_shape, true);
}

#[test]
fn owned_shape_runtime_paths_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let sensor_body = boxdd::World::create_body_id(
        &mut world,
        BodyBuilder::new().position([0.0_f32, 0.0]).build(),
    );
    let mut sensor = boxdd::World::create_circle_shape_for_owned(
        &mut world,
        sensor_body,
        &ShapeDef::builder()
            .density(1.0)
            .sensor(true)
            .enable_sensor_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 1.0),
    );

    let visitor_body = boxdd::World::create_body_id(
        &mut world,
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .build(),
    );
    let visitor = boxdd::World::create_polygon_shape_for_owned(
        &mut world,
        visitor_body,
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &shapes::box_polygon(0.25, 0.25),
    );

    let geometry_body = boxdd::World::create_body_id(
        &mut world,
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([5.0_f32, 0.0])
            .build(),
    );
    let shape_def = ShapeDef::builder().density(1.0).build();
    let mut capsule_shape = boxdd::World::create_capsule_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25),
    );
    let mut segment_shape = boxdd::World::create_segment_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]),
    );
    let mut polygon_shape = boxdd::World::create_polygon_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &shapes::box_polygon(0.5, 0.5),
    );

    boxdd::World::step(&mut world, 1.0 / 60.0, 4);

    let valid = boxdd::OwnedShape::is_valid(&sensor);
    assert!(valid);
    let is_sensor = boxdd::OwnedShape::is_sensor(&sensor);
    assert!(is_sensor);
    boxdd::OwnedShape::enable_sensor_events(&mut sensor, true);
    let sensor_events_enabled = boxdd::OwnedShape::sensor_events_enabled(&sensor);
    assert!(sensor_events_enabled);

    let owner = boxdd::OwnedShape::body_id(&sensor);
    assert_eq!(owner, sensor_body);
    let bounds = boxdd::OwnedShape::aabb(&sensor);
    assert!(bounds.is_valid());

    boxdd::OwnedShape::set_density(&mut sensor, 2.0, true);
    let density = boxdd::OwnedShape::density(&sensor);
    assert!(approx_eq(density, 2.0));
    let mass_data = boxdd::OwnedShape::mass_data(&sensor);
    assert!(mass_data.mass.is_finite());
    assert!(mass_data.center.x.is_finite() && mass_data.center.y.is_finite());

    boxdd::OwnedShape::set_friction(&mut sensor, 0.35);
    let friction = boxdd::OwnedShape::friction(&sensor);
    assert!(approx_eq(friction, 0.35));
    boxdd::OwnedShape::set_restitution(&mut sensor, 0.2);
    let restitution = boxdd::OwnedShape::restitution(&sensor);
    assert!(approx_eq(restitution, 0.2));

    let material = SurfaceMaterial::default()
        .with_friction(0.45)
        .with_restitution(0.3);
    boxdd::OwnedShape::set_surface_material(&mut sensor, &material);
    let observed_material = boxdd::OwnedShape::surface_material(&sensor);
    assert!(approx_eq(observed_material.friction(), 0.45));
    assert!(approx_eq(observed_material.restitution(), 0.3));

    boxdd::OwnedShape::set_user_material(&mut sensor, 77);
    let user_material =
        boxdd::OwnedShape::try_user_material(&sensor).expect("user material query should succeed");
    assert_eq!(user_material, 77);
    boxdd::OwnedShape::set_user_data(&mut sensor, 91_u32);
    let cleared_user_data = boxdd::OwnedShape::clear_user_data(&mut sensor);
    assert!(cleared_user_data);

    let contains_origin = boxdd::OwnedShape::test_point(&sensor, [0.0_f32, 0.0]);
    assert!(contains_origin);
    let cast = boxdd::OwnedShape::ray_cast(&sensor, [-2.0_f32, 0.0], [4.0_f32, 0.0]);
    assert!(cast.hit);

    let sensor_capacity = boxdd::OwnedShape::sensor_capacity(&sensor);
    assert!(sensor_capacity >= 1);
    let overlaps = boxdd::OwnedShape::sensor_overlaps(&sensor);
    assert!(!overlaps.is_empty());
    assert!(overlaps.iter().any(|shape| *shape == visitor.id()));

    let original_capsule = boxdd::OwnedShape::capsule(&capsule_shape);
    assert!(approx_eq(original_capsule.radius, 0.25));
    let replacement_capsule = shapes::capsule([-0.25_f32, 0.0], [0.25_f32, 0.0], 0.2);
    boxdd::OwnedShape::set_capsule(&mut capsule_shape, &replacement_capsule);
    let updated_capsule = boxdd::OwnedShape::capsule(&capsule_shape);
    assert!(approx_eq(updated_capsule.radius, 0.2));

    let original_segment = boxdd::OwnedShape::segment(&segment_shape);
    assert!(original_segment.point1.x < original_segment.point2.x);
    let replacement_segment = shapes::segment([-1.0_f32, 0.0], [1.0_f32, 0.0]);
    boxdd::OwnedShape::set_segment(&mut segment_shape, &replacement_segment);
    let updated_segment = boxdd::OwnedShape::segment(&segment_shape);
    assert!(approx_eq(updated_segment.point1.x, -1.0));
    assert!(approx_eq(updated_segment.point2.x, 1.0));

    let replacement_polygon = shapes::box_polygon(0.75, 0.25);
    boxdd::OwnedShape::set_polygon(&mut polygon_shape, &replacement_polygon);
    let updated_polygon = boxdd::OwnedShape::polygon(&polygon_shape);
    assert_eq!(updated_polygon.count(), 4);

    boxdd::OwnedShape::destroy(sensor, true);
}

#[test]
fn chain_segment_shape_runtime_paths_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body = boxdd::World::create_body_id(&mut world, BodyBuilder::new().build());
    let chain = boxdd::World::create_chain_for_owned(
        &mut world,
        body,
        &boxdd::ChainDef::builder()
            .points([
                [-2.0_f32, 0.0],
                [-1.0_f32, 0.0],
                [1.0_f32, 0.0],
                [2.0_f32, 0.0],
            ])
            .build(),
    );
    let chain_id = boxdd::OwnedChain::id(&chain);
    let segments = boxdd::OwnedChain::segments(&chain);
    assert!(!segments.is_empty());

    let segment_shape =
        boxdd::World::shape(&mut world, segments[0]).expect("chain segment shape should be valid");
    let parent = boxdd::Shape::parent_chain_id(&segment_shape)
        .expect("chain segment should reference its parent chain");
    assert_eq!(parent, chain_id);
    let geometry = boxdd::Shape::chain_segment(&segment_shape);
    assert!(geometry.ghost1.x <= geometry.segment.point1.x);
    assert!(geometry.segment.point1.x < geometry.segment.point2.x);
    assert!(geometry.segment.point2.x <= geometry.ghost2.x);

    std::mem::drop(segment_shape);
    boxdd::OwnedChain::destroy(chain);
}

#[test]
fn dynamic_tree_runtime_paths_succeed() {
    let default_tree = boxdd::DynamicTree::default();
    let default_proxy_count = boxdd::DynamicTree::proxy_count(&default_tree);
    assert_eq!(default_proxy_count, 0);
    std::mem::drop(default_tree);

    let mut tree = boxdd::DynamicTree::new();
    let original_proxy =
        boxdd::DynamicTree::create_proxy(&mut tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 42);

    let proxy_aabb = boxdd::DynamicTree::aabb(&tree, original_proxy);
    assert!(proxy_aabb.is_valid());
    let proxy = boxdd::DynamicTree::replace_category_bits(&mut tree, original_proxy, 0b101);
    let old_proxy_is_present = boxdd::DynamicTree::contains_proxy(&tree, original_proxy);
    assert!(!old_proxy_is_present);
    let old_proxy_aabb = boxdd::DynamicTree::try_aabb(&tree, original_proxy);
    assert!(old_proxy_aabb.is_err());
    let category_bits = boxdd::DynamicTree::category_bits(&mut tree, proxy);
    assert_eq!(category_bits, 0b101);
    let replacement_user_data = boxdd::DynamicTree::user_data(&tree, proxy);
    assert_eq!(replacement_user_data, 42);
    let replacement_aabb = boxdd::DynamicTree::aabb(&tree, proxy);
    assert_eq!(replacement_aabb, proxy_aabb);

    let height = boxdd::DynamicTree::height(&tree);
    assert_eq!(height, 0);
    let area_ratio = boxdd::DynamicTree::area_ratio(&tree);
    assert!(area_ratio.is_finite() && area_ratio >= 0.0);
    let root_bounds = boxdd::DynamicTree::root_bounds(&tree);
    assert!(root_bounds.is_valid());
    let proxy_count = boxdd::DynamicTree::proxy_count(&tree);
    assert_eq!(proxy_count, 1);
    let byte_count = boxdd::DynamicTree::byte_count(&tree);
    assert!(byte_count > 0);
    boxdd::DynamicTree::validate(&tree);

    boxdd::DynamicTree::enlarge_proxy(&mut tree, proxy, aabb(-2.0, -2.0, 2.0, 2.0));
    let rebuilt = boxdd::DynamicTree::rebuild(&mut tree, true);
    assert_eq!(rebuilt, 1);
    boxdd::DynamicTree::validate(&tree);
    boxdd::DynamicTree::validate_no_enlarged(&tree);

    boxdd::DynamicTree::destroy_proxy(&mut tree, proxy);
    std::mem::drop(tree);
}
