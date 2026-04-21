use boxdd::prelude::*;
use boxdd::shapes;
use boxdd_sys::ffi;

fn shape_id_fields(id: ShapeId) -> (i32, u16, u16) {
    (id.index1, id.world0, id.generation)
}

fn contact_id_fields(id: ContactId) -> (i32, u16, i16, u32) {
    (id.index1, id.world0, id.padding, id.generation)
}

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
            _color: HexColor,
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
            _color: HexColor,
        ) {
            if !self.errs.is_empty() {
                return;
            }
            let aabb = Aabb::from_center_half_extents([0.0, 1.0], [10.0, 10.0]);
            let mut overlap_ids = Vec::new();
            let mut ray_hits = Vec::new();
            let mut mover_planes = Vec::new();
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
                    .try_visit_overlap_aabb(aabb, QueryFilter::default(), |_| true)
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_visit_overlap_polygon_points(
                        [[-0.5_f32, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
                        0.0,
                        QueryFilter::default(),
                        |_| true,
                    )
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_visit_overlap_polygon_points_with_offset(
                        [[-0.5_f32, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
                        0.0,
                        [0.0_f32, 1.0],
                        0.0_f32,
                        QueryFilter::default(),
                        |_| true,
                    )
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
            self.errs.push(
                self.world
                    .try_cast_mover(
                        [0.0_f32, 0.75],
                        [0.0, 1.75],
                        0.25,
                        [1.0_f32, 0.0],
                        QueryFilter::default(),
                    )
                    .unwrap_err(),
            );
            self.errs.push(
                self.world
                    .try_collide_mover_into(
                        [0.0_f32, 0.75],
                        [0.0, 1.75],
                        0.25,
                        QueryFilter::default(),
                        &mut mover_planes,
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
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
        ]
    );
}

#[test]
fn try_calls_from_debug_draw_raw_return_in_callback() {
    struct Drawer {
        body: OwnedBody,
        err: Option<ApiError>,
    }
    impl RawDebugDraw for Drawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: ffi::b2Transform,
            _vertices: &[ffi::b2Vec2],
            _radius: f32,
            _color: HexColor,
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
    world.debug_draw_raw(&mut drawer, DebugDrawOptions::default());
    assert_eq!(drawer.err, Some(ApiError::InCallback));
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
            _color: HexColor,
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
fn try_body_runtime_helpers_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let handle = world.handle();
    let body_id = body.id();
    world.destroy_body_id(body_id);

    let mut shape_ids = Vec::new();
    let mut joint_ids = Vec::new();

    assert_eq!(body.try_rotation().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_linear_velocity().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_angular_velocity().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_rotation_raw().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(body.try_aabb().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_is_sleep_enabled().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_sleep_threshold().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_set_sleep_threshold(0.5).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(body.try_is_awake().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_set_awake(false).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(body.try_is_enabled().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(body.try_enable().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(body.try_disable().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(body.try_is_bullet().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_set_bullet(true).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(body.try_name().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(body.try_shape_count().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_shapes_into(&mut shape_ids).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(body.try_joint_count().unwrap_err(), ApiError::InvalidBodyId);
    assert_eq!(
        body.try_joints_into(&mut joint_ids).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_enable_contact_events(true).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        body.try_enable_hit_events(true).unwrap_err(),
        ApiError::InvalidBodyId
    );

    assert_eq!(
        world.try_body_rotation(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_linear_velocity(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_angular_velocity(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_aabb(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_is_sleep_enabled(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_sleep_threshold(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_is_awake(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_set_body_awake(body_id, false).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_is_enabled(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_enable_body(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_disable_body(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_is_bullet(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_set_body_bullet(body_id, true).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_name(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world
            .try_set_body_sleep_threshold(body_id, 0.5)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_shape_count(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world
            .try_body_shapes_into(body_id, &mut shape_ids)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_joint_count(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world
            .try_body_joints_into(body_id, &mut joint_ids)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world
            .try_body_enable_contact_events(body_id, true)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        world.try_body_enable_hit_events(body_id, true).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        handle.try_body_transform(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        handle.try_body_mass_data(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        handle
            .try_body_shapes_into(body_id, &mut shape_ids)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        handle.try_body_motion_locks(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        handle.try_body_name(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
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
fn try_shape_runtime_helpers_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let handle = world.handle();
    let sdef = ShapeDef::builder().density(1.0).build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let mut shape = world.create_circle_shape_for_owned(body_id, &sdef, &circle);
    let shape_id = shape.id();
    world.destroy_shape_id(shape_id, true);
    let mut overlap_ids = Vec::new();

    assert_eq!(shape.try_aabb().unwrap_err(), ApiError::InvalidShapeId);
    assert_eq!(
        shape.try_test_point([0.0_f32, 0.0]).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        shape.try_ray_cast([-1.0_f32, 0.0], [2.0, 0.0]).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(shape.try_mass_data().unwrap_err(), ApiError::InvalidShapeId);
    assert_eq!(
        shape.try_sensor_events_enabled().unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        shape.try_enable_contact_events(true).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        shape.try_sensor_overlaps_valid().unwrap_err(),
        ApiError::InvalidShapeId
    );

    assert_eq!(
        world.try_shape_aabb(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world
            .try_shape_test_point(shape_id, [0.0_f32, 0.0])
            .unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world
            .try_shape_ray_cast(shape_id, [-1.0_f32, 0.0], [2.0, 0.0])
            .unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world.try_shape_mass_data(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world.try_shape_sensor_events_enabled(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world
            .try_shape_enable_contact_events(shape_id, true)
            .unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        world.try_shape_sensor_overlaps_valid(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        handle.try_shape_surface_material(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        handle.try_shape_aabb(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        handle.try_shape_mass_data(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        handle.try_shape_hit_events_enabled(shape_id).unwrap_err(),
        ApiError::InvalidShapeId
    );
    assert_eq!(
        handle
            .try_shape_sensor_overlaps_into(shape_id, &mut overlap_ids)
            .unwrap_err(),
        ApiError::InvalidShapeId
    );
}

#[test]
fn try_shape_numeric_mutation_invalid_values_return_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let mut shape = world.create_polygon_shape_for_owned(body_id, &sdef, &poly);

    assert_eq!(
        shape.try_set_density(-1.0, true).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shape.try_set_friction(f32::NAN).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shape.try_set_restitution(-0.25).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn try_body_numeric_mutation_invalid_values_return_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    assert_eq!(
        body.try_set_mass_data(MassData {
            mass: -1.0,
            center: Vec2::ZERO,
            rotational_inertia: 0.0,
        })
        .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        body.try_set_gravity_scale(f32::NAN).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        body.try_set_linear_damping(-0.5).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        body.try_set_angular_damping(f32::NAN).unwrap_err(),
        ApiError::InvalidArgument
    );
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
fn try_owned_chain_material_index_out_of_range_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let materials = [
        SurfaceMaterial::default().with_friction(0.05),
        SurfaceMaterial::default().with_friction(0.10),
        SurfaceMaterial::default().with_friction(0.20),
        SurfaceMaterial::default().with_friction(0.30),
        SurfaceMaterial::default().with_friction(0.40),
        SurfaceMaterial::default().with_friction(0.50),
        SurfaceMaterial::default().with_friction(0.60),
    ];
    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([
            [-3.0_f32, 0.0],
            [-2.0, 0.0],
            [-1.0, 0.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [3.0, 0.0],
        ])
        .materials(&materials)
        .build();
    let mut chain = world.create_chain_for_owned(body_id, &def);

    assert_eq!(chain.surface_material_count(), 4);
    assert_eq!(
        chain.try_surface_material(4).unwrap_err(),
        ApiError::IndexOutOfRange
    );
    assert_eq!(
        chain
            .try_set_surface_material(4, &SurfaceMaterial::default())
            .unwrap_err(),
        ApiError::IndexOutOfRange
    );
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
fn try_joint_range_mutation_invalid_arguments_return_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );

    let mut distance = world.create_distance_joint_owned(
        &DistanceJointDef::new(JointBaseBuilder::new().bodies_by_id(body_a, body_b).build())
            .length(1.0),
    );
    assert_eq!(
        distance
            .try_distance_set_spring_force_range(2.0, 1.0)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let base = world.joint_base_from_world_with_axis(
        body_a,
        body_b,
        [0.0_f32, 0.0],
        [1.0_f32, 0.0],
        [1.0_f32, 0.0],
    );

    let prismatic = world.create_prismatic_joint_owned(&PrismaticJointDef::new(base.clone()));
    let prismatic_id = prismatic.id();
    assert_eq!(
        world
            .try_prismatic_set_limits(prismatic_id, 1.0, -1.0)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let revolute = world.create_revolute_joint_owned(&RevoluteJointDef::new(base.clone()));
    let revolute_id = revolute.id();
    assert_eq!(
        world
            .try_revolute_set_limits(revolute_id, -4.0, 0.0)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let wheel = world.create_wheel_joint_owned(&WheelJointDef::new(base));
    let wheel_id = wheel.id();
    assert_eq!(
        world.try_wheel_set_limits(wheel_id, 0.5, -0.5).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn try_joint_runtime_helpers_invalid_id_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let handle = world.handle();
    let mut joint = world.create_distance_joint_owned(
        &DistanceJointDef::new(
            JointBaseBuilder::new()
                .bodies_by_id(a, b)
                .constraint_hertz(2.0)
                .constraint_damping_ratio(0.3)
                .build(),
        )
        .length(1.0),
    );
    let joint_id = joint.id();
    world.destroy_joint_id(joint_id, true);

    assert_eq!(
        joint.try_joint_type().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_joint_type_raw().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(joint.try_body_a_id().unwrap_err(), ApiError::InvalidJointId);
    assert_eq!(joint.try_body_b_id().unwrap_err(), ApiError::InvalidJointId);
    assert_eq!(
        joint.try_collide_connected().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_set_collide_connected(true).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_constraint_tuning().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint
            .try_set_constraint_tuning(ConstraintTuning::new(4.0, 0.5))
            .unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_local_frame_a().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_local_frame_b().unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        joint.try_wake_bodies().unwrap_err(),
        ApiError::InvalidJointId
    );

    assert_eq!(
        world.try_joint_type(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_type_raw(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_body_a_id(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_body_b_id(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_collide_connected(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world
            .try_set_joint_collide_connected(joint_id, true)
            .unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_constraint_tuning(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world
            .try_set_joint_constraint_tuning(joint_id, ConstraintTuning::new(4.0, 0.5))
            .unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_local_frame_a(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_local_frame_b(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_wake_bodies(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_force_threshold(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world
            .try_set_joint_force_threshold(joint_id, 4.0)
            .unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_torque_threshold(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        world
            .try_set_joint_torque_threshold(joint_id, 5.0)
            .unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_joint_type(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_joint_constraint_tuning(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_joint_local_frame_a(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_joint_force_threshold(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_joint_torque_threshold(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_distance_length(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_prismatic_spring_enabled(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
    assert_eq!(
        handle.try_motor_linear_velocity(joint_id).unwrap_err(),
        ApiError::InvalidJointId
    );
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
fn try_joint_runtime_controls_wrong_family_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let mut joint = world
        .revolute(a, b)
        .anchor_world([0.0_f32, 0.0])
        .build_owned();
    let joint_id = joint.id();
    let handle = world.handle();

    assert_eq!(
        world.try_distance_length(joint_id).unwrap_err(),
        ApiError::InvalidJointType
    );
    assert_eq!(
        world.try_distance_set_length(joint_id, 1.0).unwrap_err(),
        ApiError::InvalidJointType
    );
    assert_eq!(
        joint.try_distance_length().unwrap_err(),
        ApiError::InvalidJointType
    );
    assert_eq!(
        joint.try_distance_set_length(1.0).unwrap_err(),
        ApiError::InvalidJointType
    );
    assert_eq!(
        handle.try_distance_length(joint_id).unwrap_err(),
        ApiError::InvalidJointType
    );
    assert_eq!(
        handle.try_motor_linear_velocity(joint_id).unwrap_err(),
        ApiError::InvalidJointType
    );

    {
        let mut scoped = world.joint(joint_id).expect("joint should still be valid");
        assert_eq!(
            scoped.try_distance_length().unwrap_err(),
            ApiError::InvalidJointType
        );
        assert_eq!(
            scoped.try_distance_set_length(1.0).unwrap_err(),
            ApiError::InvalidJointType
        );
    }
}

#[test]
fn try_contact_id_helpers_cover_invalid_and_live_contacts() {
    let invalid = ContactId::from_raw(ffi::b2ContactId {
        index1: 0,
        world0: 0,
        padding: 0,
        generation: 0,
    });
    assert!(!invalid.is_valid());
    assert!(!invalid.try_is_valid().unwrap());
    assert_eq!(invalid.try_data().unwrap_err(), ApiError::InvalidContactId);
    assert_eq!(
        invalid.try_data_raw().unwrap_err(),
        ApiError::InvalidContactId
    );

    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    let shape_a = world.create_polygon_shape_for(body_a, &sdef, &shapes::box_polygon(0.5, 0.5));
    let shape_b = world.create_polygon_shape_for(body_b, &sdef, &shapes::box_polygon(0.5, 0.5));
    world.set_body_linear_velocity(body_a, [2.0_f32, 0.0]);
    world.set_body_linear_velocity(body_b, [-2.0_f32, 0.0]);

    let mut live_contact = None;
    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
        let events = world.contact_events();
        if let Some(event) = events.begin.first() {
            live_contact = Some((event.contact_id, event.shape_a, event.shape_b));
            break;
        }
    }

    let (contact, event_shape_a, event_shape_b) =
        live_contact.expect("expected a live contact id from contact begin events");
    assert!(contact.is_valid());
    assert!(contact.try_is_valid().unwrap());

    let data = contact.data();
    let data_try = contact.try_data().unwrap();
    let raw = contact.data_raw();
    let raw_try = contact.try_data_raw().unwrap();
    let mut expected_shapes = [shape_id_fields(shape_a), shape_id_fields(shape_b)];
    let mut event_shapes = [
        shape_id_fields(event_shape_a),
        shape_id_fields(event_shape_b),
    ];
    expected_shapes.sort();
    event_shapes.sort();

    assert_eq!(
        contact_id_fields(data.contact_id),
        contact_id_fields(contact)
    );
    assert_eq!(
        contact_id_fields(data_try.contact_id),
        contact_id_fields(contact)
    );
    assert_eq!(
        contact_id_fields(ContactId::from_raw(raw.contactId)),
        contact_id_fields(contact)
    );
    assert_eq!(
        contact_id_fields(ContactId::from_raw(raw_try.contactId)),
        contact_id_fields(contact)
    );

    assert_eq!(event_shapes, expected_shapes);
    assert_eq!(
        shape_id_fields(data.shape_id_a),
        shape_id_fields(event_shape_a)
    );
    assert_eq!(
        shape_id_fields(data.shape_id_b),
        shape_id_fields(event_shape_b)
    );
    assert_eq!(
        shape_id_fields(data_try.shape_id_a),
        shape_id_fields(event_shape_a)
    );
    assert_eq!(
        shape_id_fields(data_try.shape_id_b),
        shape_id_fields(event_shape_b)
    );
    assert_eq!(
        shape_id_fields(ShapeId::from_raw(raw.shapeIdA)),
        shape_id_fields(event_shape_a)
    );
    assert_eq!(
        shape_id_fields(ShapeId::from_raw(raw.shapeIdB)),
        shape_id_fields(event_shape_b)
    );
    assert_eq!(
        shape_id_fields(ShapeId::from_raw(raw_try.shapeIdA)),
        shape_id_fields(event_shape_a)
    );
    assert_eq!(
        shape_id_fields(ShapeId::from_raw(raw_try.shapeIdB)),
        shape_id_fields(event_shape_b)
    );
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
fn try_create_body_invalid_def_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let def = BodyBuilder::new().linear_damping(f32::NAN).build();

    let err = world.try_create_body_id(def).unwrap_err();
    assert_eq!(err, ApiError::InvalidArgument);
}

#[test]
fn try_create_joint_invalid_def_returns_err() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let a = world.create_body_id(BodyBuilder::new().build());
    let b = world.create_body_id(BodyBuilder::new().build());

    let same_body_base = JointBaseBuilder::new().bodies_by_id(a, a).build();
    let same_body_def = DistanceJointDef::new(same_body_base).length(1.0);
    assert_eq!(
        world
            .try_create_distance_joint_id(&same_body_def)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let invalid_threshold_base = JointBaseBuilder::new()
        .bodies_by_id(a, b)
        .force_threshold(-1.0)
        .build();
    let invalid_threshold_def = MotorJointDef::new(invalid_threshold_base);
    assert_eq!(
        world
            .try_create_motor_joint_id(&invalid_threshold_def)
            .unwrap_err(),
        ApiError::InvalidArgument
    );
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
