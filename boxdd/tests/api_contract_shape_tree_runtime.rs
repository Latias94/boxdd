use boxdd::{Aabb, BodyType, SurfaceMaterial, Vec2, World, WorldScalar, shapes};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
}

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn approx_world_eq(actual: WorldScalar, expected: WorldScalar) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

#[test]
fn body_shape_creation_runtime_paths_succeed() {
    let mut world: World =
        boxdd::World::new(boxdd::WorldDef::default()).expect("world creation should succeed");
    let body_builder = boxdd::BodyBuilder::new();
    let body_builder = boxdd::BodyBuilder::body_type(body_builder, BodyType::Dynamic);
    let body_def = boxdd::BodyBuilder::build(body_builder);
    let mut body = boxdd::World::create_body(&mut world, body_def);
    let shape_def_builder = boxdd::ShapeDef::builder();
    let shape_def_builder = boxdd::ShapeDefBuilder::density(shape_def_builder, 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_def_builder);

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let circle_shape = boxdd::Body::create_circle_shape(&mut body, &shape_def, &circle);
    let circle_valid = boxdd::Shape::is_valid(&circle_shape);
    boxdd::Shape::destroy(circle_shape, true);

    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let capsule_shape = boxdd::Body::create_capsule_shape(&mut body, &shape_def, &capsule);
    let capsule_valid = boxdd::Shape::is_valid(&capsule_shape);
    boxdd::Shape::destroy(capsule_shape, true);

    let segment = shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]);
    let segment_shape = boxdd::Body::create_segment_shape(&mut body, &shape_def, &segment);
    let segment_valid = boxdd::Shape::is_valid(&segment_shape);
    boxdd::Shape::destroy(segment_shape, true);

    let polygon_shape = boxdd::Body::create_box(&mut body, &shape_def, 0.75, 0.5);
    let polygon_valid = boxdd::Shape::is_valid(&polygon_shape);
    boxdd::Shape::destroy(polygon_shape, true);

    assert!(circle_valid);
    assert!(capsule_valid);
    assert!(segment_valid);
    assert!(polygon_valid);
}

#[test]
fn owned_shape_runtime_paths_succeed() {
    let mut world: World =
        boxdd::World::new(boxdd::WorldDef::default()).expect("world creation should succeed");

    let sensor_body_builder = boxdd::BodyBuilder::new();
    let sensor_body_builder = boxdd::BodyBuilder::position(sensor_body_builder, [0.0_f32, 0.0]);
    let sensor_body_def = boxdd::BodyBuilder::build(sensor_body_builder);
    let sensor_body = boxdd::World::create_body_id(&mut world, sensor_body_def);

    let sensor_shape_def_builder = boxdd::ShapeDef::builder();
    let sensor_shape_def_builder = boxdd::ShapeDefBuilder::density(sensor_shape_def_builder, 1.0);
    let sensor_shape_def_builder = boxdd::ShapeDefBuilder::sensor(sensor_shape_def_builder, true);
    let sensor_shape_def_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(sensor_shape_def_builder, true);
    let sensor_shape_def = boxdd::ShapeDefBuilder::build(sensor_shape_def_builder);
    let sensor_circle = shapes::circle([0.0_f32, 0.0], 1.0);
    let mut sensor = boxdd::World::create_circle_shape_for_owned(
        &mut world,
        sensor_body,
        &sensor_shape_def,
        &sensor_circle,
    );

    let visitor_body_builder = boxdd::BodyBuilder::new();
    let visitor_body_builder =
        boxdd::BodyBuilder::body_type(visitor_body_builder, BodyType::Dynamic);
    let visitor_body_builder = boxdd::BodyBuilder::position(visitor_body_builder, [0.0_f32, 0.0]);
    let visitor_body_def = boxdd::BodyBuilder::build(visitor_body_builder);
    let visitor_body = boxdd::World::create_body_id(&mut world, visitor_body_def);

    let visitor_shape_def_builder = boxdd::ShapeDef::builder();
    let visitor_shape_def_builder = boxdd::ShapeDefBuilder::density(visitor_shape_def_builder, 1.0);
    let visitor_shape_def_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(visitor_shape_def_builder, true);
    let visitor_shape_def = boxdd::ShapeDefBuilder::build(visitor_shape_def_builder);
    let visitor_polygon = shapes::box_polygon(0.25, 0.25);
    let visitor = boxdd::World::create_polygon_shape_for_owned(
        &mut world,
        visitor_body,
        &visitor_shape_def,
        &visitor_polygon,
    );

    let geometry_body_builder = boxdd::BodyBuilder::new();
    let geometry_body_builder =
        boxdd::BodyBuilder::body_type(geometry_body_builder, BodyType::Dynamic);
    let geometry_body_builder = boxdd::BodyBuilder::position(geometry_body_builder, [5.0_f32, 0.0]);
    let geometry_body_def = boxdd::BodyBuilder::build(geometry_body_builder);
    let geometry_body = boxdd::World::create_body_id(&mut world, geometry_body_def);

    let shape_def_builder = boxdd::ShapeDef::builder();
    let shape_def_builder = boxdd::ShapeDefBuilder::density(shape_def_builder, 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_def_builder);
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let mut capsule_shape = boxdd::World::create_capsule_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &capsule,
    );
    let segment = shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]);
    let mut segment_shape = boxdd::World::create_segment_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &segment,
    );
    let polygon = shapes::box_polygon(0.5, 0.5);
    let mut polygon_shape = boxdd::World::create_polygon_shape_for_owned(
        &mut world,
        geometry_body,
        &shape_def,
        &polygon,
    );

    boxdd::World::step(&mut world, 1.0 / 60.0, 4);

    let valid = boxdd::OwnedShape::is_valid(&sensor);
    let is_sensor = boxdd::OwnedShape::is_sensor(&sensor);
    boxdd::OwnedShape::enable_sensor_events(&mut sensor, true);
    let sensor_events_enabled = boxdd::OwnedShape::sensor_events_enabled(&sensor);
    boxdd::OwnedShape::enable_contact_events(&mut sensor, true);
    let contact_events_enabled = boxdd::OwnedShape::contact_events_enabled(&sensor);
    boxdd::OwnedShape::enable_pre_solve_events(&mut sensor, true);
    let pre_solve_events_enabled = boxdd::OwnedShape::pre_solve_events_enabled(&sensor);
    boxdd::OwnedShape::enable_hit_events(&mut sensor, true);
    let hit_events_enabled = boxdd::OwnedShape::hit_events_enabled(&sensor);
    let shape_type = boxdd::OwnedShape::shape_type(&sensor);
    let observed_circle = boxdd::OwnedShape::circle(&sensor);

    let owner = boxdd::OwnedShape::body_id(&sensor);
    let shape_world = boxdd::OwnedShape::try_world_id_raw(&sensor);
    let bounds = boxdd::OwnedShape::aabb(&sensor);
    let bounds_valid = boxdd::Aabb::is_valid(bounds);

    boxdd::OwnedShape::set_density(&mut sensor, 2.0, true);
    let density = boxdd::OwnedShape::density(&sensor);
    let mass_data = boxdd::OwnedShape::mass_data(&sensor);

    boxdd::OwnedShape::set_friction(&mut sensor, 0.35);
    let friction = boxdd::OwnedShape::friction(&sensor);
    boxdd::OwnedShape::set_restitution(&mut sensor, 0.2);
    let restitution = boxdd::OwnedShape::restitution(&sensor);
    let observed_filter = boxdd::OwnedShape::filter(&sensor);

    let material = SurfaceMaterial::default();
    let material = SurfaceMaterial::with_friction(material, 0.45);
    let material = SurfaceMaterial::with_restitution(material, 0.3);
    boxdd::OwnedShape::set_surface_material(&mut sensor, &material);
    let observed_material = boxdd::OwnedShape::surface_material(&sensor);
    let observed_friction = SurfaceMaterial::friction(&observed_material);
    let observed_restitution = SurfaceMaterial::restitution(&observed_material);

    boxdd::OwnedShape::set_user_material(&mut sensor, 77);
    let user_material =
        boxdd::OwnedShape::try_user_material(&sensor).expect("user material query should succeed");
    boxdd::OwnedShape::set_user_data(&mut sensor, 91_u32);
    let user_data = boxdd::OwnedShape::try_user_data_ptr_raw(&sensor);
    let cleared_user_data = boxdd::OwnedShape::clear_user_data(&mut sensor);
    let contact_data = boxdd::OwnedShape::contact_data(&sensor);

    let contains_origin = boxdd::OwnedShape::test_point(&sensor, boxdd::Position::ZERO);
    let closest_point = boxdd::OwnedShape::closest_point(&sensor, boxdd::Position::new(2.0, 0.0));
    let cast =
        boxdd::OwnedShape::ray_cast(&sensor, boxdd::Position::new(-2.0, 0.0), [4.0_f32, 0.0]);

    let sensor_capacity = boxdd::OwnedShape::sensor_capacity(&sensor);
    let overlaps = boxdd::OwnedShape::sensor_overlaps(&sensor);
    let visitor_id = boxdd::OwnedShape::id(&visitor);
    let replacement_circle = shapes::circle([0.0_f32, 0.0], 0.75);
    boxdd::OwnedShape::set_circle(&mut sensor, &replacement_circle);
    boxdd::OwnedShape::set_filter(&mut sensor, boxdd::Filter::default());

    let original_capsule = boxdd::OwnedShape::capsule(&capsule_shape);
    boxdd::OwnedShape::apply_wind(&mut capsule_shape, [1.0_f32, 0.0], 0.5, 0.25, true);
    let replacement_capsule = shapes::capsule([-0.25_f32, 0.0], [0.25_f32, 0.0], 0.2);
    boxdd::OwnedShape::set_capsule(&mut capsule_shape, &replacement_capsule);
    let updated_capsule = boxdd::OwnedShape::capsule(&capsule_shape);

    let original_segment = boxdd::OwnedShape::segment(&segment_shape);
    let replacement_segment = shapes::segment([-1.0_f32, 0.0], [1.0_f32, 0.0]);
    boxdd::OwnedShape::set_segment(&mut segment_shape, &replacement_segment);
    let updated_segment = boxdd::OwnedShape::segment(&segment_shape);

    let replacement_polygon = shapes::box_polygon(0.75, 0.25);
    boxdd::OwnedShape::set_polygon(&mut polygon_shape, &replacement_polygon);
    let updated_polygon = boxdd::OwnedShape::polygon(&polygon_shape);
    let updated_polygon_count = boxdd::Polygon::count(&updated_polygon);

    std::mem::drop(visitor);
    boxdd::OwnedShape::destroy(sensor, true);

    assert!(valid);
    assert!(is_sensor);
    assert!(sensor_events_enabled);
    assert!(contact_events_enabled);
    assert!(pre_solve_events_enabled);
    assert!(hit_events_enabled);
    assert_eq!(shape_type, boxdd::ShapeType::Circle);
    assert!(approx_eq(observed_circle.radius, 1.0));
    assert_eq!(owner, sensor_body);
    assert!(shape_world.is_ok());
    assert!(bounds_valid);
    assert!(approx_eq(density, 2.0));
    assert!(mass_data.mass.is_finite());
    assert!(mass_data.center.x.is_finite() && mass_data.center.y.is_finite());
    assert!(approx_eq(friction, 0.35));
    assert!(approx_eq(restitution, 0.2));
    assert_eq!(observed_filter, boxdd::Filter::default());
    assert!(approx_eq(observed_friction, 0.45));
    assert!(approx_eq(observed_restitution, 0.3));
    assert_eq!(user_material, 77);
    assert!(!user_data.expect("user-data query should succeed").is_null());
    assert!(cleared_user_data);
    assert!(contact_data.is_empty());
    assert!(contains_origin);
    assert!(approx_world_eq(closest_point.x, 1.0));
    assert!(approx_world_eq(closest_point.y, 0.0));
    assert!(cast.hit);
    assert!(sensor_capacity >= 1);
    assert!(!overlaps.is_empty());
    assert!(overlaps.contains(&visitor_id));
    assert!(approx_eq(original_capsule.radius, 0.25));
    assert!(approx_eq(updated_capsule.radius, 0.2));
    assert!(original_segment.point1.x < original_segment.point2.x);
    assert!(approx_eq(updated_segment.point1.x, -1.0));
    assert!(approx_eq(updated_segment.point2.x, 1.0));
    assert_eq!(updated_polygon_count, 4);
}

#[test]
fn chain_segment_shape_runtime_paths_succeed() {
    let mut world: World =
        boxdd::World::new(boxdd::WorldDef::default()).expect("world creation should succeed");
    let body_builder = boxdd::BodyBuilder::new();
    let body_def = boxdd::BodyBuilder::build(body_builder);
    let body = boxdd::World::create_body_id(&mut world, body_def);

    let chain_def_builder = boxdd::ChainDef::builder();
    let chain_def_builder = boxdd::ChainDefBuilder::points(
        chain_def_builder,
        [
            [-2.0_f32, 0.0],
            [-1.0_f32, 0.0],
            [1.0_f32, 0.0],
            [2.0_f32, 0.0],
        ],
    );
    let chain_def = boxdd::ChainDefBuilder::build(chain_def_builder);
    let chain = boxdd::World::create_chain_for_owned(&mut world, body, &chain_def);
    let chain_id = boxdd::OwnedChain::id(&chain);
    let segments = boxdd::OwnedChain::segments(&chain);

    let segment_shape: boxdd::Shape<'_> =
        boxdd::World::shape(&mut world, segments[0]).expect("chain segment shape should be valid");
    let parent = boxdd::Shape::parent_chain_id(&segment_shape)
        .expect("chain segment should reference its parent chain");
    let geometry = boxdd::Shape::chain_segment(&segment_shape);
    std::mem::drop(segment_shape);
    boxdd::OwnedChain::destroy(chain);

    assert!(!segments.is_empty());
    assert_eq!(parent, chain_id);
    assert!(geometry.ghost1.x <= geometry.segment.point1.x);
    assert!(geometry.segment.point1.x < geometry.segment.point2.x);
    assert!(geometry.segment.point2.x <= geometry.ghost2.x);
}

#[test]
fn dynamic_tree_runtime_paths_succeed() {
    let default_tree = boxdd::DynamicTree::default();
    let default_proxy_count = boxdd::DynamicTree::proxy_count(&default_tree);
    std::mem::drop(default_tree);

    let capacity_tree = boxdd::DynamicTree::with_capacity(32);
    let capacity_proxy_count = boxdd::DynamicTree::proxy_count(&capacity_tree);
    std::mem::drop(capacity_tree);
    let invalid_capacity =
        boxdd::DynamicTree::try_with_capacity(boxdd::DynamicTree::MAX_PROXY_CAPACITY + 1);
    let invalid_capacity_rejected = invalid_capacity.is_err();

    let mut tree = boxdd::DynamicTree::new();
    let original_proxy =
        boxdd::DynamicTree::create_proxy(&mut tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 42);

    let mut foreign_tree = boxdd::DynamicTree::new();
    let foreign_proxy =
        boxdd::DynamicTree::create_proxy(&mut foreign_tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 7);
    let foreign_proxy_rejected =
        boxdd::DynamicTree::try_user_data(&tree, foreign_proxy) == Err(boxdd::ApiError::WrongTree);
    std::mem::drop(foreign_tree);

    let proxy_aabb = boxdd::DynamicTree::aabb(&tree, original_proxy);
    let proxy_aabb_valid = boxdd::Aabb::is_valid(proxy_aabb);
    let proxy = boxdd::DynamicTree::replace_category_bits(&mut tree, original_proxy, 0b101);
    let old_proxy_is_present = boxdd::DynamicTree::contains_proxy(&tree, original_proxy);
    let old_proxy_aabb = boxdd::DynamicTree::try_aabb(&tree, original_proxy);
    let old_proxy_aabb_rejected = old_proxy_aabb.is_err();
    let category_bits = boxdd::DynamicTree::category_bits(&mut tree, proxy);
    let replacement_user_data = boxdd::DynamicTree::user_data(&tree, proxy);
    let replacement_aabb = boxdd::DynamicTree::aabb(&tree, proxy);

    let height = boxdd::DynamicTree::height(&tree);
    let area_ratio = boxdd::DynamicTree::area_ratio(&tree);
    let root_bounds = boxdd::DynamicTree::root_bounds(&tree);
    let root_bounds_valid = boxdd::Aabb::is_valid(root_bounds);
    let proxy_count = boxdd::DynamicTree::proxy_count(&tree);
    let byte_count = boxdd::DynamicTree::byte_count(&tree);

    let mut box_hits = Vec::new();
    let box_cast_input = boxdd::TreeBoxCastInput::new(aabb(-3.0, -0.5, -2.0, 0.5), [4.0_f32, 0.0]);
    let box_cast_stats =
        boxdd::DynamicTree::try_box_cast(&tree, box_cast_input, u64::MAX, &mut |_, id, data| {
            box_hits.push((id, data));
            boxdd::TreeCastControl::Continue
        })
        .expect("valid box cast should succeed");

    boxdd::DynamicTree::move_proxy(&mut tree, proxy, aabb(-0.5, -0.5, 0.5, 0.5));
    let mut query_hits = 0;
    let query_stats =
        boxdd::DynamicTree::query(&tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b101, &mut |_, _| {
            query_hits += 1;
            true
        });
    let mut query_all_hits = 0;
    let query_all_stats =
        boxdd::DynamicTree::query_all(&tree, aabb(-1.0, -1.0, 1.0, 1.0), &mut |_, _| {
            query_all_hits += 1;
            true
        });
    let mut ray_hits = 0;
    let ray_stats = boxdd::DynamicTree::ray_cast(
        &tree,
        boxdd::TreeRayCastInput::new([-2.0_f32, 0.0], [4.0_f32, 0.0]),
        0b101,
        &mut |_, _, _| {
            ray_hits += 1;
            boxdd::TreeCastControl::Continue
        },
    );
    boxdd::DynamicTree::enlarge_proxy(&mut tree, proxy, aabb(-2.0, -2.0, 2.0, 2.0));
    let rebuilt = boxdd::DynamicTree::rebuild(&mut tree, true);
    boxdd::DynamicTree::validate(&tree);
    boxdd::DynamicTree::validate_no_enlarged(&tree);
    boxdd::DynamicTree::destroy_proxy(&mut tree, proxy);
    let recycled_proxy =
        boxdd::DynamicTree::create_proxy(&mut tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 84);
    let retired_proxy_rejected =
        boxdd::DynamicTree::try_user_data(&tree, proxy) == Err(boxdd::ApiError::InvalidTreeProxyId);
    let recycled_user_data = boxdd::DynamicTree::user_data(&tree, recycled_proxy);
    boxdd::DynamicTree::destroy_proxy(&mut tree, recycled_proxy);
    std::mem::drop(tree);

    assert_eq!(default_proxy_count, 0);
    assert_eq!(capacity_proxy_count, 0);
    assert!(invalid_capacity_rejected);
    assert!(foreign_proxy_rejected);
    assert!(proxy_aabb_valid);
    assert!(!old_proxy_is_present);
    assert!(old_proxy_aabb_rejected);
    assert_eq!(category_bits, 0b101);
    assert_eq!(replacement_user_data, 42);
    assert_eq!(replacement_aabb, proxy_aabb);
    assert_eq!(height, 0);
    assert!(area_ratio.is_finite() && area_ratio >= 0.0);
    assert!(root_bounds_valid);
    assert_eq!(proxy_count, 1);
    assert!(byte_count > 0);
    assert!(box_cast_stats.leaf_visits >= 1);
    assert!(box_hits.contains(&(proxy, 42)));
    assert!(retired_proxy_rejected);
    assert_eq!(recycled_user_data, 84);
    assert_eq!(query_hits, 1);
    assert_eq!(query_all_hits, 1);
    assert_eq!(ray_hits, 1);
    assert!(query_stats.leaf_visits >= 1);
    assert!(query_all_stats.leaf_visits >= 1);
    assert!(ray_stats.leaf_visits >= 1);
    assert_eq!(rebuilt, 1);
}
