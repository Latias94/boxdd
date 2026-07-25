use boxdd::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct DropOwnedDuringDraw {
    body: Option<OwnedBody>,
    shape: Option<OwnedShape>,
    joint: Option<OwnedJoint>,
    called: bool,
}

struct BoundaryTrackedDrop(Arc<AtomicBool>);

impl Drop for BoundaryTrackedDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct DropWorldDuringDraw {
    world: Option<World>,
    payload_dropped: Arc<AtomicBool>,
    called: bool,
}

impl DebugDraw for DropWorldDuringDraw {
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
        drop(self.world.take());
        assert!(!self.payload_dropped.load(Ordering::SeqCst));
    }
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
fn debug_draw_defers_another_world_teardown_until_the_native_callback_returns() {
    let mut drawing_world = World::new(WorldDef::default()).unwrap();
    let body =
        drawing_world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    drawing_world.create_polygon_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::box_polygon(0.5, 0.5),
    );

    let payload_dropped = Arc::new(AtomicBool::new(false));
    let mut doomed_world = World::new(WorldDef::default()).unwrap();
    let doomed_raw = doomed_world.world_id_raw();
    doomed_world.set_user_data(BoundaryTrackedDrop(Arc::clone(&payload_dropped)));
    let mut drawer = DropWorldDuringDraw {
        world: Some(doomed_world),
        payload_dropped: Arc::clone(&payload_dropped),
        called: false,
    };

    drawing_world.debug_draw(&mut drawer, DebugDrawOptions::default());

    assert!(drawer.called);
    assert!(payload_dropped.load(Ordering::SeqCst));
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(doomed_raw) });
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

#[test]
fn panicking_event_view_flushes_owned_destroys_before_resuming() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();

    let result = catch_unwind(AssertUnwindSafe(|| {
        world.with_body_events_view(move |_| {
            drop(body);
            panic!("intentional event-view panic");
        });
    }));

    assert!(result.is_err());
    assert_eq!(
        world.try_body_position(body_id),
        Err(ApiError::InvalidBodyId)
    );
}

struct PanicOnDrop;

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        panic!("intentional user-data drop panic");
    }
}

struct FoundationAwareDrop(Arc<AtomicBool>);

impl Drop for FoundationAwareDrop {
    fn drop(&mut self) {
        assert_eq!(boxdd::foundation().activity().ordinary_worlds, 1);
        let _ = boxdd::version();
        self.0.store(true, Ordering::SeqCst);
    }
}

#[test]
fn typed_user_data_access_defers_world_teardown_until_payload_release() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let raw = world.world_id_raw();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let payload_dropped = Arc::new(AtomicBool::new(false));
    body.set_user_data(FoundationAwareDrop(Arc::clone(&payload_dropped)));
    let mut world = Some(world);

    assert_eq!(
        body.with_user_data::<FoundationAwareDrop, _>(|_| {
            drop(world.take());
            assert!(unsafe { boxdd_sys::ffi::b2World_IsValid(raw) });
            assert_eq!(boxdd::foundation().activity().ordinary_worlds, 1);
            assert!(!payload_dropped.load(Ordering::SeqCst));
        }),
        Some(())
    );

    assert!(payload_dropped.load(Ordering::SeqCst));
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(raw) });
    assert_eq!(boxdd::foundation().activity().ordinary_worlds, 0);
}

#[test]
fn deferred_destroy_drains_remaining_items_after_user_data_drop_panics() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let mut first = world.create_circle_shape_for_owned(body, &ShapeDef::default(), &circle);
    let second = world.create_circle_shape_for_owned(body, &ShapeDef::default(), &circle);
    let first_id = first.id();
    let second_id = second.id();
    first.set_user_data(PanicOnDrop);

    let result = catch_unwind(AssertUnwindSafe(|| {
        world.with_body_events_view(move |_| {
            drop(first);
            drop(second);
        });
    }));

    assert!(result.is_err());
    assert_eq!(
        world.try_shape_aabb(first_id),
        Err(ApiError::InvalidShapeId)
    );
    assert_eq!(
        world.try_shape_aabb(second_id),
        Err(ApiError::InvalidShapeId)
    );
}

#[test]
fn query_preserves_primary_panic_while_flushing_another_world() {
    let mut query_world = World::new(WorldDef::default()).unwrap();
    let query_body = query_world.create_body_id(BodyBuilder::new().build());
    query_world.create_circle_shape_for(
        query_body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let mut affected_world = World::new(WorldDef::default()).unwrap();
    let mut affected_body =
        affected_world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let affected_id = affected_body.id();
    affected_body.set_user_data(PanicOnDrop);
    let mut affected_body = Some(affected_body);

    let result = catch_unwind(AssertUnwindSafe(|| {
        query_world.visit_overlap_aabb(
            Position::ZERO,
            Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]),
            QueryFilter::default(),
            |_| {
                drop(affected_body.take());
                panic!("primary query panic");
            },
        );
    }));

    let payload = result.expect_err("query callback must resume its panic");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"primary query panic")
    );
    assert_eq!(
        affected_world.try_body_position(affected_id),
        Err(ApiError::InvalidBodyId)
    );
}

#[test]
fn query_defers_another_world_teardown_until_the_native_callback_returns() {
    let mut query_world = World::new(WorldDef::default()).unwrap();
    let query_body = query_world.create_body_id(BodyBuilder::new().build());
    query_world.create_circle_shape_for(
        query_body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let payload_dropped = Arc::new(AtomicBool::new(false));
    let mut doomed_world = World::new(WorldDef::default()).unwrap();
    let doomed_raw = doomed_world.world_id_raw();
    doomed_world.set_user_data(BoundaryTrackedDrop(Arc::clone(&payload_dropped)));
    let mut doomed_world = Some(doomed_world);

    let completed = query_world.visit_overlap_aabb(
        Position::ZERO,
        Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]),
        QueryFilter::default(),
        |_| {
            drop(doomed_world.take());
            assert!(!payload_dropped.load(Ordering::SeqCst));
            false
        },
    );

    assert!(!completed);
    assert!(payload_dropped.load(Ordering::SeqCst));
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(doomed_raw) });
}

#[test]
fn query_preserves_visitor_panic_when_native_guard_triggers_panicking_world_teardown() {
    let mut query_world = World::new(WorldDef::default()).unwrap();
    let query_body = query_world.create_body_id(BodyBuilder::new().build());
    query_world.create_circle_shape_for(
        query_body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let callback_drop = PanicOnDrop;
    query_world.set_custom_filter(move |_, _| {
        let _keep_capture_alive = &callback_drop;
        true
    });
    let query_raw = query_world.world_id_raw();
    let query_handle = query_world.handle();
    let mut query_world = Some(query_world);

    let mut affected_world = World::new(WorldDef::default()).unwrap();
    let affected_body =
        affected_world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let affected_id = affected_body.id();
    let mut affected_body = Some(affected_body);

    let result = catch_unwind(AssertUnwindSafe(|| {
        query_handle.visit_overlap_aabb(
            Position::ZERO,
            Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]),
            QueryFilter::default(),
            |_| {
                drop(affected_body.take());
                drop(query_world.take());
                panic!("primary visitor panic");
            },
        );
    }));

    let payload = result.expect_err("query visitor must resume its original panic");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"primary visitor panic")
    );
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(query_raw) });
    assert_eq!(
        affected_world.try_body_position(affected_id),
        Err(ApiError::InvalidBodyId)
    );
}
