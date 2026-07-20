use std::panic::{AssertUnwindSafe, catch_unwind};

use boxdd::prelude::*;
use boxdd::shapes::{self, chain::ChainDef};

#[derive(Copy, Clone)]
struct ObjectIds {
    body_a: BodyId,
    body_b: BodyId,
    shape: ShapeId,
    joint: JointId,
    chain: ChainId,
}

fn new_world() -> World {
    World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap()
}

fn chain_def() -> ChainDef {
    ChainDef::builder()
        .points([[-2.0_f32, -10.0], [-1.0, -10.0], [1.0, -10.0], [2.0, -10.0]])
        .build()
}

fn populate_world(world: &mut World, x: f32) -> ObjectIds {
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([x, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([x + 3.0, 0.0])
            .build(),
    );
    let shape = world.create_circle_shape_for(
        body_a,
        &ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let joint =
        world.create_distance_joint_id(&DistanceJointDef::new(JointBase::new(body_a, body_b)));
    let chain = world.create_chain_for_id(body_b, &chain_def());

    ObjectIds {
        body_a,
        body_b,
        shape,
        joint,
        chain,
    }
}

fn create_live_contact(world: &mut World, center_x: f32) -> (BodyId, ShapeId, ShapeId, ContactId) {
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([center_x - 1.0, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([center_x + 1.0, 0.0])
            .build(),
    );
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    let shape_a =
        world.create_polygon_shape_for(body_a, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    let shape_b =
        world.create_polygon_shape_for(body_b, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    world.set_body_linear_velocity(body_a, [2.0_f32, 0.0]);
    world.set_body_linear_velocity(body_b, [-2.0_f32, 0.0]);

    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
        if let Some(event) = world.contact_events().begin.first() {
            return (body_a, shape_a, shape_b, event.contact_id);
        }
    }

    panic!("expected a live contact id from contact begin events");
}

struct DropWorldOnInto(Option<World>);

impl From<DropWorldOnInto> for Vec2 {
    fn from(mut value: DropWorldOnInto) -> Self {
        drop(value.0.take());
        Self::ZERO
    }
}

struct DropShapeOnInto(Option<OwnedShape>);

impl From<DropShapeOnInto> for Vec2 {
    fn from(mut value: DropShapeOnInto) -> Self {
        drop(value.0.take());
        Self::ZERO
    }
}

#[test]
fn user_conversion_cannot_leave_a_stale_body_ffi_call() {
    let mut world = new_world();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    assert_eq!(
        body.try_set_linear_velocity(DropWorldOnInto(Some(world))),
        Err(ApiError::WorldDestroyed)
    );
    assert_eq!(body.try_position(), Err(ApiError::WorldDestroyed));
}

#[test]
fn query_visitor_can_drop_the_world_without_destroying_an_active_native_call() {
    let mut world = new_world();
    for x in [-1.0_f32, 1.0] {
        let body = world.create_body_id(BodyBuilder::new().position([x, 0.0]).build());
        world.create_circle_shape_for(
            body,
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5),
        );
    }
    let handle = world.handle();
    let mut owner = Some(world);
    let mut visits = 0;

    let completed = handle
        .try_visit_overlap_aabb(
            Position::ZERO,
            Aabb {
                lower: Vec2::new(-4.0, -4.0),
                upper: Vec2::new(4.0, 4.0),
            },
            QueryFilter::default(),
            |_| {
                visits += 1;
                if visits == 1 {
                    drop(owner.take());
                }
                true
            },
        )
        .unwrap();

    assert!(completed);
    assert!(visits >= 2);
    assert!(owner.is_none());
    assert_eq!(handle.try_counters(), Err(ApiError::WorldDestroyed));
}

#[test]
fn shape_ray_cast_rechecks_ownership_after_user_conversion() {
    let mut world = new_world();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let shape = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::default(),
        &shapes::circle(Vec2::ZERO, 0.5),
    );
    let shape_id = shape.id();

    assert_eq!(
        world.try_shape_ray_cast(shape_id, Position::ZERO, DropShapeOnInto(Some(shape)),),
        Err(ApiError::InvalidShapeId)
    );

    let shape_id =
        world.create_circle_shape_for(body, &ShapeDef::default(), &shapes::circle(Vec2::ZERO, 0.5));
    let handle = world.handle();
    assert_eq!(
        handle.try_shape_ray_cast(shape_id, Position::ZERO, DropWorldOnInto(Some(world)),),
        Err(ApiError::WorldDestroyed)
    );
}

#[test]
fn shape_apply_wind_rechecks_shape_after_user_conversion() {
    let mut world = new_world();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let shape = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle(Vec2::ZERO, 0.5),
    );
    let shape_id = shape.id();

    assert_eq!(
        world.try_shape_apply_wind(shape_id, DropShapeOnInto(Some(shape)), 1.0, 0.5, true,),
        Err(ApiError::InvalidShapeId)
    );
}

#[track_caller]
fn assert_error<T>(result: ApiResult<T>, expected: ApiError) {
    match result {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => panic!("expected {expected:?}, got Ok"),
    }
}

#[track_caller]
fn assert_panics(f: impl FnOnce()) {
    assert!(catch_unwind(AssertUnwindSafe(f)).is_err());
}

#[test]
fn foreign_ids_are_rejected_before_world_anchored_creation() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let source_before = source.counters();
    let target_before = target.counters();
    let shape_def = ShapeDef::builder().density(1.0).build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.25);

    assert_error(
        target.try_create_circle_shape_for(source_ids.body_a, &shape_def, &circle),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_create_chain_for_id(source_ids.body_b, &chain_def()),
        ApiError::WrongWorld,
    );
    let foreign_joint = DistanceJointDef::new(JointBase::new(source_ids.body_a, source_ids.body_b));
    assert_error(
        target.try_create_distance_joint_id(&foreign_joint),
        ApiError::WrongWorld,
    );
    let mixed_joint = DistanceJointDef::new(JointBase::new(source_ids.body_a, target_ids.body_a));
    assert_error(
        target.try_create_distance_joint_id(&mixed_joint),
        ApiError::WrongWorld,
    );

    assert_panics(|| {
        let _ = target.create_circle_shape_for(source_ids.body_a, &shape_def, &circle);
    });
    assert_panics(|| {
        let _ = target.create_chain_for_id(source_ids.body_b, &chain_def());
    });
    assert_panics(|| {
        let _ = target.create_distance_joint_id(&foreign_joint);
    });

    assert_eq!(source.counters(), source_before);
    assert_eq!(target.counters(), target_before);
    assert!(source.try_body_position(source_ids.body_a).is_ok());
    assert!(target.try_body_position(target_ids.body_a).is_ok());
}

#[test]
fn foreign_ids_cannot_read_or_control_through_world_or_world_handle() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let source_shape_events = source.shape_contact_events_enabled(source_ids.shape);
    let source_collide_connected = source.joint_collide_connected(source_ids.joint);
    let source_joint_length = source.distance_length(source_ids.joint);
    let source_velocity = source.body_linear_velocity(source_ids.body_a);

    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let source_before = source.counters();
    let target_before = target.counters();
    let target_handle = target.handle();

    assert_error(
        target.try_body_position(source_ids.body_a),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_shape_aabb(source_ids.shape),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_joint_type(source_ids.joint),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_distance_length(source_ids.joint),
        ApiError::WrongWorld,
    );
    assert_error(target.try_chain(source_ids.chain), ApiError::WrongWorld);

    assert_error(
        target_handle.try_body_position(source_ids.body_a),
        ApiError::WrongWorld,
    );
    assert_error(
        target_handle.try_shape_aabb(source_ids.shape),
        ApiError::WrongWorld,
    );
    assert_error(
        target_handle.try_joint_type(source_ids.joint),
        ApiError::WrongWorld,
    );
    assert_error(
        target_handle.try_distance_length(source_ids.joint),
        ApiError::WrongWorld,
    );

    assert_error(
        target.try_set_body_linear_velocity(source_ids.body_a, [99.0_f32, 0.0]),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_shape_enable_contact_events(source_ids.shape, !source_shape_events),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_set_joint_collide_connected(source_ids.joint, !source_collide_connected),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_distance_set_length(source_ids.joint, source_joint_length + 1.0),
        ApiError::WrongWorld,
    );

    assert_panics(|| {
        let _ = target.body_position(source_ids.body_a);
    });
    assert_panics(|| {
        let _ = target_handle.shape_aabb(source_ids.shape);
    });
    assert_panics(|| {
        let _ = target.chain(source_ids.chain);
    });

    assert_eq!(
        source.body_linear_velocity(source_ids.body_a),
        source_velocity
    );
    assert_eq!(
        source.shape_contact_events_enabled(source_ids.shape),
        source_shape_events
    );
    assert_eq!(
        source.joint_collide_connected(source_ids.joint),
        source_collide_connected
    );
    assert_eq!(
        source.distance_length(source_ids.joint),
        source_joint_length
    );
    assert_eq!(source.counters(), source_before);
    assert_eq!(target.counters(), target_before);
    assert!(target.try_body_position(target_ids.body_a).is_ok());
}

#[test]
fn foreign_destroy_is_transactional_for_both_worlds() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    {
        let mut body = source.try_body(source_ids.body_a).unwrap();
        body.try_set_user_data(String::from("source body")).unwrap();
    }
    {
        let mut shape = source.try_shape(source_ids.shape).unwrap();
        shape
            .try_set_user_data(String::from("source shape"))
            .unwrap();
    }
    {
        let mut joint = source.try_joint(source_ids.joint).unwrap();
        joint
            .try_set_user_data(String::from("source joint"))
            .unwrap();
    }
    let source_before = source.counters();
    let target_before = target.counters();

    assert_error(
        target.try_destroy_body_id(source_ids.body_a),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_destroy_shape_id(source_ids.shape, true),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_destroy_joint_id(source_ids.joint, true),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_destroy_chain_id(source_ids.chain),
        ApiError::WrongWorld,
    );

    assert_panics(|| target.destroy_body_id(source_ids.body_a));
    assert_panics(|| target.destroy_shape_id(source_ids.shape, true));
    assert_panics(|| target.destroy_joint_id(source_ids.joint, true));
    assert_panics(|| target.destroy_chain_id(source_ids.chain));

    assert_eq!(source.counters(), source_before);
    assert_eq!(target.counters(), target_before);
    assert!(source.try_body_position(source_ids.body_a).is_ok());
    assert!(source.try_shape_aabb(source_ids.shape).is_ok());
    assert!(source.try_distance_length(source_ids.joint).is_ok());
    assert!(source.try_chain(source_ids.chain).is_ok());
    assert!(target.try_body_position(target_ids.body_a).is_ok());
    assert_eq!(
        source
            .try_body(source_ids.body_a)
            .unwrap()
            .try_with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source body")
    );
    assert_eq!(
        source
            .try_shape(source_ids.shape)
            .unwrap()
            .try_with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source shape")
    );
    assert_eq!(
        source
            .try_joint(source_ids.joint)
            .unwrap()
            .try_with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source joint")
    );
}

#[test]
fn foreign_scoped_borrows_cannot_cross_into_user_data_registries() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);

    {
        let mut body = source.try_body(source_ids.body_a).unwrap();
        body.try_set_user_data(String::from("source body")).unwrap();
    }
    {
        let mut shape = source.try_shape(source_ids.shape).unwrap();
        shape
            .try_set_user_data(String::from("source shape"))
            .unwrap();
    }
    {
        let mut joint = source.try_joint(source_ids.joint).unwrap();
        joint
            .try_set_user_data(String::from("source joint"))
            .unwrap();
    }
    {
        let mut body = target.try_body(target_ids.body_a).unwrap();
        body.try_set_user_data(String::from("target body")).unwrap();
    }

    assert_error(target.try_body(source_ids.body_a), ApiError::WrongWorld);
    assert_error(target.try_shape(source_ids.shape), ApiError::WrongWorld);
    assert_error(target.try_joint(source_ids.joint), ApiError::WrongWorld);
    assert_error(target.try_chain(source_ids.chain), ApiError::WrongWorld);
    assert_panics(|| {
        let _ = target.body(source_ids.body_a);
    });

    let source_body_data = source
        .try_body(source_ids.body_a)
        .unwrap()
        .try_with_user_data::<String, _>(Clone::clone)
        .unwrap();
    let source_shape_data = source
        .try_shape(source_ids.shape)
        .unwrap()
        .try_with_user_data::<String, _>(Clone::clone)
        .unwrap();
    let source_joint_data = source
        .try_joint(source_ids.joint)
        .unwrap()
        .try_with_user_data::<String, _>(Clone::clone)
        .unwrap();
    let target_body_data = target
        .try_body(target_ids.body_a)
        .unwrap()
        .try_with_user_data::<String, _>(Clone::clone)
        .unwrap();

    assert_eq!(source_body_data.as_deref(), Some("source body"));
    assert_eq!(source_shape_data.as_deref(), Some("source shape"));
    assert_eq!(source_joint_data.as_deref(), Some("source joint"));
    assert_eq!(target_body_data.as_deref(), Some("target body"));
}

#[test]
fn owned_handle_ids_remain_bound_to_their_origin_world() {
    let mut source = new_world();
    let owned_body_a = source.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-3.0_f32, 0.0])
            .build(),
    );
    let owned_body_b = source.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([3.0_f32, 0.0])
            .build(),
    );
    let body_a = owned_body_a.id();
    let body_b = owned_body_b.id();
    let owned_shape = source.create_circle_shape_for_owned(
        body_a,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let owned_joint =
        source.create_distance_joint_owned(&DistanceJointDef::new(JointBase::new(body_a, body_b)));
    let owned_chain = source.create_chain_for_owned(body_b, &chain_def());
    let shape = owned_shape.id();
    let joint = owned_joint.id();
    let chain = owned_chain.id();

    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let target_before = target.counters();
    assert_error(target.try_body_position(body_a), ApiError::WrongWorld);
    assert_error(target.try_shape_aabb(shape), ApiError::WrongWorld);
    assert_error(target.try_distance_length(joint), ApiError::WrongWorld);
    assert_error(target.try_chain(chain), ApiError::WrongWorld);
    assert_error(target.try_destroy_body_id(body_a), ApiError::WrongWorld);
    assert_error(
        target.try_destroy_shape_id(shape, true),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_destroy_joint_id(joint, true),
        ApiError::WrongWorld,
    );
    assert_error(target.try_destroy_chain_id(chain), ApiError::WrongWorld);
    assert_eq!(target.counters(), target_before);

    drop(owned_joint);
    drop(owned_shape);
    drop(owned_chain);
    drop(owned_body_a);
    drop(owned_body_b);
    let target_after_origin_drop = target.counters();
    assert_eq!(
        target_after_origin_drop.body_count,
        target_before.body_count
    );
    assert_eq!(
        target_after_origin_drop.shape_count,
        target_before.shape_count
    );
    assert_eq!(
        target_after_origin_drop.joint_count,
        target_before.joint_count
    );
    assert_eq!(
        target_after_origin_drop.contact_count,
        target_before.contact_count
    );
    assert!(target.try_body_position(target_ids.body_a).is_ok());
}

#[test]
fn event_snapshots_and_borrowed_views_preserve_world_provenance() {
    let mut source = new_world();
    let (source_body, source_shape, _, source_contact) = create_live_contact(&mut source, -20.0);
    {
        let mut body = source.try_body(source_body).unwrap();
        body.try_set_user_data(String::from("event body")).unwrap();
    }
    {
        let mut shape = source.try_shape(source_shape).unwrap();
        shape
            .try_set_user_data(String::from("event shape"))
            .unwrap();
    }
    let source_handle = source.handle();
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let source_before = source.counters();
    let target_before = target.counters();
    let target_handle = target.handle();

    let snapshot = source_handle.contact_events();
    let begin = snapshot
        .begin
        .first()
        .expect("contact begin snapshot should remain available");
    assert_error(target.try_shape_aabb(begin.shape_a), ApiError::WrongWorld);
    assert_error(
        target_handle.try_contact_data(begin.contact_id),
        ApiError::WrongWorld,
    );

    source.with_contact_events_view(|mut begin, _, _| {
        let event = begin
            .next()
            .expect("contact begin view should remain available");
        assert_error(target.try_shape_aabb(event.shape_a()), ApiError::WrongWorld);
        assert_error(
            target_handle.try_contact_data(event.contact_id()),
            ApiError::WrongWorld,
        );
        assert_error(
            target.try_destroy_shape_id(event.shape_a(), true),
            ApiError::WrongWorld,
        );
        assert_error(
            target.try_destroy_body_id(source_body),
            ApiError::WrongWorld,
        );
    });

    let moved_body = source.with_body_events_view(|events| {
        events
            .map(|event| event.body_id())
            .find(|id| *id == source_body)
            .expect("moving source body should be present in the borrowed event view")
    });
    assert_error(target.try_body_position(moved_body), ApiError::WrongWorld);
    assert_error(
        target_handle.try_body_position(moved_body),
        ApiError::WrongWorld,
    );
    assert_error(
        target.try_contact_data(source_contact),
        ApiError::WrongWorld,
    );

    assert_eq!(source.counters(), source_before);
    assert_eq!(source.try_contact_is_valid(source_contact), Ok(true));
    assert_eq!(
        source
            .try_body(source_body)
            .unwrap()
            .try_with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("event body")
    );
    assert_eq!(
        source
            .try_shape(source_shape)
            .unwrap()
            .try_with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("event shape")
    );
    assert_eq!(target.counters(), target_before);
    assert!(target.try_body_position(target_ids.body_a).is_ok());
}

#[test]
fn raw_binding_distinguishes_foreign_and_stale_ids_for_every_object_kind() {
    let mut source = new_world();
    let ids = populate_world(&mut source, -20.0);
    let (_, contact_shape, _, contact) = create_live_contact(&mut source, 20.0);
    let raw_body = ids.body_a.unbind();
    let raw_shape = ids.shape.unbind();
    let raw_joint = ids.joint.unbind();
    let raw_chain = ids.chain.unbind();
    let raw_contact = contact.unbind();
    let foreign = new_world();

    assert_eq!(source.bind_body_id(raw_body).unwrap(), ids.body_a);
    assert_eq!(source.bind_shape_id(raw_shape).unwrap(), ids.shape);
    assert_eq!(source.bind_joint_id(raw_joint).unwrap(), ids.joint);
    assert_eq!(source.bind_chain_id(raw_chain).unwrap(), ids.chain);
    assert_eq!(source.bind_contact_id(raw_contact).unwrap(), contact);

    assert_error(foreign.bind_body_id(raw_body), ApiError::WrongWorld);
    assert_error(foreign.bind_shape_id(raw_shape), ApiError::WrongWorld);
    assert_error(foreign.bind_joint_id(raw_joint), ApiError::WrongWorld);
    assert_error(foreign.bind_chain_id(raw_chain), ApiError::WrongWorld);
    assert_error(foreign.bind_contact_id(raw_contact), ApiError::WrongWorld);

    source.destroy_shape_id(contact_shape, true);
    assert_error(source.try_contact_data(contact), ApiError::InvalidContactId);
    assert_eq!(source.try_contact_is_valid(contact), Ok(false));
    assert_error(
        source.bind_contact_id(raw_contact),
        ApiError::InvalidContactId,
    );

    source.destroy_joint_id(ids.joint, true);
    assert_error(
        source.try_distance_length(ids.joint),
        ApiError::InvalidJointId,
    );
    assert_error(source.bind_joint_id(raw_joint), ApiError::InvalidJointId);

    source.destroy_chain_id(ids.chain);
    assert_error(source.try_chain(ids.chain), ApiError::InvalidChainId);
    assert_error(source.bind_chain_id(raw_chain), ApiError::InvalidChainId);

    source.destroy_shape_id(ids.shape, true);
    assert_error(source.try_shape_aabb(ids.shape), ApiError::InvalidShapeId);
    assert_error(source.bind_shape_id(raw_shape), ApiError::InvalidShapeId);

    source.destroy_body_id(ids.body_a);
    assert_error(
        source.try_body_position(ids.body_a),
        ApiError::InvalidBodyId,
    );
    assert_error(source.bind_body_id(raw_body), ApiError::InvalidBodyId);

    assert_panics(|| {
        let _ = source.contact_data(contact);
    });
    assert_panics(|| {
        let _ = source.distance_length(ids.joint);
    });
    assert!(source.chain(ids.chain).is_none());
    assert_panics(|| {
        let _ = source.shape_aabb(ids.shape);
    });
    assert_panics(|| {
        let _ = source.body_position(ids.body_a);
    });
}

#[test]
fn recycled_native_world_slot_cannot_rebrand_old_safe_or_raw_ids() {
    let (old_world_id, old_body, old_raw) = {
        let mut old_world = new_world();
        let old_body =
            old_world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        (old_world.world_id_raw(), old_body, old_body.unbind())
    };

    let mut retained_worlds = Vec::new();
    let mut recycled_world = None;
    for _ in 0..256 {
        let candidate = new_world();
        if candidate.world_id_raw().index1 == old_world_id.index1 {
            recycled_world = Some(candidate);
            break;
        }
        retained_worlds.push(candidate);
    }
    let mut recycled_world = recycled_world.expect("Box2D should eventually recycle the slot");
    assert_ne!(
        recycled_world.world_id_raw().generation,
        old_world_id.generation
    );

    let replacement = recycled_world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([7.0_f32, 0.0])
            .build(),
    );
    let replacement_position = recycled_world.body_position(replacement);
    assert_ne!(old_body, replacement);

    assert_error(
        recycled_world.try_body_position(old_body),
        ApiError::WrongWorld,
    );
    assert_error(recycled_world.bind_body_id(old_raw), ApiError::WrongWorld);
    assert_panics(|| {
        let _ = recycled_world.body_position(old_body);
    });
    assert_eq!(
        recycled_world.body_position(replacement),
        replacement_position
    );

    drop(retained_worlds);
}
