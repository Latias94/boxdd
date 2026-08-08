use boxdd::{Aabb, BodyType, SurfaceMaterial, Vec2, World, WorldScalar, shapes};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)).unwrap()
}

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn approx_world_eq(actual: WorldScalar, expected: WorldScalar) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

#[test]
fn body_shape_creation_runtime_paths_succeed() {
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let body_builder = boxdd::BodyBuilder::body_type(body_builder, BodyType::Dynamic);
    let body_def = boxdd::BodyBuilder::build(body_builder).unwrap();
    let body_id =
        boxdd::World::create_body(&mut world, body_def).expect("body creation should succeed");
    let shape_def_builder = boxdd::ShapeDef::builder();
    let shape_def_builder = boxdd::ShapeDefBuilder::density(shape_def_builder, 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_def_builder).unwrap();

    let circle = shapes::circle([0.0_f32, 0.0], 0.5).unwrap();
    let circle_shape = boxdd::World::body(&mut world, body_id)
        .unwrap()
        .create_circle(&shape_def, &circle)
        .unwrap();
    boxdd::World::shape(&mut world, circle_shape)
        .unwrap()
        .destroy(true)
        .unwrap();

    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25).unwrap();
    let capsule_shape = boxdd::World::body(&mut world, body_id)
        .unwrap()
        .create_capsule(&shape_def, &capsule)
        .unwrap();
    boxdd::World::shape(&mut world, capsule_shape)
        .unwrap()
        .destroy(true)
        .unwrap();

    let segment = shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]).unwrap();
    let segment_shape = boxdd::World::body(&mut world, body_id)
        .unwrap()
        .create_segment(&shape_def, &segment)
        .unwrap();
    boxdd::World::shape(&mut world, segment_shape)
        .unwrap()
        .destroy(true)
        .unwrap();

    let polygon_shape = boxdd::World::body(&mut world, body_id)
        .unwrap()
        .create_box(&shape_def, 0.75, 0.5)
        .unwrap();
    boxdd::World::shape(&mut world, polygon_shape)
        .unwrap()
        .destroy(true)
        .unwrap();

    for shape in [circle_shape, capsule_shape, segment_shape, polygon_shape] {
        assert!(matches!(
            boxdd::World::shape(&mut world, shape),
            Err(boxdd::Error::InvalidShapeId)
        ));
    }
}

#[test]
fn scoped_shape_runtime_paths_succeed() {
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");

    let sensor_body_builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let sensor_body_builder = boxdd::BodyBuilder::position(sensor_body_builder, [0.0_f32, 0.0]);
    let sensor_body_def = boxdd::BodyBuilder::build(sensor_body_builder).unwrap();
    let sensor_body = boxdd::World::create_body(&mut world, sensor_body_def).unwrap();

    let sensor_shape_def_builder = boxdd::ShapeDef::builder();
    let sensor_shape_def_builder = boxdd::ShapeDefBuilder::density(sensor_shape_def_builder, 1.0);
    let sensor_shape_def_builder = boxdd::ShapeDefBuilder::sensor(sensor_shape_def_builder, true);
    let sensor_shape_def_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(sensor_shape_def_builder, true);
    let sensor_shape_def = boxdd::ShapeDefBuilder::build(sensor_shape_def_builder).unwrap();
    let sensor_circle = shapes::circle([0.0_f32, 0.0], 1.0).unwrap();
    let sensor_id = boxdd::World::body(&mut world, sensor_body)
        .unwrap()
        .create_circle(&sensor_shape_def, &sensor_circle)
        .unwrap();

    let visitor_body_builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let visitor_body_builder =
        boxdd::BodyBuilder::body_type(visitor_body_builder, BodyType::Dynamic);
    let visitor_body_builder = boxdd::BodyBuilder::position(visitor_body_builder, [0.0_f32, 0.0]);
    let visitor_body_def = boxdd::BodyBuilder::build(visitor_body_builder).unwrap();
    let visitor_body = boxdd::World::create_body(&mut world, visitor_body_def).unwrap();

    let visitor_shape_def_builder = boxdd::ShapeDef::builder();
    let visitor_shape_def_builder = boxdd::ShapeDefBuilder::density(visitor_shape_def_builder, 1.0);
    let visitor_shape_def_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(visitor_shape_def_builder, true);
    let visitor_shape_def = boxdd::ShapeDefBuilder::build(visitor_shape_def_builder).unwrap();
    let visitor_polygon = shapes::box_polygon(0.25, 0.25).unwrap();
    let visitor_id = boxdd::World::body(&mut world, visitor_body)
        .unwrap()
        .create_polygon(&visitor_shape_def, &visitor_polygon)
        .unwrap();

    let geometry_body_builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let geometry_body_builder =
        boxdd::BodyBuilder::body_type(geometry_body_builder, BodyType::Dynamic);
    let geometry_body_builder = boxdd::BodyBuilder::position(geometry_body_builder, [5.0_f32, 0.0]);
    let geometry_body_def = boxdd::BodyBuilder::build(geometry_body_builder).unwrap();
    let geometry_body = boxdd::World::create_body(&mut world, geometry_body_def).unwrap();

    let shape_def_builder = boxdd::ShapeDef::builder();
    let shape_def_builder = boxdd::ShapeDefBuilder::density(shape_def_builder, 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_def_builder).unwrap();
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25).unwrap();
    let capsule_shape_id = boxdd::World::body(&mut world, geometry_body)
        .unwrap()
        .create_capsule(&shape_def, &capsule)
        .unwrap();
    let segment = shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]).unwrap();
    let segment_shape_id = boxdd::World::body(&mut world, geometry_body)
        .unwrap()
        .create_segment(&shape_def, &segment)
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let polygon_shape_id = boxdd::World::body(&mut world, geometry_body)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();

    drop(boxdd::World::step(&mut world, 1.0 / 60.0, 4).unwrap());

    let mut sensor = boxdd::World::shape(&mut world, sensor_id).unwrap();
    let is_sensor = boxdd::Shape::is_sensor(&sensor).unwrap();
    boxdd::Shape::enable_sensor_events(&mut sensor, true).unwrap();
    let sensor_events_enabled = boxdd::Shape::sensor_events_enabled(&sensor).unwrap();
    boxdd::Shape::enable_contact_events(&mut sensor, true).unwrap();
    let contact_events_enabled = boxdd::Shape::contact_events_enabled(&sensor).unwrap();
    boxdd::Shape::enable_pre_solve_events(&mut sensor, true).unwrap();
    let pre_solve_events_enabled = boxdd::Shape::pre_solve_events_enabled(&sensor).unwrap();
    boxdd::Shape::enable_hit_events(&mut sensor, true).unwrap();
    let hit_events_enabled = boxdd::Shape::hit_events_enabled(&sensor).unwrap();
    let shape_type = boxdd::Shape::shape_type(&sensor).unwrap();
    let observed_circle = boxdd::Shape::circle(&sensor).unwrap();

    let owner = boxdd::Shape::body_id(&sensor).unwrap();
    let bounds = boxdd::Shape::aabb(&sensor).unwrap();
    let bounds_valid = boxdd::Aabb::is_valid(bounds);

    boxdd::Shape::set_density(&mut sensor, 2.0, true).unwrap();
    let density = boxdd::Shape::density(&sensor).unwrap();
    let mass_data = boxdd::Shape::mass_data(&sensor).unwrap();

    boxdd::Shape::set_friction(&mut sensor, 0.35).unwrap();
    let friction = boxdd::Shape::friction(&sensor).unwrap();
    boxdd::Shape::set_restitution(&mut sensor, 0.2).unwrap();
    let restitution = boxdd::Shape::restitution(&sensor).unwrap();
    let observed_filter = boxdd::Shape::filter(&sensor).unwrap();

    let material = SurfaceMaterial::default()
        .with_friction(0.45)
        .unwrap()
        .with_restitution(0.3)
        .unwrap();
    boxdd::Shape::set_surface_material(&mut sensor, &material).unwrap();
    let observed_material = boxdd::Shape::surface_material(&sensor).unwrap();
    let observed_friction = SurfaceMaterial::friction(&observed_material);
    let observed_restitution = SurfaceMaterial::restitution(&observed_material);

    boxdd::Shape::set_user_material(&mut sensor, 77).unwrap();
    let user_material =
        boxdd::Shape::user_material(&sensor).expect("user material query should succeed");
    boxdd::Shape::set_user_data(&mut sensor, 91_u32).unwrap();
    let user_data = boxdd::Shape::user_data_ptr_raw(&sensor).unwrap();
    let cleared_user_data = boxdd::Shape::clear_user_data(&mut sensor).unwrap();
    let contact_data = boxdd::Shape::contact_data(&sensor).unwrap();

    let contains_origin = boxdd::Shape::test_point(&sensor, boxdd::Position::ZERO).unwrap();
    let closest_point =
        boxdd::Shape::closest_point(&sensor, boxdd::Position::new(2.0, 0.0)).unwrap();
    let cast =
        boxdd::Shape::ray_cast(&sensor, boxdd::Position::new(-2.0, 0.0), [4.0_f32, 0.0]).unwrap();

    let sensor_capacity = boxdd::Shape::sensor_capacity(&sensor).unwrap();
    let overlaps = boxdd::Shape::sensor_overlaps(&sensor).unwrap();
    let replacement_circle = shapes::circle([0.0_f32, 0.0], 0.75).unwrap();
    boxdd::Shape::set_circle(&mut sensor, &replacement_circle).unwrap();
    boxdd::Shape::set_filter(&mut sensor, boxdd::Filter::default()).unwrap();
    boxdd::Shape::destroy(sensor, true).unwrap();

    let (original_capsule, updated_capsule) = {
        let mut capsule_shape = boxdd::World::shape(&mut world, capsule_shape_id).unwrap();
        let original = boxdd::Shape::capsule(&capsule_shape).unwrap();
        boxdd::Shape::apply_wind(&mut capsule_shape, [1.0_f32, 0.0], 0.5, 0.25, true).unwrap();
        let replacement = shapes::capsule([-0.25_f32, 0.0], [0.25_f32, 0.0], 0.2).unwrap();
        boxdd::Shape::set_capsule(&mut capsule_shape, &replacement).unwrap();
        (original, boxdd::Shape::capsule(&capsule_shape).unwrap())
    };

    let (original_segment, updated_segment) = {
        let mut segment_shape = boxdd::World::shape(&mut world, segment_shape_id).unwrap();
        let original = boxdd::Shape::segment(&segment_shape).unwrap();
        let replacement = shapes::segment([-1.0_f32, 0.0], [1.0_f32, 0.0]).unwrap();
        boxdd::Shape::set_segment(&mut segment_shape, &replacement).unwrap();
        (original, boxdd::Shape::segment(&segment_shape).unwrap())
    };

    let mut polygon_shape = boxdd::World::shape(&mut world, polygon_shape_id).unwrap();
    let replacement_polygon = shapes::box_polygon(0.75, 0.25).unwrap();
    boxdd::Shape::set_polygon(&mut polygon_shape, &replacement_polygon).unwrap();
    let updated_polygon = boxdd::Shape::polygon(&polygon_shape).unwrap();
    let updated_polygon_count = boxdd::Polygon::count(&updated_polygon);

    assert!(is_sensor);
    assert!(sensor_events_enabled);
    assert!(contact_events_enabled);
    assert!(pre_solve_events_enabled);
    assert!(hit_events_enabled);
    assert_eq!(shape_type, boxdd::ShapeType::Circle);
    assert!(approx_eq(observed_circle.radius(), 1.0));
    assert_eq!(owner, sensor_body);
    assert!(bounds_valid);
    assert!(approx_eq(density, 2.0));
    assert!(mass_data.mass().is_finite());
    assert!(mass_data.center().x.is_finite() && mass_data.center().y.is_finite());
    assert!(approx_eq(friction, 0.35));
    assert!(approx_eq(restitution, 0.2));
    assert_eq!(observed_filter, boxdd::Filter::default());
    assert!(approx_eq(observed_friction, 0.45));
    assert!(approx_eq(observed_restitution, 0.3));
    assert_eq!(user_material, 77);
    assert!(!user_data.is_null());
    assert!(cleared_user_data);
    assert!(contact_data.is_empty());
    assert!(contains_origin);
    assert!(approx_world_eq(closest_point.x, 1.0));
    assert!(approx_world_eq(closest_point.y, 0.0));
    assert!(cast.hit);
    assert!(sensor_capacity >= 1);
    assert!(!overlaps.is_empty());
    assert!(overlaps.contains(&visitor_id));
    assert!(approx_eq(original_capsule.radius(), 0.25));
    assert!(approx_eq(updated_capsule.radius(), 0.2));
    assert!(original_segment.point1().x < original_segment.point2().x);
    assert!(approx_eq(updated_segment.point1().x, -1.0));
    assert!(approx_eq(updated_segment.point2().x, 1.0));
    assert_eq!(updated_polygon_count, 4);
}

#[test]
fn chain_segment_shape_runtime_paths_succeed() {
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_builder = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a BodyDef")
        .body_builder();
    let body_def = boxdd::BodyBuilder::build(body_builder).unwrap();
    let body = boxdd::World::create_body(&mut world, body_def).unwrap();

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
    let chain_def = boxdd::ChainDefBuilder::build(chain_def_builder).unwrap();
    let chain_id = boxdd::World::body(&mut world, body)
        .unwrap()
        .create_chain(&chain_def)
        .unwrap();
    let segments = boxdd::World::chain(&mut world, chain_id)
        .unwrap()
        .segments()
        .unwrap();

    let (parent, geometry) = {
        let segment_shape: boxdd::Shape<'_> = boxdd::World::shape(&mut world, segments[0])
            .expect("chain segment shape should be valid");
        let parent = boxdd::Shape::parent_chain_id(&segment_shape)
            .unwrap()
            .expect("chain segment should reference its parent chain");
        let geometry = boxdd::Shape::chain_segment(&segment_shape).unwrap();
        (parent, geometry)
    };
    boxdd::World::chain(&mut world, chain_id)
        .unwrap()
        .destroy()
        .unwrap();

    assert!(!segments.is_empty());
    assert_eq!(parent, chain_id);
    assert!(geometry.ghost1().x <= geometry.segment().point1().x);
    assert!(geometry.segment().point1().x < geometry.segment().point2().x);
    assert!(geometry.segment().point2().x <= geometry.ghost2().x);
}

#[test]
fn dynamic_tree_runtime_paths_succeed() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");

    let default_tree = boxdd::DynamicTree::new().expect("default tree creation should succeed");
    let default_proxy_count =
        boxdd::DynamicTree::proxy_count(&default_tree).expect("proxy count should be readable");
    std::mem::drop(default_tree);

    let capacity_tree =
        boxdd::DynamicTree::with_capacity(32).expect("valid capacity should create a tree");
    let capacity_proxy_count =
        boxdd::DynamicTree::proxy_count(&capacity_tree).expect("proxy count should be readable");
    std::mem::drop(capacity_tree);
    let invalid_capacity =
        boxdd::DynamicTree::with_capacity(boxdd::DynamicTree::MAX_PROXY_CAPACITY + 1);
    let invalid_capacity_rejected = invalid_capacity.is_err();

    let mut tree = boxdd::DynamicTree::new().expect("tree creation should succeed");
    let original_proxy =
        boxdd::DynamicTree::create_proxy(&mut tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 42)
            .expect("valid proxy should be created");

    let mut foreign_tree = boxdd::DynamicTree::new().expect("tree creation should succeed");
    let foreign_proxy =
        boxdd::DynamicTree::create_proxy(&mut foreign_tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 7)
            .expect("valid proxy should be created");
    let foreign_proxy_rejected =
        boxdd::DynamicTree::user_data(&tree, foreign_proxy) == Err(boxdd::Error::WrongTree);
    std::mem::drop(foreign_tree);

    let proxy_aabb = boxdd::DynamicTree::aabb(&tree, original_proxy)
        .expect("live proxy AABB should be readable");
    let proxy_aabb_valid = boxdd::Aabb::is_valid(proxy_aabb);
    let proxy = boxdd::DynamicTree::replace_category_bits(&mut tree, original_proxy, 0b101)
        .expect("live proxy category bits should be replaceable");
    let old_proxy_is_present = boxdd::DynamicTree::contains_proxy(&tree, original_proxy);
    let old_proxy_aabb = boxdd::DynamicTree::aabb(&tree, original_proxy);
    let old_proxy_aabb_rejected = old_proxy_aabb.is_err();
    let category_bits = boxdd::DynamicTree::category_bits(&mut tree, proxy)
        .expect("live proxy category bits should be readable");
    let replacement_user_data = boxdd::DynamicTree::user_data(&tree, proxy)
        .expect("live proxy user data should be readable");
    let replacement_aabb =
        boxdd::DynamicTree::aabb(&tree, proxy).expect("live proxy AABB should be readable");

    let height = boxdd::DynamicTree::height(&tree).expect("tree height should be readable");
    let area_ratio =
        boxdd::DynamicTree::area_ratio(&tree).expect("tree area ratio should be readable");
    let root_bounds =
        boxdd::DynamicTree::root_bounds(&tree).expect("tree root bounds should be readable");
    let root_bounds_valid = boxdd::Aabb::is_valid(root_bounds);
    let proxy_count =
        boxdd::DynamicTree::proxy_count(&tree).expect("proxy count should be readable");
    let byte_count =
        boxdd::DynamicTree::byte_count(&tree).expect("tree byte count should be readable");

    let mut box_hits = Vec::new();
    let box_cast_input =
        boxdd::TreeBoxCastInput::new(aabb(-3.0, -0.5, -2.0, 0.5), [4.0_f32, 0.0]).unwrap();
    let box_cast_stats =
        boxdd::DynamicTree::box_cast(&tree, box_cast_input, u64::MAX, &mut |_, id, data| {
            box_hits.push((id, data));
            boxdd::TreeCastControl::Continue
        })
        .expect("valid box cast should succeed");

    boxdd::DynamicTree::move_proxy(&mut tree, proxy, aabb(-0.5, -0.5, 0.5, 0.5))
        .expect("valid proxy move should succeed");
    let mut query_hits = 0;
    let query_stats =
        boxdd::DynamicTree::query(&tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b101, &mut |_, _| {
            query_hits += 1;
            true
        })
        .expect("valid query should succeed");
    let mut query_all_hits = 0;
    let query_all_stats =
        boxdd::DynamicTree::query_all(&tree, aabb(-1.0, -1.0, 1.0, 1.0), &mut |_, _| {
            query_all_hits += 1;
            true
        })
        .expect("valid query should succeed");
    let mut ray_hits = 0;
    let ray_stats = boxdd::DynamicTree::ray_cast(
        &tree,
        boxdd::TreeRayCastInput::new([-2.0_f32, 0.0], [4.0_f32, 0.0]).unwrap(),
        0b101,
        &mut |_, _, _| {
            ray_hits += 1;
            boxdd::TreeCastControl::Continue
        },
    )
    .expect("valid ray cast should succeed");
    boxdd::DynamicTree::enlarge_proxy(&mut tree, proxy, aabb(-2.0, -2.0, 2.0, 2.0))
        .expect("valid proxy enlargement should succeed");
    let rebuilt =
        boxdd::DynamicTree::rebuild(&mut tree, true).expect("tree rebuild should succeed");
    boxdd::DynamicTree::validate(&tree).expect("rebuilt tree should be valid");
    boxdd::DynamicTree::destroy_proxy(&mut tree, proxy).expect("live proxy should be destroyed");
    let recycled_proxy =
        boxdd::DynamicTree::create_proxy(&mut tree, aabb(-1.0, -1.0, 1.0, 1.0), 0b001, 84)
            .expect("valid proxy should be created");
    let retired_proxy_rejected =
        boxdd::DynamicTree::user_data(&tree, proxy) == Err(boxdd::Error::InvalidTreeProxyId);
    let recycled_user_data = boxdd::DynamicTree::user_data(&tree, recycled_proxy)
        .expect("live proxy user data should be readable");
    boxdd::DynamicTree::destroy_proxy(&mut tree, recycled_proxy)
        .expect("live proxy should be destroyed");
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
    assert!(box_cast_stats.leaf_visits() >= 1);
    assert!(box_hits.contains(&(proxy, 42)));
    assert!(retired_proxy_rejected);
    assert_eq!(recycled_user_data, 84);
    assert_eq!(query_hits, 1);
    assert_eq!(query_all_hits, 1);
    assert_eq!(ray_hits, 1);
    assert!(query_stats.leaf_visits() >= 1);
    assert!(query_all_stats.leaf_visits() >= 1);
    assert!(ray_stats.leaf_visits() >= 1);
    assert_eq!(rebuilt, 1);
}
