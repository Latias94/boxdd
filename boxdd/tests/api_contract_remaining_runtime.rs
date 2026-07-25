#[test]
fn remaining_runtime_paths_have_direct_ufcs_evidence() {
    let mut world: boxdd::World =
        boxdd::World::new(boxdd::WorldDef::default()).expect("world creation should succeed");

    let static_body_def = boxdd::BodyBuilder::build(boxdd::BodyBuilder::new());
    let static_body = boxdd::World::create_body_id(&mut world, static_body_def);
    let dynamic_body_builder =
        boxdd::BodyBuilder::body_type(boxdd::BodyBuilder::new(), boxdd::BodyType::Dynamic);
    let dynamic_body_builder = boxdd::BodyBuilder::position(dynamic_body_builder, [2.0_f32, 0.0]);
    let dynamic_body =
        boxdd::World::create_body_id(&mut world, boxdd::BodyBuilder::build(dynamic_body_builder));

    let polygon = boxdd::shapes::box_polygon(0.5_f32, 0.5);
    let shape_id = boxdd::World::create_polygon_shape_for(
        &mut world,
        static_body,
        &boxdd::ShapeDef::default(),
        &polygon,
    );

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
    let chain_def = boxdd::ChainDefBuilder::build(chain_builder);
    let updated_material =
        boxdd::SurfaceMaterial::with_friction(boxdd::SurfaceMaterial::default(), 0.75);

    let mut body: boxdd::Body<'_> =
        boxdd::World::body(&mut world, static_body).expect("static body should remain valid");
    let attached_shapes = boxdd::Body::shapes(&body);
    let mut chain: boxdd::Chain<'_> = boxdd::Body::create_chain(&mut body, &chain_def);
    boxdd::Chain::set_surface_material(&mut chain, 0, &updated_material);
    let observed_material = boxdd::Chain::surface_material(&chain, 0);
    let segment_count = boxdd::Chain::segment_count(&chain);
    std::mem::drop(chain);
    std::mem::drop(body);

    let joint_base = boxdd::JointBase::new(static_body, dynamic_body);
    let joint_def = boxdd::DistanceJointDef::length(boxdd::DistanceJointDef::new(joint_base), 2.0);
    let joint: boxdd::Joint<'_> = boxdd::World::create_distance_joint(&mut world, &joint_def);
    let joint_valid = boxdd::Joint::is_valid(&joint);
    std::mem::drop(joint);
    std::mem::drop(world);

    assert_eq!(attached_shapes, [shape_id]);
    assert_eq!(observed_material, updated_material);
    assert_eq!(segment_count, 2);
    assert!(joint_valid);
}
