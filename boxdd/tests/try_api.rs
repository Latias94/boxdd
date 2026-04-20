use boxdd::prelude::*;
use boxdd::shapes;

#[test]
fn try_body_position_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    world.destroy_body_id(body);

    let err = world.try_body_position(body).unwrap_err();
    assert_eq!(err, ApiError::InvalidBodyId);
}

#[test]
fn try_set_body_name_rejects_interior_nul() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    let err = world.try_set_body_name(body, "a\0b").unwrap_err();
    assert_eq!(err, ApiError::NulByteInString);
}

#[test]
fn try_calls_from_debug_draw_return_in_callback() {
    struct Drawer {
        body: OwnedBody,
        err: Option<ApiError>,
    }
    impl DebugDraw for Drawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::Transform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: u32,
        ) {
            self.err = Some(self.body.try_position().unwrap_err());
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body_id, &sdef, &poly);

    let mut drawer = Drawer { body, err: None };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());
    assert_eq!(drawer.err, Some(ApiError::InCallback));
}

#[test]
fn try_query_calls_from_debug_draw_return_in_callback() {
    struct Drawer {
        world: WorldHandle,
        errs: Vec<ApiError>,
    }
    impl DebugDraw for Drawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::Transform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: u32,
        ) {
            if !self.errs.is_empty() {
                return;
            }
            let aabb = Aabb::from_center_half_extents([0.0, 1.0], [10.0, 10.0]);
            let mut overlap_ids = Vec::new();
            let mut ray_hits = Vec::new();
            self.errs.push(
                self.world
                    .try_overlap_aabb(aabb, QueryFilter::default())
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_overlap_aabb_into(aabb, QueryFilter::default(), &mut overlap_ids)
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_cast_ray_closest([0.0, 5.0], [0.0, -10.0], QueryFilter::default())
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_cast_ray_all_into(
                        [0.0, 5.0],
                        [0.0, -10.0],
                        QueryFilter::default(),
                        &mut ray_hits,
                    )
                    .unwrap_err(),
            );
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body_id, &sdef, &poly);

    let mut drawer = Drawer {
        world: world.handle(),
        errs: Vec::new(),
    };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());
    assert_eq!(
        drawer.errs,
        vec![
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
        ]
    );
}

#[test]
fn try_create_chain_invalid_def_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]])
        .build();

    let err = world.try_create_chain_for_id(body, &def).unwrap_err();
    assert_eq!(err, ApiError::InvalidChainDef);
}

#[test]
fn try_body_mutations_from_debug_draw_return_in_callback() {
    struct Drawer {
        body: OwnedBody,
        errs: Vec<ApiError>,
    }
    impl DebugDraw for Drawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::Transform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: u32,
        ) {
            if !self.errs.is_empty() {
                return;
            }
            self.errs.push(
                self.body
                    .try_apply_force_to_center([0.0, 1.0], true)
                    .unwrap_err(),
            );
            self.errs.push(
                self.body
                    .try_set_target_transform(boxdd::Transform::IDENTITY, 1.0 / 60.0, true)
                    .unwrap_err(),
            );
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body_id, &sdef, &poly);

    let mut drawer = Drawer { body, errs: vec![] };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());
    assert_eq!(
        drawer.errs,
        vec![ApiError::InCallback, ApiError::InCallback]
    );
}

#[test]
fn try_owned_body_mutation_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    world.destroy_body_id(body_id);

    let err = body
        .try_apply_force_to_center([0.0, 1.0], true)
        .unwrap_err();
    assert_eq!(err, ApiError::InvalidBodyId);
}

#[test]
fn try_set_body_target_transform_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    world.destroy_body_id(body);

    let err = world
        .try_set_body_target_transform(body, boxdd::Transform::IDENTITY, 1.0 / 60.0, true)
        .unwrap_err();
    assert_eq!(err, ApiError::InvalidBodyId);
}

#[test]
fn try_body_mass_data_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    world.destroy_body_id(body);

    let err = world.try_body_mass_data(body).unwrap_err();
    assert_eq!(err, ApiError::InvalidBodyId);
}

#[test]
fn try_owned_shape_mutation_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let mut shape = world.create_polygon_shape_for_owned(body_id, &sdef, &poly);
    let shape_id = shape.id();
    world.destroy_shape_id(shape_id, true);

    let err = shape.try_set_friction(0.5).unwrap_err();
    assert_eq!(err, ApiError::InvalidShapeId);
}

#[test]
fn try_owned_chain_mutation_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]])
        .build();
    let mut chain = world.create_chain_for_owned(body_id, &def);
    let chain_id = chain.id();
    world.destroy_chain_id(chain_id);

    let err = chain
        .try_set_surface_material(0, &boxdd::shapes::SurfaceMaterial::default())
        .unwrap_err();
    assert_eq!(err, ApiError::InvalidChainId);
}

#[test]
fn try_owned_joint_mutation_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let mut joint = world.revolute(a, b).anchor_world([0.0, 0.0]).build_owned();
    let joint_id = joint.id();
    world.destroy_joint_id(joint_id, true);

    let err = joint.try_set_force_threshold(10.0).unwrap_err();
    assert_eq!(err, ApiError::InvalidJointId);
}

#[test]
fn try_joint_runtime_controls_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let j = world.revolute(a, b).anchor_world([0.0, 0.0]).build().id();
    world.destroy_joint_id(j, true);

    let err = world.try_revolute_set_motor_speed(j, 1.0).unwrap_err();
    assert_eq!(err, ApiError::InvalidJointId);
}

#[test]
fn try_create_joint_invalid_body_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().build());
    let b = world.create_body_id(BodyBuilder::new().build());
    world.destroy_body_id(a);

    let base = JointBaseBuilder::new()
        .bodies_by_id(a, b)
        .collide_connected(false)
        .build();
    let def = DistanceJointDef::new(base);

    let err = world.try_create_distance_joint_id(&def).unwrap_err();
    assert_eq!(err, ApiError::InvalidBodyId);
}

#[test]
fn try_build_joint_builder_invalid_body_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().build());
    let b = world.create_body_id(BodyBuilder::new().build());
    world.destroy_body_id(a);

    let err = world
        .revolute(a, b)
        .anchor_world([0.0, 0.0])
        .try_build()
        .err()
        .unwrap();
    assert_eq!(err, ApiError::InvalidBodyId);
}
