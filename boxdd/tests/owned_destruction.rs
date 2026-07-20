use boxdd::prelude::*;

struct DropOwnedDuringDraw {
    body: Option<OwnedBody>,
    shape: Option<OwnedShape>,
    joint: Option<OwnedJoint>,
    called: bool,
}

impl DebugDraw for DropOwnedDuringDraw {
    fn draw_solid_polygon(
        &mut self,
        _transform: WorldTransform,
        _vertices: &[Vec2],
        _radius: f32,
        _color: HexColor,
    ) {
        if self.called {
            return;
        }
        self.called = true;
        drop(self.body.take());
        drop(self.shape.take());
        drop(self.joint.take());
    }
}

#[test]
fn debug_draw_callback_drops_owned_objects_without_object_ffi_reentry() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let shape_def = ShapeDef::builder().density(1.0).build();
    let polygon = shapes::box_polygon(0.5, 0.5);

    let body = world.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-3.0_f32, 0.0])
            .build(),
    );
    let body_id = body.id();
    world.create_polygon_shape_for(body_id, &shape_def, &polygon);

    let shape_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .build(),
    );
    let shape = world.create_polygon_shape_for_owned(shape_body, &shape_def, &polygon);
    let shape_id = shape.id();

    let joint_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([2.0_f32, 0.0])
            .build(),
    );
    let joint = world.create_distance_joint_owned(
        &DistanceJointDef::new(JointBase::new(shape_body, joint_body)).length(2.0),
    );
    let joint_id = joint.id();

    let mut drawer = DropOwnedDuringDraw {
        body: Some(body),
        shape: Some(shape),
        joint: Some(joint),
        called: false,
    };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());

    assert!(drawer.called);
    assert_eq!(
        world.try_body_position(body_id),
        Err(ApiError::InvalidBodyId)
    );
    assert_eq!(
        world.try_shape_aabb(shape_id),
        Err(ApiError::InvalidShapeId)
    );
    assert_eq!(
        world.try_joint_type(joint_id),
        Err(ApiError::InvalidJointId)
    );
}

#[test]
fn reentrant_explicit_destroy_remains_pending_until_user_data_is_released() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let mut shape = world.create_circle_shape_for_owned(
        body_id,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    shape.set_user_data(7_u32);

    assert_eq!(
        shape.with_user_data::<u32, _>(|value| {
            assert_eq!(*value, 7);
            body.destroy();
            world.flush_deferred_destroys();
            assert!(world.try_body_position(body_id).is_ok());
        }),
        Some(())
    );

    world.flush_deferred_destroys();
    assert_eq!(
        world.try_body_position(body_id),
        Err(ApiError::InvalidBodyId)
    );
}
