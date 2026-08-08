#[test]
fn remaining_runtime_paths_have_direct_ufcs_evidence() {
    let mut world: boxdd::World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");

    let static_body_def = boxdd::BodyBuilder::build(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
    )
    .unwrap();
    let static_body = boxdd::World::create_body(&mut world, static_body_def)
        .expect("static body creation should succeed");
    let dynamic_body_builder = boxdd::BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        boxdd::BodyType::Dynamic,
    );
    let dynamic_body_builder = boxdd::BodyBuilder::position(dynamic_body_builder, [2.0_f32, 0.0]);
    let dynamic_body = boxdd::World::create_body(
        &mut world,
        boxdd::BodyBuilder::build(dynamic_body_builder).unwrap(),
    )
    .expect("dynamic body creation should succeed");

    let polygon = boxdd::shapes::box_polygon(0.5_f32, 0.5).expect("valid polygon geometry");
    let shape_id = boxdd::World::body(&mut world, static_body)
        .expect("static body should remain valid")
        .create_polygon(&boxdd::ShapeDef::default(), &polygon)
        .expect("polygon shape creation should succeed");

    let chain_builder = boxdd::ChainDefBuilder::points(
        boxdd::ChainDef::builder(),
        [
            [-2.0_f32, 0.0],
            [-1.0, 0.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
        ],
    );
    let chain_def = boxdd::ChainDefBuilder::build(chain_builder).unwrap();
    let updated_material =
        boxdd::SurfaceMaterial::with_friction(boxdd::SurfaceMaterial::default(), 0.75)
            .expect("updated material should be valid");

    let (attached_shapes, chain_id) = {
        let mut body: boxdd::Body<'_> =
            boxdd::World::body(&mut world, static_body).expect("static body should remain valid");
        let attached_shapes = boxdd::Body::shapes(&body).expect("shape enumeration should succeed");
        let chain_id = boxdd::Body::create_chain(&mut body, &chain_def)
            .expect("chain creation should succeed");
        (attached_shapes, chain_id)
    };

    let (observed_material, segment_count) = {
        let mut chain: boxdd::Chain<'_> =
            boxdd::World::chain(&mut world, chain_id).expect("chain should remain valid");
        boxdd::Chain::set_surface_material(&mut chain, 0, &updated_material)
            .expect("material update should succeed");
        let observed_material =
            boxdd::Chain::surface_material(&chain, 0).expect("material query should succeed");
        let segment_count =
            boxdd::Chain::segment_count(&chain).expect("segment count should succeed");
        (observed_material, segment_count)
    };

    let joint_base = world.joint_base(static_body, dynamic_body).unwrap();
    let joint_def = boxdd::DistanceJointDef::length(boxdd::DistanceJointDef::new(joint_base), 2.0);
    let joint_id = boxdd::World::create_distance_joint(&mut world, &joint_def)
        .expect("joint creation should succeed");
    let joint_type = {
        let joint: boxdd::Joint<'_> =
            boxdd::World::joint(&mut world, joint_id).expect("joint should remain valid");
        boxdd::Joint::joint_type(&joint).expect("joint type should be available")
    };
    std::mem::drop(world);

    assert_eq!(attached_shapes, [shape_id]);
    assert_eq!(observed_material, updated_material);
    assert_eq!(segment_count, 2);
    assert_eq!(joint_type, boxdd::JointType::Distance);
}
