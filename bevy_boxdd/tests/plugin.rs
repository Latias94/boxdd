use bevy_app::{App, FixedUpdate};
use bevy_boxdd::prelude::*;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::message::{Message, MessageCursor, Messages};
use bevy_ecs::prelude::{Commands, Entity, NonSend, Resource};
use bevy_ecs::schedule::{ApplyDeferred, IntoScheduleConfigs};
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};
use bevy_transform::components::Transform;
use static_assertions::assert_not_impl_any;
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

assert_not_impl_any!(BoxddPhysicsContext: Send, Sync);
assert_not_impl_any!(BoxddBody: Copy, Clone);
assert_not_impl_any!(BoxddShape: Copy, Clone);
assert_not_impl_any!(BoxddJoint: Copy, Clone);
assert_not_impl_any!(BoxddPhysicsSnapshot: Copy, Clone, Send, Sync);
assert_not_impl_any!(BoxddSnapshotRestoreTicket: Copy);
assert_not_impl_any!(BoxddPhysicsSettings: Resource);
assert_not_impl_any!(BoxddPhysicsPlugin: Default);
static_assertions::assert_impl_all!(BoxddSnapshotRestoreTicket: Clone, Send, Sync);

#[derive(Resource, Default)]
struct ProjectionHookPanics {
    insert: bool,
    remove: bool,
}

#[derive(Resource, Default)]
struct ProjectionHookMutation {
    remove_inserted_body: bool,
    despawn_inserted_body: bool,
    replace_world_origin: bool,
    replace_context: bool,
    request_world_origin_rebase: Option<boxdd::Position>,
    remove_restore_messages: bool,
    replace_restore_messages_with: Option<BoxddSnapshotRestoreMessage>,
}

#[derive(Resource, Default)]
struct PhysicsSetTrace(Vec<&'static str>);

#[derive(Resource, Default)]
struct ContextReplacementQueued(bool);

#[derive(Resource)]
struct RestoreStepProbe {
    native_shape_count_at_step: Option<i32>,
}

fn trace_prepare_constraints(mut trace: bevy_ecs::prelude::ResMut<PhysicsSetTrace>) {
    trace.0.push("prepare_constraints");
}

fn trace_restore(mut trace: bevy_ecs::prelude::ResMut<PhysicsSetTrace>) {
    trace.0.push("restore");
}

fn trace_send_bridge(mut trace: bevy_ecs::prelude::ResMut<PhysicsSetTrace>) {
    trace.0.push("send_bridge");
}

fn trace_writeback(mut trace: bevy_ecs::prelude::ResMut<PhysicsSetTrace>) {
    trace.0.push("writeback");
}

fn replace_context_with_same_world(world: &mut bevy_ecs::world::World) {
    drop(world.remove_non_send::<BoxddPhysicsContext>());
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let replacement =
        BoxddPhysicsContext::new(world, foundation, &BoxddPhysicsSettings::default()).unwrap();
    world.insert_non_send(replacement);
}

fn remove_snapshot_restore_messages(world: &mut bevy_ecs::world::World) {
    drop(world.remove_resource::<Messages<BoxddSnapshotRestoreMessage>>());
}

fn replace_snapshot_restore_messages(
    world: &mut bevy_ecs::world::World,
    sentinel: BoxddSnapshotRestoreMessage,
) {
    let mut messages = Messages::<BoxddSnapshotRestoreMessage>::default();
    messages.write(sentinel);
    world.insert_resource(messages);
}

fn queue_same_world_context_replacement(
    mut commands: Commands,
    mut queued: bevy_ecs::prelude::ResMut<ContextReplacementQueued>,
) {
    if queued.0 {
        return;
    }
    queued.0 = true;
    commands.queue(replace_context_with_same_world);
}

fn capture_native_shape_count_at_step(
    context: NonSend<BoxddPhysicsContext>,
    mut probe: bevy_ecs::prelude::ResMut<RestoreStepProbe>,
) {
    probe.native_shape_count_at_step = context
        .world()
        .and_then(|world| world.counters().ok())
        .map(|counters| counters.shape_count);
}

fn step_fixed(app: &mut App, steps: usize) {
    for _ in 0..steps {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn app_with_settings(settings: BoxddPhysicsSettings) -> App {
    let mut app = App::new();
    app.add_plugins(physics_plugin(settings));
    app
}

fn physics_plugin(settings: BoxddPhysicsSettings) -> BoxddPhysicsPlugin {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    BoxddPhysicsPlugin::new(foundation, settings)
}

fn world_position(x: f32, y: f32) -> boxdd::Position {
    boxdd::Position::from([x, y])
}

fn read_messages<M>(app: &App) -> Vec<M>
where
    M: Message + Clone,
{
    let messages = app.world().resource::<Messages<M>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).cloned().collect()
}

fn queue_snapshot_restore(
    app: &mut App,
    snapshot: BoxddPhysicsSnapshot,
) -> BoxddSnapshotRestoreTicket {
    app.world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .queue_snapshot_restore(snapshot)
        .unwrap()
}

fn finish_snapshot_restore(
    app: &mut App,
    ticket: &BoxddSnapshotRestoreTicket,
) -> Result<(), BoxddSnapshotError> {
    step_fixed(app, 1);
    let mut outcomes = read_messages::<BoxddSnapshotRestoreMessage>(app)
        .into_iter()
        .filter(|message| message.ticket.eq(ticket));
    let outcome = outcomes
        .next()
        .expect("queued snapshot restore must emit exactly one outcome");
    assert!(
        outcomes.next().is_none(),
        "queued snapshot restore emitted more than one outcome"
    );
    outcome.result
}

fn restore_snapshot_through_fixed_pipeline(
    app: &mut App,
    snapshot: BoxddPhysicsSnapshot,
) -> Result<(), BoxddSnapshotError> {
    let ticket = queue_snapshot_restore(app, snapshot);
    finish_snapshot_restore(app, &ticket)
}

fn matches_pair(
    entity_a: Option<Entity>,
    entity_b: Option<Entity>,
    expected_a: Entity,
    expected_b: Entity,
) -> bool {
    matches!(
        (entity_a, entity_b),
        (Some(a), Some(b))
            if (a == expected_a && b == expected_b) || (a == expected_b && b == expected_a)
    )
}

fn matches_shape_pair(
    shape_a: boxdd::ShapeId,
    shape_b: boxdd::ShapeId,
    expected_a: boxdd::ShapeId,
    expected_b: boxdd::ShapeId,
) -> bool {
    (shape_a == expected_a && shape_b == expected_b)
        || (shape_a == expected_b && shape_b == expected_a)
}

#[derive(Clone, Copy)]
struct TestRng(u64);

impl TestRng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn index(&mut self, len: usize) -> usize {
        (self.next() as usize) % len
    }
}

fn run_randomized_identity_scenario(seed: u64) {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let bodies: [Entity; 4] = std::array::from_fn(|index| {
        app.world_mut()
            .spawn(Transform::from_xyz(index as f32, 0.0, 0.0))
            .id()
    });
    app.world_mut()
        .entity_mut(bodies[0])
        .insert(RigidBody::Static);
    app.world_mut()
        .entity_mut(bodies[1])
        .insert(RigidBody::Dynamic);

    let colliders: [Entity; 4] = std::array::from_fn(|index| {
        app.world_mut()
            .spawn((Transform::default(), ChildOf(bodies[index])))
            .id()
    });
    app.world_mut()
        .entity_mut(colliders[0])
        .insert(Collider::circle(0.25));
    app.world_mut()
        .entity_mut(colliders[1])
        .insert(Collider::square(0.2));

    let joints: [Entity; 3] = std::array::from_fn(|_| app.world_mut().spawn_empty().id());
    app.world_mut()
        .entity_mut(joints[0])
        .insert(random_joint_descriptor(bodies[0], bodies[1], false));
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);

    app.world_mut()
        .entity_mut(colliders[0])
        .remove::<Collider>();
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);
    app.world_mut()
        .entity_mut(colliders[0])
        .insert((Collider::circle(0.3), ChildOf(bodies[1])));
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);
    app.world_mut().entity_mut(bodies[1]).remove::<RigidBody>();
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);
    app.world_mut()
        .entity_mut(bodies[1])
        .insert(RigidBody::Dynamic);
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);

    let mut rng = TestRng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    for _ in 0..96 {
        match rng.index(12) {
            0 => {
                let entity = bodies[rng.index(bodies.len())];
                if app.world().entity(entity).contains::<RigidBody>() {
                    app.world_mut().entity_mut(entity).remove::<RigidBody>();
                } else {
                    let body = match rng.index(3) {
                        0 => RigidBody::Static,
                        1 => RigidBody::Kinematic,
                        _ => RigidBody::Dynamic,
                    };
                    app.world_mut().entity_mut(entity).insert(body);
                }
            }
            1 => {
                let entity = bodies[rng.index(bodies.len())];
                let body = match rng.index(3) {
                    0 => RigidBody::Static,
                    1 => RigidBody::Kinematic,
                    _ => RigidBody::Dynamic,
                };
                app.world_mut().entity_mut(entity).insert(body);
            }
            2 => {
                let entity = colliders[rng.index(colliders.len())];
                if app.world().entity(entity).contains::<Collider>() {
                    app.world_mut().entity_mut(entity).remove::<Collider>();
                } else {
                    app.world_mut()
                        .entity_mut(entity)
                        .insert(Collider::circle(0.1 + rng.index(5) as f32 * 0.05));
                }
            }
            3 => {
                let child = colliders[rng.index(colliders.len())];
                let parent = bodies[rng.index(bodies.len())];
                app.world_mut().entity_mut(child).insert(ChildOf(parent));
            }
            4 => {
                let child = colliders[rng.index(colliders.len())];
                app.world_mut().entity_mut(child).remove::<ChildOf>();
            }
            5 => {
                let child = colliders[rng.index(colliders.len())];
                let parent = bodies[rng.index(bodies.len())];
                app.world_mut()
                    .entity_mut(child)
                    .insert((ChildOf(parent), Collider::square(0.2)));
            }
            6 => {
                let joint = joints[rng.index(joints.len())];
                if app.world().entity(joint).contains::<JointDescriptor>() {
                    app.world_mut()
                        .entity_mut(joint)
                        .remove::<JointDescriptor>();
                } else {
                    let a = rng.index(bodies.len());
                    let b = (a + 1 + rng.index(bodies.len() - 1)) % bodies.len();
                    let revolute = rng.index(2) == 0;
                    app.world_mut()
                        .entity_mut(joint)
                        .insert(random_joint_descriptor(bodies[a], bodies[b], revolute));
                }
            }
            7 => {
                let joint = joints[rng.index(joints.len())];
                let a = rng.index(bodies.len());
                let b = (a + 1 + rng.index(bodies.len() - 1)) % bodies.len();
                let revolute = rng.index(2) == 0;
                app.world_mut()
                    .entity_mut(joint)
                    .insert(random_joint_descriptor(bodies[a], bodies[b], revolute));
            }
            8 => {
                let collider = colliders[rng.index(colliders.len())];
                if app.world().entity(collider).contains::<PhysicsMaterial>() {
                    app.world_mut()
                        .entity_mut(collider)
                        .remove::<PhysicsMaterial>();
                } else {
                    app.world_mut()
                        .entity_mut(collider)
                        .insert(PhysicsMaterial {
                            friction: 0.1 + rng.index(8) as f32 * 0.1,
                            ..Default::default()
                        });
                }
            }
            9 => {
                let source_index = rng.index(bodies.len());
                let target = bodies[(source_index + 1) % bodies.len()];
                if let Some(projection) = app
                    .world_mut()
                    .entity_mut(bodies[source_index])
                    .take::<BoxddBody>()
                {
                    app.world_mut().entity_mut(target).insert(projection);
                }
            }
            10 => {
                let source_index = rng.index(colliders.len());
                let target = colliders[(source_index + 1) % colliders.len()];
                if let Some(projection) = app
                    .world_mut()
                    .entity_mut(colliders[source_index])
                    .take::<BoxddShape>()
                {
                    app.world_mut().entity_mut(target).insert(projection);
                }
            }
            _ => {
                let source_index = rng.index(joints.len());
                let target = joints[(source_index + 1) % joints.len()];
                if let Some(projection) = app
                    .world_mut()
                    .entity_mut(joints[source_index])
                    .take::<BoxddJoint>()
                {
                    app.world_mut().entity_mut(target).insert(projection);
                }
            }
        }

        step_fixed(&mut app, 1);
        assert_identity_graph(&mut app);
    }
}

fn choose_entity<const N: usize>(rng: &mut TestRng, slots: &[Option<Entity>; N]) -> Option<Entity> {
    let start = rng.index(N);
    (0..N).find_map(|offset| slots[(start + offset) % N])
}

fn choose_body_pair<const N: usize>(
    rng: &mut TestRng,
    slots: &[Option<Entity>; N],
) -> Option<(Entity, Entity)> {
    let live: Vec<_> = slots.iter().flatten().copied().collect();
    if live.len() < 2 {
        return None;
    }
    let index_a = rng.index(live.len());
    let index_b = (index_a + 1 + rng.index(live.len() - 1)) % live.len();
    Some((live[index_a], live[index_b]))
}

fn spawn_random_body(app: &mut App, rng: &mut TestRng) -> Entity {
    let rigid_body = match rng.index(3) {
        0 => RigidBody::Static,
        1 => RigidBody::Kinematic,
        _ => RigidBody::Dynamic,
    };
    app.world_mut()
        .spawn((rigid_body, Transform::default()))
        .id()
}

fn detach_colliders_from_body(app: &mut App, colliders: &[Option<Entity>], body: Entity) {
    for collider in colliders.iter().flatten().copied() {
        let attached = app
            .world()
            .get_entity(collider)
            .ok()
            .and_then(|entity| entity.get::<ChildOf>())
            .is_some_and(|parent| parent.parent() == body);
        if attached {
            app.world_mut().entity_mut(collider).remove::<ChildOf>();
        }
    }
}

fn run_randomized_entity_generation_scenario(seed: u64) {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let mut rng = TestRng(seed.wrapping_mul(0xd134_2543_de82_ef95));
    let mut bodies: [Option<Entity>; 4] =
        std::array::from_fn(|_| Some(spawn_random_body(&mut app, &mut rng)));
    let mut colliders: [Option<Entity>; 4] = std::array::from_fn(|index| {
        Some(
            app.world_mut()
                .spawn((
                    Collider::circle(0.2),
                    Transform::default(),
                    ChildOf(bodies[index].unwrap()),
                ))
                .id(),
        )
    });
    let mut joints: [Option<Entity>; 3] = std::array::from_fn(|_| {
        let (body_a, body_b) = choose_body_pair(&mut rng, &bodies).unwrap();
        Some(
            app.world_mut()
                .spawn(random_joint_descriptor(body_a, body_b, rng.index(2) == 0))
                .id(),
        )
    });
    step_fixed(&mut app, 1);
    assert_identity_graph(&mut app);

    for iteration in 0..64 {
        match rng.index(11) {
            0 => {
                let index = rng.index(bodies.len());
                if let Some(entity) = bodies[index].take() {
                    detach_colliders_from_body(&mut app, &colliders, entity);
                    assert!(app.world_mut().despawn(entity));
                } else {
                    bodies[index] = Some(spawn_random_body(&mut app, &mut rng));
                }
            }
            1 => {
                let index = rng.index(colliders.len());
                if let Some(entity) = colliders[index].take() {
                    assert!(app.world_mut().despawn(entity));
                } else {
                    let entity = app
                        .world_mut()
                        .spawn((Collider::square(0.2), Transform::default()))
                        .id();
                    if let Some(parent) = choose_entity(&mut rng, &bodies) {
                        app.world_mut().entity_mut(entity).insert(ChildOf(parent));
                    }
                    colliders[index] = Some(entity);
                }
            }
            2 => {
                let index = rng.index(joints.len());
                if let Some(entity) = joints[index].take() {
                    assert!(app.world_mut().despawn(entity));
                } else {
                    let entity = app.world_mut().spawn_empty().id();
                    if let Some((body_a, body_b)) = choose_body_pair(&mut rng, &bodies) {
                        let descriptor = random_joint_descriptor(body_a, body_b, rng.index(2) == 0);
                        app.world_mut().entity_mut(entity).insert(descriptor);
                    }
                    joints[index] = Some(entity);
                }
            }
            3 => {
                if let Some(collider) = choose_entity(&mut rng, &colliders) {
                    if let Some(parent) = choose_entity(&mut rng, &bodies) {
                        app.world_mut().entity_mut(collider).insert(ChildOf(parent));
                    } else {
                        app.world_mut().entity_mut(collider).remove::<ChildOf>();
                    }
                }
            }
            4 => {
                if let Some(body) = choose_entity(&mut rng, &bodies) {
                    if app.world().entity(body).contains::<RigidBody>() {
                        app.world_mut().entity_mut(body).remove::<RigidBody>();
                    } else {
                        let rigid_body = match rng.index(3) {
                            0 => RigidBody::Static,
                            1 => RigidBody::Kinematic,
                            _ => RigidBody::Dynamic,
                        };
                        app.world_mut().entity_mut(body).insert(rigid_body);
                    }
                }
            }
            5 => {
                if let Some(collider) = choose_entity(&mut rng, &colliders) {
                    if app.world().entity(collider).contains::<Collider>() {
                        app.world_mut().entity_mut(collider).remove::<Collider>();
                    } else {
                        app.world_mut()
                            .entity_mut(collider)
                            .insert(Collider::circle(0.1 + rng.index(5) as f32 * 0.05));
                    }
                }
            }
            6 => {
                if let (Some(joint), Some((body_a, body_b))) = (
                    choose_entity(&mut rng, &joints),
                    choose_body_pair(&mut rng, &bodies),
                ) {
                    let descriptor = random_joint_descriptor(body_a, body_b, rng.index(2) == 0);
                    app.world_mut().entity_mut(joint).insert(descriptor);
                }
            }
            7 => {
                let index = rng.index(bodies.len());
                let retired = bodies[index].take();
                if let Some(entity) = retired {
                    detach_colliders_from_body(&mut app, &colliders, entity);
                    assert!(app.world_mut().despawn(entity));
                }
                let replacement = spawn_random_body(&mut app, &mut rng);
                if let Some(retired) = retired {
                    assert_ne!(replacement, retired);
                }
                bodies[index] = Some(replacement);
            }
            8 => {
                let index = rng.index(colliders.len());
                let retired = colliders[index].take();
                if let Some(entity) = retired {
                    assert!(app.world_mut().despawn(entity));
                }
                let replacement = app
                    .world_mut()
                    .spawn((Collider::square(0.15), Transform::default()))
                    .id();
                if let Some(retired) = retired {
                    assert_ne!(replacement, retired);
                }
                if let Some(parent) = choose_entity(&mut rng, &bodies) {
                    app.world_mut()
                        .entity_mut(replacement)
                        .insert(ChildOf(parent));
                }
                colliders[index] = Some(replacement);
            }
            9 => {
                let index = rng.index(joints.len());
                let retired = joints[index].take();
                if let Some(entity) = retired {
                    assert!(app.world_mut().despawn(entity));
                }
                let replacement = app.world_mut().spawn_empty().id();
                if let Some(retired) = retired {
                    assert_ne!(replacement, retired);
                }
                if let Some((body_a, body_b)) = choose_body_pair(&mut rng, &bodies) {
                    let descriptor = random_joint_descriptor(body_a, body_b, rng.index(2) == 0);
                    app.world_mut().entity_mut(replacement).insert(descriptor);
                }
                joints[index] = Some(replacement);
            }
            _ => {
                if let (Some(source), Some(target)) = (
                    choose_entity(&mut rng, &bodies),
                    choose_entity(&mut rng, &colliders),
                ) && let Some(projection) =
                    app.world_mut().entity_mut(source).take::<BoxddBody>()
                {
                    app.world_mut().entity_mut(target).insert(projection);
                }
            }
        }

        step_fixed(&mut app, 1);
        assert_identity_graph(&mut app);

        if iteration % 8 == 7 {
            let snapshot = app
                .world_mut()
                .non_send_mut::<BoxddPhysicsContext>()
                .snapshot()
                .unwrap();
            let divergent = app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Collider::circle(0.17),
                    Transform::default(),
                ))
                .id();
            step_fixed(&mut app, 1);
            if let Some(source) = choose_entity(&mut rng, &bodies)
                && let Some(projection) = app.world_mut().entity_mut(source).take::<BoxddBody>()
            {
                app.world_mut().entity_mut(divergent).insert(projection);
            }
            assert!(app.world_mut().despawn(divergent));
            assert_eq!(
                restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
                Ok(())
            );
            assert_identity_graph(&mut app);
        }
    }
}

fn random_joint_descriptor(body_a: Entity, body_b: Entity, revolute: bool) -> JointDescriptor {
    if revolute {
        JointDescriptor::revolute(body_a, body_b, world_position(0.5, 0.0))
    } else {
        JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        )
    }
}

fn assert_identity_graph(app: &mut App) {
    let ecs_world = app.world();
    let context = ecs_world.non_send::<BoxddPhysicsContext>();
    let native = context.world().unwrap();
    let mut body_ids = HashSet::new();
    let mut shape_ids = HashSet::new();
    let mut joint_ids = HashSet::new();
    let mut expected_bodies = 0;
    let mut actual_bodies = 0;
    let mut actual_shapes = 0;
    let mut actual_joints = 0;

    for entity_ref in ecs_world.iter_entities() {
        let entity = entity_ref.id();
        let expected_body = entity_ref.contains::<RigidBody>();
        expected_bodies += usize::from(expected_body);
        assert_eq!(
            entity_ref.contains::<BoxddBody>(),
            expected_body,
            "body projection drift for {entity:?}"
        );
        if let Some(body) = entity_ref.get::<BoxddBody>() {
            actual_bodies += 1;
            assert!(
                body_ids.insert(body.id()),
                "duplicate body id on {entity:?}"
            );
            assert_eq!(context.body_entity(body.id()), Some(entity));
        }

        let expected_shape_owner = entity_ref.get::<Collider>().and_then(|_| {
            if entity_ref.contains::<BoxddBody>() {
                Some(entity)
            } else {
                entity_ref.get::<ChildOf>().and_then(|parent| {
                    ecs_world
                        .get_entity(parent.parent())
                        .ok()
                        .filter(|parent| parent.contains::<BoxddBody>())
                        .map(|_| parent.parent())
                })
            }
        });
        if expected_shape_owner.is_some() {
            assert!(
                entity_ref.contains::<BoxddShape>(),
                "valid shape descriptor has no projection for {entity:?}"
            );
        }
        if let Some(shape) = entity_ref.get::<BoxddShape>() {
            actual_shapes += 1;
            assert!(
                shape_ids.insert(shape.id()),
                "duplicate shape id on {entity:?}"
            );
            assert_eq!(context.shape_entity(shape.id()), Some(entity));
            assert!(
                context.shape_owner_entity(shape.id()).is_some(),
                "authoritative shape has no active body for {entity:?}"
            );
        }

        let expected_joint = entity_ref
            .get::<JointDescriptor>()
            .is_some_and(|descriptor| {
                ecs_world
                    .get_entity(descriptor.entity_a)
                    .is_ok_and(|body| body.contains::<BoxddBody>())
                    && ecs_world
                        .get_entity(descriptor.entity_b)
                        .is_ok_and(|body| body.contains::<BoxddBody>())
            });
        if expected_joint {
            assert!(
                entity_ref.contains::<BoxddJoint>(),
                "valid joint descriptor has no projection for {entity:?}"
            );
        }
        if let Some(joint) = entity_ref.get::<BoxddJoint>() {
            actual_joints += 1;
            assert!(
                joint_ids.insert(joint.id()),
                "duplicate joint id on {entity:?}"
            );
            assert_eq!(context.joint_entity(joint.id()), Some(entity));
            assert!(
                context.joint_endpoint_entities(joint.id()).is_some(),
                "authoritative joint has an inactive endpoint for {entity:?}"
            );
        }
    }

    assert_eq!(actual_bodies, expected_bodies);
    let counters = native.counters().unwrap();
    assert_eq!(counters.body_count, actual_bodies as i32);
    assert_eq!(counters.shape_count, actual_shapes);
    assert_eq!(counters.joint_count, actual_joints);
    let _snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
}

#[test]
fn plugin_creates_body_shape_and_syncs_dynamic_transform() {
    let mut app = App::new();
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        fixed_timestep_seconds: Some(1.0 / 60.0),
        ..Default::default()
    }));

    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::from_xyz(0.0, 3.0, 2.0),
        ))
        .id();

    step_fixed(&mut app, 8);

    let entity_ref = app.world().entity(entity);
    assert!(entity_ref.contains::<BoxddBody>());
    assert!(entity_ref.contains::<BoxddShape>());
    let transform = entity_ref.get::<Transform>().unwrap();
    assert!(transform.translation.y < 3.0);
    assert_eq!(transform.translation.z, 2.0);
}

#[test]
fn public_physics_sets_preserve_restore_prepare_and_writeback_order() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    app.init_resource::<PhysicsSetTrace>().add_systems(
        FixedUpdate,
        (
            trace_restore.in_set(BoxddPhysicsSet::Restore),
            trace_send_bridge
                .after(BoxddPhysicsSet::Restore)
                .before(BoxddPhysicsSet::PrepareConstraints),
            trace_prepare_constraints.in_set(BoxddPhysicsSet::PrepareConstraints),
            trace_writeback.in_set(BoxddPhysicsSet::Writeback),
        ),
    );

    step_fixed(&mut app, 1);

    assert_eq!(
        app.world().resource::<PhysicsSetTrace>().0,
        ["restore", "send_bridge", "prepare_constraints", "writeback"]
    );
}

#[test]
fn same_world_context_replacement_after_validation_stops_the_remaining_chain() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    app.init_resource::<ContextReplacementQueued>().add_systems(
        FixedUpdate,
        (queue_same_world_context_replacement, ApplyDeferred)
            .chain()
            .after(BoxddPhysicsSet::Validate)
            .before(BoxddPhysicsSet::Rebase),
    );

    step_fixed(&mut app, 1);

    assert!(
        !app.world().entity(entity).contains::<BoxddBody>(),
        "a context inserted after validation must not mutate ECS or native state"
    );
    assert_eq!(
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .counters()
            .unwrap()
            .body_count,
        0
    );

    step_fixed(&mut app, 1);

    assert!(
        app.world().entity(entity).contains::<BoxddBody>(),
        "the replacement context may take over after the next validation"
    );
}

#[test]
fn queued_snapshot_restore_runs_before_cleanup_and_cannot_step_a_removed_collider() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    app.world_mut().entity_mut(entity).remove::<Collider>();
    app.insert_resource(RestoreStepProbe {
        native_shape_count_at_step: None,
    })
    .add_systems(
        FixedUpdate,
        capture_native_shape_count_at_step.in_set(BoxddPhysicsSet::Step),
    );

    let ticket = queue_snapshot_restore(&mut app, snapshot);
    assert!(read_messages::<BoxddSnapshotRestoreMessage>(&app).is_empty());
    assert!(app.world().entity(entity).contains::<BoxddShape>());

    assert_eq!(finish_snapshot_restore(&mut app, &ticket), Ok(()));
    assert_eq!(
        app.world()
            .resource::<RestoreStepProbe>()
            .native_shape_count_at_step,
        Some(0)
    );
    assert!(!app.world().entity(entity).contains::<BoxddShape>());
    assert_eq!(
        app.world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .counters()
            .unwrap()
            .shape_count,
        0
    );
}

#[test]
fn queued_snapshot_restore_waits_for_a_pending_origin_rebase() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    app.world_mut()
        .spawn((RigidBody::Static, Transform::default()));
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let invalid = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(f32::MAX, 0.0, 0.0)))
        .id();
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(boxdd::Position::from([-f32::MAX, 0.0]))
        .unwrap();

    let ticket = queue_snapshot_restore(&mut app, snapshot);
    step_fixed(&mut app, 1);
    assert!(
        app.world()
            .resource::<BoxddWorldOrigin>()
            .pending()
            .is_some()
    );
    assert!(read_messages::<BoxddSnapshotRestoreMessage>(&app).is_empty());

    assert!(app.world_mut().despawn(invalid));
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .cancel_pending_rebase();
    assert_eq!(finish_snapshot_restore(&mut app, &ticket), Ok(()));
}

#[test]
fn queued_snapshot_restore_rejects_a_second_pending_request() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    app.world_mut()
        .spawn((RigidBody::Static, Transform::default()));
    step_fixed(&mut app, 1);
    let first = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let second = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    let ticket = queue_snapshot_restore(&mut app, first);
    assert_eq!(
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .queue_snapshot_restore(second),
        Err(BoxddSnapshotError::RestoreAlreadyQueued)
    );
    assert_eq!(finish_snapshot_restore(&mut app, &ticket), Ok(()));
}

#[test]
fn snapshot_restore_can_be_cancelled_while_the_origin_is_unavailable() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let first_ticket = queue_snapshot_restore(&mut app, snapshot);

    app.world_mut().remove_resource::<BoxddWorldOrigin>();
    step_fixed(&mut app, 1);
    assert!(read_messages::<BoxddSnapshotRestoreMessage>(&app).is_empty());

    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .cancel_snapshot_restore(&first_ticket)
        .expect("the matching pending restore must return its snapshot");
    let second_ticket = queue_snapshot_restore(&mut app, snapshot);
    assert!(
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .cancel_snapshot_restore(&first_ticket)
            .is_none(),
        "a stale ticket must not cancel the current request"
    );
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .cancel_snapshot_restore(&second_ticket)
        .expect("the current ticket must remain cancellable");

    app.world_mut().insert_resource(BoxddWorldOrigin::default());
    let final_ticket = queue_snapshot_restore(&mut app, snapshot);
    assert_eq!(finish_snapshot_restore(&mut app, &final_ticket), Ok(()));
}

#[test]
fn snapshot_restore_tickets_do_not_alias_across_context_replacement() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    let old_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let old_ticket = queue_snapshot_restore(&mut app, old_snapshot);
    assert_eq!(finish_snapshot_restore(&mut app, &old_ticket), Ok(()));

    replace_context_with_same_world(app.world_mut());
    let new_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let new_ticket = queue_snapshot_restore(&mut app, new_snapshot);
    assert_ne!(old_ticket, new_ticket);
    assert_eq!(finish_snapshot_restore(&mut app, &new_ticket), Ok(()));

    let outcomes = read_messages::<BoxddSnapshotRestoreMessage>(&app);
    assert_eq!(
        outcomes
            .iter()
            .filter(|message| message.ticket.eq(&old_ticket))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|message| message.ticket.eq(&new_ticket))
            .count(),
        1
    );
}

#[test]
fn replacing_a_context_cancels_its_pending_snapshot_ticket_without_an_outcome() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let cancelled_ticket = queue_snapshot_restore(&mut app, snapshot);

    replace_context_with_same_world(app.world_mut());
    step_fixed(&mut app, 1);

    assert!(
        read_messages::<BoxddSnapshotRestoreMessage>(&app)
            .iter()
            .all(|message| !message.ticket.eq(&cancelled_ticket))
    );
}

#[test]
fn rigid_body_child_of_is_rejected_until_hierarchy_is_removed() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let parent = app.world_mut().spawn_empty().id();
    let body = app
        .world_mut()
        .spawn((RigidBody::Dynamic, ChildOf(parent), Transform::default()))
        .id();

    step_fixed(&mut app, 2);

    assert!(!app.world().entity(body).contains::<BoxddBody>());
    let errors = read_messages::<BoxddErrorMessage>(&app)
        .into_iter()
        .filter(|message| message.entity == Some(body))
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 1, "unchanged hierarchy errors must be cached");
    assert_eq!(errors[0].operation, BoxddOperation::ValidateBodyHierarchy);
    assert_eq!(
        errors[0].error,
        BoxddPluginError::RigidBodyChildOf { parent }
    );

    app.world_mut().entity_mut(body).remove::<ChildOf>();
    step_fixed(&mut app, 1);
    assert!(app.world().entity(body).contains::<BoxddBody>());
}

#[test]
fn runtime_configuration_is_split_by_mutability_and_events_are_opt_in() {
    let app = app_with_settings(BoxddPhysicsSettings {
        sub_step_count: 7,
        event_interests: BoxddEventInterests::NONE.with_contacts(true),
        error_policy: BoxddErrorPolicy::MessageAndLog,
        ..Default::default()
    });

    assert_eq!(
        app.world().resource::<BoxddStepSettings>().sub_step_count,
        7
    );
    assert_eq!(
        *app.world().resource::<BoxddEventInterests>(),
        BoxddEventInterests::NONE.with_contacts(true)
    );
    assert_eq!(
        *app.world().resource::<BoxddErrorPolicy>(),
        BoxddErrorPolicy::MessageAndLog
    );
    assert_eq!(
        BoxddPhysicsSettings::default().event_interests,
        BoxddEventInterests::NONE
    );

    let initialized = app_with_settings(BoxddPhysicsSettings {
        fixed_timestep_seconds: None,
        ..Default::default()
    });
    assert!(initialized.world().contains_resource::<Time<Fixed>>());

    let mut preserved = App::new();
    preserved.insert_resource(Time::<Fixed>::from_hz(120.0));
    preserved.add_plugins(physics_plugin(BoxddPhysicsSettings {
        fixed_timestep_seconds: None,
        ..Default::default()
    }));
    assert_eq!(
        preserved.world().resource::<Time<Fixed>>().timestep(),
        Time::<Fixed>::from_hz(120.0).timestep()
    );
    assert_eq!(
        preserved
            .world()
            .resource::<BoxddStepSettings>()
            .fallback_timestep_seconds,
        Time::<Fixed>::from_hz(120.0).timestep().as_secs_f32()
    );

    let invalid = app_with_settings(BoxddPhysicsSettings {
        fixed_timestep_seconds: Some(f64::MAX),
        ..Default::default()
    });
    assert!(
        read_messages::<BoxddErrorMessage>(&invalid)
            .iter()
            .any(|message| message.operation == BoxddOperation::ConfigureFixedTimestep)
    );
}

#[test]
fn static_transform_syncs_from_bevy_to_boxdd() {
    let mut app = App::new();
    app.add_plugins(physics_plugin(BoxddPhysicsSettings::default()));

    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(1.0, 0.25),
            Transform::from_xyz(2.0, -1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let body = app.world().entity(entity).get::<BoxddBody>().unwrap().id();
    let position = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .body_transform(body)
        .unwrap()
        .position();
    assert_eq!(position, boxdd::Position::from([2.0_f32, -1.0]));
}

#[test]
fn removing_rigid_body_destroys_native_handles() {
    let mut app = App::new();
    app.add_plugins(physics_plugin(BoxddPhysicsSettings::default()));

    let entity: Entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    assert!(app.world().entity(entity).contains::<BoxddBody>());

    app.world_mut().entity_mut(entity).remove::<RigidBody>();
    step_fixed(&mut app, 1);

    let entity_ref = app.world().entity(entity);
    assert!(!entity_ref.contains::<BoxddBody>());
    assert!(!entity_ref.contains::<BoxddShape>());
}

#[test]
fn despawn_and_replacement_cannot_rebind_a_stale_entity_or_body_id() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let original = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let original_body = app
        .world()
        .entity(original)
        .get::<BoxddBody>()
        .unwrap()
        .id();

    assert!(app.world_mut().despawn(original));
    step_fixed(&mut app, 1);
    let replacement = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);

    let replacement_body = app
        .world()
        .entity(replacement)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    assert_ne!(replacement_body, original_body);
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(context.body_entity(original_body), None);
    assert_eq!(context.body_entity(replacement_body), Some(replacement));
}

#[test]
fn moving_runtime_projections_cannot_transfer_native_ownership() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let body_a = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.5),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let body_b = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.4),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();
    let joint_a = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();
    let joint_b = app
        .world_mut()
        .spawn(JointDescriptor::revolute(
            body_a,
            body_b,
            world_position(0.5, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let body_id_a = app.world().entity(body_a).get::<BoxddBody>().unwrap().id();
    let body_id_b = app.world().entity(body_b).get::<BoxddBody>().unwrap().id();
    let shape_id_a = app.world().entity(body_a).get::<BoxddShape>().unwrap().id();
    let shape_id_b = app.world().entity(body_b).get::<BoxddShape>().unwrap().id();
    let joint_id_a = app
        .world()
        .entity(joint_a)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let joint_id_b = app
        .world()
        .entity(joint_b)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    let body_projection = app
        .world_mut()
        .entity_mut(body_a)
        .take::<BoxddBody>()
        .unwrap();
    app.world_mut().entity_mut(body_b).insert(body_projection);
    let shape_projection = app
        .world_mut()
        .entity_mut(body_a)
        .take::<BoxddShape>()
        .unwrap();
    app.world_mut().entity_mut(body_b).insert(shape_projection);
    let joint_projection = app
        .world_mut()
        .entity_mut(joint_a)
        .take::<BoxddJoint>()
        .unwrap();
    app.world_mut().entity_mut(joint_b).insert(joint_projection);

    step_fixed(&mut app, 1);

    assert_eq!(
        app.world().entity(body_a).get::<BoxddBody>().unwrap().id(),
        body_id_a
    );
    assert_eq!(
        app.world().entity(body_b).get::<BoxddBody>().unwrap().id(),
        body_id_b
    );
    assert_eq!(
        app.world().entity(body_a).get::<BoxddShape>().unwrap().id(),
        shape_id_a
    );
    assert_eq!(
        app.world().entity(body_b).get::<BoxddShape>().unwrap().id(),
        shape_id_b
    );
    assert_eq!(
        app.world()
            .entity(joint_a)
            .get::<BoxddJoint>()
            .unwrap()
            .id(),
        joint_id_a
    );
    assert_eq!(
        app.world()
            .entity(joint_b)
            .get::<BoxddJoint>()
            .unwrap()
            .id(),
        joint_id_b
    );

    {
        let context = app.world().non_send::<BoxddPhysicsContext>();
        assert_eq!(context.body_entity(body_id_a), Some(body_a));
        assert_eq!(context.body_entity(body_id_b), Some(body_b));
        assert_eq!(context.shape_entity(shape_id_a), Some(body_a));
        assert_eq!(context.shape_entity(shape_id_b), Some(body_b));
        assert_eq!(context.joint_entity(joint_id_a), Some(joint_a));
        assert_eq!(context.joint_entity(joint_id_b), Some(joint_b));
    }
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert!(context.body_transform(body_id_a).is_ok());
    assert!(context.body_transform(body_id_b).is_ok());
    assert_eq!(context.shape_body_id(shape_id_a).unwrap(), body_id_a);
    assert_eq!(context.shape_body_id(shape_id_b).unwrap(), body_id_b);
    assert_eq!(
        context.joint_type(joint_id_a).unwrap(),
        boxdd::JointType::Distance
    );
    assert_eq!(
        context.joint_type(joint_id_b).unwrap(),
        boxdd::JointType::Revolute
    );
}

#[test]
fn queued_snapshot_restore_rebinds_the_complete_plugin_identity_graph() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let body_a = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.5),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let body_b = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.4),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();
    step_fixed(&mut app, 1);

    let snapshot_body_b = app.world().entity(body_b).get::<BoxddBody>().unwrap().id();
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    app.world_mut()
        .entity_mut(body_a)
        .insert(Collider::rectangle(0.8, 0.2));
    app.world_mut()
        .entity_mut(joint_entity)
        .insert(JointDescriptor::revolute(
            body_a,
            body_b,
            world_position(0.5, 0.0),
        ));
    app.world_mut().entity_mut(body_b).remove::<RigidBody>();
    step_fixed(&mut app, 1);
    app.world_mut()
        .entity_mut(body_b)
        .insert(RigidBody::Dynamic);
    step_fixed(&mut app, 1);

    let divergent_body_b = app.world().entity(body_b).get::<BoxddBody>().unwrap().id();
    let divergent_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let post_snapshot = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::square(0.25),
            Transform::from_xyz(2.0, 0.0, 0.0),
        ))
        .id();
    step_fixed(&mut app, 1);
    let post_snapshot_body = app
        .world()
        .entity(post_snapshot)
        .get::<BoxddBody>()
        .unwrap()
        .id();

    let displaced_body = app
        .world_mut()
        .entity_mut(body_a)
        .take::<BoxddBody>()
        .unwrap();
    app.world_mut()
        .entity_mut(post_snapshot)
        .insert(displaced_body);
    let displaced_shape = app
        .world_mut()
        .entity_mut(body_a)
        .take::<BoxddShape>()
        .unwrap();
    app.world_mut()
        .entity_mut(post_snapshot)
        .insert(displaced_shape);
    let displaced_joint = app
        .world_mut()
        .entity_mut(joint_entity)
        .take::<BoxddJoint>()
        .unwrap();
    app.world_mut()
        .entity_mut(post_snapshot)
        .insert(displaced_joint);

    assert!(app.world_mut().despawn(post_snapshot));
    assert_eq!(
        restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
        Ok(())
    );

    let restored_body_a = app.world().entity(body_a).get::<BoxddBody>().unwrap().id();
    let restored_body_b = app.world().entity(body_b).get::<BoxddBody>().unwrap().id();
    let restored_shape_a = app.world().entity(body_a).get::<BoxddShape>().unwrap().id();
    let restored_shape_b = app.world().entity(body_b).get::<BoxddShape>().unwrap().id();
    let restored_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    assert_ne!(restored_body_b, snapshot_body_b);
    assert_ne!(restored_body_b, divergent_body_b);
    assert!(app.world().get_entity(post_snapshot).is_err());

    {
        let context = app.world().non_send::<BoxddPhysicsContext>();
        assert_eq!(context.body_entity(restored_body_a), Some(body_a));
        assert_eq!(context.body_entity(restored_body_b), Some(body_b));
        assert_eq!(context.shape_entity(restored_shape_a), Some(body_a));
        assert_eq!(context.shape_entity(restored_shape_b), Some(body_b));
        assert_eq!(context.joint_entity(restored_joint), Some(joint_entity));
    }
    {
        let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
        assert_eq!(
            context.body_transform(snapshot_body_b).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidBodyId)
        );
        assert_eq!(
            context.body_transform(divergent_body_b).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidBodyId)
        );
        assert_eq!(
            context.body_transform(post_snapshot_body).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidBodyId)
        );
        assert_eq!(
            context.shape_body_id(restored_shape_a).unwrap(),
            restored_body_a
        );
        assert_eq!(
            context.shape_body_id(restored_shape_b).unwrap(),
            restored_body_b
        );
        assert_eq!(
            context.joint_body_ids(restored_joint).unwrap(),
            (restored_body_a, restored_body_b)
        );
        assert_eq!(
            context.joint_type(restored_joint).unwrap(),
            boxdd::JointType::Revolute
        );
        assert_eq!(
            context.joint_type(divergent_joint).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidJointId)
        );
    }

    assert_identity_graph(&mut app);
}

#[test]
fn snapshot_restore_reports_missing_entities_before_pipeline_cleanup() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let captured = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 1);
    let captured_id = app
        .world()
        .entity(captured)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    let post_snapshot = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let post_snapshot_id = app
        .world()
        .entity(post_snapshot)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    assert!(app.world_mut().despawn(captured));

    let error = restore_snapshot_through_fixed_pipeline(&mut app, snapshot).unwrap_err();
    assert_eq!(
        error,
        BoxddSnapshotError::EntityMissing {
            entity: captured,
            kind: BoxddSnapshotObjectKind::Body,
        }
    );
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .into_iter()
            .any(|message| {
                message.operation == BoxddOperation::RestoreSnapshot
                    && message.error == BoxddPluginError::Snapshot(error)
            })
    );

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(
        context.body_transform(captured_id).unwrap_err(),
        BoxddPluginError::Api(boxdd::Error::InvalidBodyId)
    );
    assert!(context.body_transform(post_snapshot_id).is_ok());
    assert_eq!(context.body_entity(captured_id), None);
    assert_eq!(context.body_entity(post_snapshot_id), Some(post_snapshot));
}

#[test]
fn queued_snapshot_restore_honors_the_panic_error_policy() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        error_policy: BoxddErrorPolicy::Panic,
        ..Default::default()
    });
    let entity = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    assert!(app.world_mut().despawn(entity));

    let ticket = queue_snapshot_restore(&mut app, snapshot);
    let result = catch_unwind(AssertUnwindSafe(|| {
        finish_snapshot_restore(&mut app, &ticket)
    }));
    assert!(result.is_err());
    let outcomes: Vec<_> = read_messages::<BoxddSnapshotRestoreMessage>(&app)
        .into_iter()
        .filter(|message| message.ticket == ticket)
        .collect();
    assert_eq!(
        outcomes,
        [BoxddSnapshotRestoreMessage {
            ticket,
            result: Err(BoxddSnapshotError::EntityMissing {
                entity,
                kind: BoxddSnapshotObjectKind::Body,
            }),
        }]
    );
}

#[test]
fn snapshot_restore_rejects_a_foreign_bevy_world_before_native_mutation() {
    let mut source = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let source_entity = source
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut source, 1);
    let snapshot = source
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let source_world = source.world().id();

    let mut target = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let target_entity = target
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::square(0.25),
            Transform::default(),
        ))
        .id();
    let target_world = target.world().id();
    assert_eq!(source_entity, target_entity);
    step_fixed(&mut target, 1);
    let target_body = target
        .world()
        .entity(target_entity)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let target_shape = target
        .world()
        .entity(target_entity)
        .get::<BoxddShape>()
        .unwrap()
        .id();

    let error = target
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .queue_snapshot_restore(snapshot)
        .unwrap_err();
    assert_eq!(
        error,
        BoxddSnapshotError::WrongEcsWorld {
            expected: source_world,
            actual: target_world,
        }
    );

    let entity = target.world().entity(target_entity);
    assert_eq!(entity.get::<BoxddBody>().unwrap().id(), target_body);
    assert_eq!(entity.get::<BoxddShape>().unwrap().id(), target_shape);
    {
        let context = target.world().non_send::<BoxddPhysicsContext>();
        assert_eq!(context.body_entity(target_body), Some(target_entity));
        assert_eq!(context.shape_entity(target_shape), Some(target_entity));
    }
    assert!(
        target
            .world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .body_transform(target_body)
            .is_ok()
    );
    assert_eq!(
        target
            .world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .shape_body_id(target_shape)
            .unwrap(),
        target_body
    );
}

#[test]
fn snapshot_restore_rejects_a_foreign_native_context_before_native_mutation() {
    let settings = BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    };
    let mut app = app_with_settings(settings.clone());
    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 1);
    let body_id = app.world().entity(entity).get::<BoxddBody>().unwrap().id();
    let shape_id = app.world().entity(entity).get::<BoxddShape>().unwrap().id();

    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let mut foreign_context = BoxddPhysicsContext::new(app.world(), foundation, &settings).unwrap();
    let foreign_snapshot = foreign_context.snapshot().unwrap();
    let error = restore_snapshot_through_fixed_pipeline(&mut app, foreign_snapshot).unwrap_err();
    assert_eq!(
        error,
        BoxddSnapshotError::Api(boxdd::Error::ForeignSnapshot)
    );

    let entity_ref = app.world().entity(entity);
    assert_eq!(entity_ref.get::<BoxddBody>().unwrap().id(), body_id);
    assert_eq!(entity_ref.get::<BoxddShape>().unwrap().id(), shape_id);
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(context.body_entity(body_id), Some(entity));
    assert_eq!(context.shape_entity(shape_id), Some(entity));
}

#[test]
fn snapshot_restore_rejects_a_context_moved_to_another_bevy_world() {
    let mut source = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let source_entity = source
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut source, 1);
    let body_id = source
        .world()
        .entity(source_entity)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let snapshot = source
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let source_world = source.world().id();
    let context = source
        .world_mut()
        .remove_non_send::<BoxddPhysicsContext>()
        .unwrap();

    let mut target = app_with_settings(BoxddPhysicsSettings::default());
    let unrelated = target.world_mut().spawn_empty().id();
    let target_world = target.world().id();
    drop(
        target
            .world_mut()
            .remove_non_send::<BoxddPhysicsContext>()
            .unwrap(),
    );
    target.world_mut().insert_non_send(context);
    target
        .world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(world_position(1.0, 0.0))
        .unwrap();

    let ticket = queue_snapshot_restore(&mut target, snapshot);
    let error = finish_snapshot_restore(&mut target, &ticket).unwrap_err();
    assert_eq!(
        error,
        BoxddSnapshotError::WrongEcsWorld {
            expected: source_world,
            actual: target_world,
        }
    );
    assert!(
        target
            .world()
            .resource::<BoxddWorldOrigin>()
            .pending()
            .is_some(),
        "a foreign context must report its terminal ownership error before pending-origin waiting"
    );
    let context = target.world().non_send::<BoxddPhysicsContext>();
    assert!(context.world().is_some());
    assert_eq!(context.body_entity(body_id), Some(source_entity));
    assert!(!target.world().entity(unrelated).contains::<BoxddBody>());
}

#[test]
fn fixed_update_rejects_a_context_moved_between_plugin_worlds_before_any_mutation() {
    let settings = BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    };
    let mut source = app_with_settings(settings.clone());
    let source_entity = source
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            LinearVelocity(Vec2::X),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut source, 1);
    let source_world = source.world().id();
    let body_id = source
        .world()
        .entity(source_entity)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let native_position = source
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .body_transform(body_id)
        .unwrap()
        .position();
    let foreign_context = source
        .world_mut()
        .remove_non_send::<BoxddPhysicsContext>()
        .unwrap();
    let source_requested_origin = world_position(-1_000.0, 0.0);
    source
        .world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(source_requested_origin)
        .unwrap();
    step_fixed(&mut source, 2);
    assert_eq!(
        source.world().resource::<BoxddWorldOrigin>().pending(),
        Some(source_requested_origin)
    );
    let missing_context_errors = read_messages::<BoxddErrorMessage>(&source)
        .into_iter()
        .filter(|message| {
            message.operation == BoxddOperation::ValidateWorldBinding
                && message.error == BoxddPluginError::ContextUnavailable
        })
        .count();
    assert_eq!(missing_context_errors, 1);

    let mut target = app_with_settings(settings);
    let target_entity = target
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(100.0, 0.0, 0.0)))
        .id();
    assert_eq!(source_entity, target_entity);
    let target_world = target.world().id();
    let requested_origin = world_position(1_000.0, 0.0);
    target
        .world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(requested_origin)
        .unwrap();
    drop(
        target
            .world_mut()
            .remove_non_send::<BoxddPhysicsContext>()
            .unwrap(),
    );
    target.world_mut().insert_non_send(foreign_context);

    step_fixed(&mut target, 2);

    let target_ref = target.world().entity(target_entity);
    assert!(!target_ref.contains::<BoxddBody>());
    assert_eq!(target_ref.get::<Transform>().unwrap().translation.x, 100.0);
    let origin = target.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), world_position(0.0, 0.0));
    assert_eq!(origin.pending(), Some(requested_origin));
    let mut context = target.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(context.body_entity(body_id), Some(source_entity));
    assert_eq!(
        context.body_transform(body_id).unwrap().position(),
        native_position
    );
    let binding_errors = read_messages::<BoxddErrorMessage>(&target)
        .into_iter()
        .filter(|message| {
            message.operation == BoxddOperation::ValidateWorldBinding
                && message.error
                    == BoxddPluginError::WrongEcsWorld {
                        expected: source_world,
                        actual: target_world,
                    }
        })
        .count();
    assert_eq!(binding_errors, 1);
}

#[test]
fn snapshot_restore_reinserts_a_disabled_context_after_projection_hooks_panic() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookPanics::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let mut panics = world.resource_mut::<ProjectionHookPanics>();
            if std::mem::take(&mut panics.insert) {
                panic!("intentional BoxddBody insert hook panic");
            }
        })
        .on_remove(|mut world, _| {
            let mut panics = world.resource_mut::<ProjectionHookPanics>();
            if std::mem::take(&mut panics.remove) {
                panic!("intentional BoxddBody remove hook panic");
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 1);
    let body_id = app.world().entity(entity).get::<BoxddBody>().unwrap().id();
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    {
        let mut panics = app.world_mut().resource_mut::<ProjectionHookPanics>();
        panics.insert = true;
        panics.remove = true;
    }
    let ticket = queue_snapshot_restore(&mut app, snapshot);
    let result = catch_unwind(AssertUnwindSafe(|| {
        finish_snapshot_restore(&mut app, &ticket)
    }));
    assert!(result.is_err());
    let outcomes: Vec<_> = read_messages::<BoxddSnapshotRestoreMessage>(&app)
        .into_iter()
        .filter(|message| message.ticket == ticket)
        .collect();
    assert_eq!(
        outcomes,
        [BoxddSnapshotRestoreMessage {
            ticket,
            result: Err(BoxddSnapshotError::RestorePanicked),
        }]
    );
    assert!(app.world().contains_non_send::<BoxddPhysicsContext>());
    {
        let context = app.world().non_send::<BoxddPhysicsContext>();
        assert!(context.world().is_none());
        assert_eq!(
            context.disabled_reason(),
            Some(BoxddContextDisabledReason::SnapshotRestoreFailed)
        );
        assert_eq!(context.body_entity(body_id), None);
    }

    let entity_ref = app.world().entity(entity);
    assert!(!entity_ref.contains::<BoxddBody>());
    assert!(!entity_ref.contains::<BoxddShape>());
}

#[test]
fn snapshot_restore_rejects_non_panicking_projection_hook_drift() {
    for despawn in [false, true] {
        let mut app = App::new();
        app.insert_resource(ProjectionHookMutation::default());
        app.world_mut()
            .register_component_hooks::<BoxddBody>()
            .on_insert(|mut world, context| {
                let (remove, despawn) = {
                    let mut mutation = world.resource_mut::<ProjectionHookMutation>();
                    (
                        std::mem::take(&mut mutation.remove_inserted_body),
                        std::mem::take(&mut mutation.despawn_inserted_body),
                    )
                };
                if remove {
                    world
                        .commands()
                        .entity(context.entity)
                        .remove::<BoxddBody>();
                }
                if despawn {
                    world.commands().entity(context.entity).despawn();
                }
            });
        app.add_plugins(physics_plugin(BoxddPhysicsSettings {
            gravity: Vec2::ZERO,
            ..Default::default()
        }));
        let entity = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default()))
            .id();
        step_fixed(&mut app, 1);
        let snapshot = app
            .world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .snapshot()
            .unwrap();

        app.world_mut().entity_mut(entity).remove::<BoxddBody>();
        {
            let mut mutation = app.world_mut().resource_mut::<ProjectionHookMutation>();
            mutation.remove_inserted_body = !despawn;
            mutation.despawn_inserted_body = despawn;
        }

        assert_eq!(
            restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
            Err(BoxddSnapshotError::Api(
                boxdd::Error::SnapshotManifestMismatch
            ))
        );
        {
            let context = app.world().non_send::<BoxddPhysicsContext>();
            assert!(context.world().is_none());
            assert_eq!(
                context.disabled_reason(),
                Some(BoxddContextDisabledReason::SnapshotRestoreFailed)
            );
        }
        let body_projection_absent = match app.world().get_entity(entity) {
            Ok(entity) => !entity.contains::<BoxddBody>(),
            Err(_) => true,
        };
        assert!(body_projection_absent);
    }
}

#[test]
fn snapshot_restore_rejects_world_origin_drift_from_projection_hooks() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookMutation::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let replace_world_origin = {
                let mut mutation = world.resource_mut::<ProjectionHookMutation>();
                std::mem::take(&mut mutation.replace_world_origin)
            };
            if replace_world_origin {
                *world.resource_mut::<BoxddWorldOrigin>() =
                    BoxddWorldOrigin::new(boxdd::Position::from([1_000.0_f32, 0.0])).unwrap();
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    app.world_mut().entity_mut(entity).remove::<BoxddBody>();
    app.world_mut()
        .resource_mut::<ProjectionHookMutation>()
        .replace_world_origin = true;

    assert_eq!(
        restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
        Err(BoxddSnapshotError::WorldBindingChanged)
    );
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert!(context.world().is_none());
    assert_eq!(
        context.disabled_reason(),
        Some(BoxddContextDisabledReason::SnapshotRestoreFailed)
    );
    assert!(!app.world().entity(entity).contains::<BoxddBody>());
}

#[test]
fn snapshot_restore_preserves_a_replacement_context_installed_by_a_projection_hook() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookMutation::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let replace_context = {
                let mut mutation = world.resource_mut::<ProjectionHookMutation>();
                std::mem::take(&mut mutation.replace_context)
            };
            if replace_context {
                world.commands().queue(replace_context_with_same_world);
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    app.world_mut().entity_mut(entity).remove::<BoxddBody>();
    app.world_mut()
        .resource_mut::<ProjectionHookMutation>()
        .replace_context = true;

    assert_eq!(
        restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
        Err(BoxddSnapshotError::WorldBindingChanged)
    );
    {
        let context = app.world().non_send::<BoxddPhysicsContext>();
        let counters = context.world().unwrap().counters().unwrap();
        assert_eq!(context.disabled_reason(), None);
        assert_eq!(counters.body_count, 0);
    }
    assert!(
        !app.world().entity(entity).contains::<BoxddBody>(),
        "the failed restore must not leave a projection owned by the retired context"
    );

    step_fixed(&mut app, 1);
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(context.world().unwrap().counters().unwrap().body_count, 1);
    assert!(app.world().entity(entity).contains::<BoxddBody>());
}

#[test]
fn snapshot_restore_preserves_a_rebase_requested_by_a_projection_hook() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookMutation::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let target = world
                .resource_mut::<ProjectionHookMutation>()
                .request_world_origin_rebase
                .take();
            if let Some(target) = target {
                world
                    .resource_mut::<BoxddWorldOrigin>()
                    .request_rebase(target)
                    .unwrap();
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    let requested_origin = world_position(1_000.0, 0.0);

    app.world_mut().entity_mut(entity).remove::<BoxddBody>();
    app.world_mut()
        .resource_mut::<ProjectionHookMutation>()
        .request_world_origin_rebase = Some(requested_origin);

    assert_eq!(
        restore_snapshot_through_fixed_pipeline(&mut app, snapshot),
        Ok(())
    );
    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), boxdd::Position::ZERO);
    assert_eq!(origin.pending(), Some(requested_origin));
    assert!(
        app.world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .is_some()
    );

    step_fixed(&mut app, 1);
    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), requested_origin);
    assert_eq!(origin.pending(), None);
    assert!(
        app.world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .is_some()
    );
}

#[test]
fn snapshot_restore_preserves_persistent_outcome_readers_when_a_hook_removes_the_buffer() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookMutation::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let remove_messages = {
                let mut mutation = world.resource_mut::<ProjectionHookMutation>();
                std::mem::take(&mut mutation.remove_restore_messages)
            };
            if remove_messages {
                world.commands().queue(remove_snapshot_restore_messages);
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let first_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    let mut outcome_reader = MessageCursor::<BoxddSnapshotRestoreMessage>::default();
    let first_ticket = queue_snapshot_restore(&mut app, first_snapshot);
    step_fixed(&mut app, 1);
    let first_outcomes = outcome_reader
        .read(
            app.world()
                .resource::<Messages<BoxddSnapshotRestoreMessage>>(),
        )
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(first_outcomes.len(), 1);
    assert_eq!(first_outcomes[0].ticket, first_ticket);
    assert_eq!(first_outcomes[0].result, Ok(()));

    let second_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    app.world_mut().entity_mut(entity).remove::<BoxddBody>();
    app.world_mut()
        .resource_mut::<ProjectionHookMutation>()
        .remove_restore_messages = true;
    let second_ticket = queue_snapshot_restore(&mut app, second_snapshot);

    step_fixed(&mut app, 1);
    let second_outcomes = outcome_reader
        .read(
            app.world()
                .resource::<Messages<BoxddSnapshotRestoreMessage>>(),
        )
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(second_outcomes.len(), 1);
    assert_eq!(second_outcomes[0].ticket, second_ticket);
    assert_eq!(second_outcomes[0].result, Ok(()));
}

#[test]
fn snapshot_restore_merges_replacement_messages_for_persistent_outcome_readers() {
    let mut app = App::new();
    app.insert_resource(ProjectionHookMutation::default());
    app.world_mut()
        .register_component_hooks::<BoxddBody>()
        .on_insert(|mut world, _| {
            let sentinel = world
                .resource_mut::<ProjectionHookMutation>()
                .replace_restore_messages_with
                .take();
            if let Some(sentinel) = sentinel {
                world
                    .commands()
                    .queue(move |world: &mut bevy_ecs::world::World| {
                        replace_snapshot_restore_messages(world, sentinel);
                    });
            }
        });
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    step_fixed(&mut app, 1);
    let first_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();

    let mut outcome_reader = MessageCursor::<BoxddSnapshotRestoreMessage>::default();
    let first_ticket = queue_snapshot_restore(&mut app, first_snapshot);
    step_fixed(&mut app, 1);
    let first_outcomes = outcome_reader
        .read(
            app.world()
                .resource::<Messages<BoxddSnapshotRestoreMessage>>(),
        )
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(first_outcomes.len(), 1);
    assert_eq!(first_outcomes[0].ticket, first_ticket);
    assert_eq!(first_outcomes[0].result, Ok(()));

    let second_snapshot = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .snapshot()
        .unwrap();
    app.world_mut().entity_mut(entity).remove::<BoxddBody>();

    let sentinel = BoxddSnapshotRestoreMessage {
        ticket: first_ticket.clone(),
        result: Ok(()),
    };
    app.world_mut()
        .resource_mut::<ProjectionHookMutation>()
        .replace_restore_messages_with = Some(sentinel.clone());
    let second_ticket = queue_snapshot_restore(&mut app, second_snapshot);

    step_fixed(&mut app, 1);
    let outcomes = outcome_reader
        .read(
            app.world()
                .resource::<Messages<BoxddSnapshotRestoreMessage>>(),
        )
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        vec![
            sentinel,
            BoxddSnapshotRestoreMessage {
                ticket: second_ticket.clone(),
                result: Ok(()),
            },
        ]
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|message| message.ticket == second_ticket)
            .count(),
        1,
        "the current restore ticket must produce exactly one outcome"
    );
}

#[test]
fn randomized_identity_graph_survives_create_remove_reparent_and_recreate() {
    for seed in 1..=4 {
        run_randomized_identity_scenario(seed);
    }
}

#[test]
fn randomized_identity_graph_survives_entity_generation_churn_and_restore() {
    for seed in 1..=4 {
        run_randomized_entity_generation_scenario(seed);
    }
}

#[test]
fn linear_impulse_is_one_shot_component() {
    let mut app = App::new();
    app.add_plugins(physics_plugin(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));

    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            LinearImpulse::new(Vec2::new(1.0, 0.0)),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 2);

    let entity_ref = app.world().entity(entity);
    assert!(!entity_ref.contains::<LinearImpulse>());
    assert!(entity_ref.get::<Transform>().unwrap().translation.x > 0.0);
}

#[test]
fn contact_begin_and_end_messages_include_current_contact_ids() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        event_interests: BoxddEventInterests::NONE.with_contacts(true),
        ..Default::default()
    });
    let contact_material = PhysicsMaterial {
        enable_contact_events: true,
        enable_hit_events: true,
        ..Default::default()
    };

    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.5),
            contact_material,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let box_entity = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::rectangle(0.5, 0.5),
            contact_material,
            Transform::from_xyz(0.0, 0.75, 0.0),
        ))
        .id();

    let begin = (0..8)
        .find_map(|_| {
            step_fixed(&mut app, 1);
            read_messages::<BoxddContactBeginMessage>(&app)
                .into_iter()
                .find(|message| {
                    matches_pair(message.entity_a, message.entity_b, ground, box_entity)
                })
        })
        .expect("expected a contact begin message mapped to both Bevy entities");
    assert!(
        app.world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .contact_is_valid(begin.contact_id)
            .unwrap(),
        "the begin message should expose the current completed-step contact id"
    );

    app.world_mut().entity_mut(box_entity).insert((
        TransformSyncMode::BevyToPhysics,
        Transform::from_xyz(0.0, 4.0, 0.0),
    ));
    let end = (0..8)
        .find_map(|_| {
            step_fixed(&mut app, 1);
            read_messages::<BoxddContactEndMessage>(&app)
                .into_iter()
                .find(|message| {
                    matches_pair(message.entity_a, message.entity_b, ground, box_entity)
                })
        })
        .expect("expected a contact end message mapped to both Bevy entities");
    assert_ne!(
        end.contact_id, begin.contact_id,
        "the end message must carry its current-step id instead of a stale begin id"
    );
}

#[test]
fn contact_end_keeps_the_destroyed_collider_entity_mapping() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        event_interests: BoxddEventInterests::NONE.with_contacts(true),
        ..Default::default()
    });
    let material = PhysicsMaterial {
        enable_contact_events: true,
        ..Default::default()
    };
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.5),
            material,
            Transform::default(),
        ))
        .id();
    let visitor = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::rectangle(0.5, 0.5),
            material,
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .id();

    (0..8)
        .find_map(|_| {
            step_fixed(&mut app, 1);
            read_messages::<BoxddContactBeginMessage>(&app)
                .into_iter()
                .find(|message| matches_pair(message.entity_a, message.entity_b, ground, visitor))
        })
        .expect("expected the initial contact");

    let ground_shape = app.world().entity(ground).get::<BoxddShape>().unwrap().id();
    let visitor_shape = app
        .world()
        .entity(visitor)
        .get::<BoxddShape>()
        .unwrap()
        .id();

    let mut end_reader = MessageCursor::<BoxddContactEndMessage>::default();
    app.world_mut().entity_mut(visitor).remove::<Collider>();
    step_fixed(&mut app, 1);

    let ends = {
        let messages = app.world().resource::<Messages<BoxddContactEndMessage>>();
        end_reader
            .read(messages)
            .filter(|message| {
                matches_shape_pair(
                    message.shape_a,
                    message.shape_b,
                    ground_shape,
                    visitor_shape,
                )
            })
            .copied()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ends.len(),
        1,
        "destroying a collider must publish exactly one mapped contact end"
    );
    assert!(matches_pair(
        ends[0].entity_a,
        ends[0].entity_b,
        ground,
        visitor
    ));
    {
        let context = app.world().non_send::<BoxddPhysicsContext>();
        let live_entities = [
            context.shape_entity(ends[0].shape_a),
            context.shape_entity(ends[0].shape_b),
        ];
        assert!(live_entities.contains(&Some(ground)));
        assert!(live_entities.contains(&None));
    }
    step_fixed(&mut app, 1);
    let messages = app.world().resource::<Messages<BoxddContactEndMessage>>();
    assert_eq!(
        end_reader
            .read(messages)
            .filter(|message| {
                matches_shape_pair(
                    message.shape_a,
                    message.shape_b,
                    ground_shape,
                    visitor_shape,
                )
            })
            .count(),
        0,
        "the retired contact end must not be republished"
    );
}

#[test]
fn sensor_end_keeps_body_cascade_and_recreated_shape_entity_mappings() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        event_interests: BoxddEventInterests::NONE.with_sensors(true),
        ..Default::default()
    });
    let sensor = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(1.0, 1.0),
            PhysicsMaterial {
                is_sensor: true,
                enable_sensor_events: true,
                ..Default::default()
            },
            Transform::default(),
        ))
        .id();
    let visitor = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.25),
            PhysicsMaterial {
                enable_sensor_events: true,
                ..Default::default()
            },
            Transform::default(),
        ))
        .id();

    (0..8)
        .find_map(|_| {
            step_fixed(&mut app, 1);
            read_messages::<BoxddSensorBeginMessage>(&app)
                .into_iter()
                .find(|message| {
                    message.sensor_entity == Some(sensor) && message.visitor_entity == Some(visitor)
                })
        })
        .expect("expected the initial sensor overlap");

    let sensor_shape = app.world().entity(sensor).get::<BoxddShape>().unwrap().id();
    let visitor_shape = app
        .world()
        .entity(visitor)
        .get::<BoxddShape>()
        .unwrap()
        .id();

    let mut end_reader = MessageCursor::<BoxddSensorEndMessage>::default();
    app.world_mut().despawn(visitor);
    step_fixed(&mut app, 1);
    {
        let messages = app.world().resource::<Messages<BoxddSensorEndMessage>>();
        let ends = end_reader
            .read(messages)
            .filter(|message| {
                message.sensor_shape == sensor_shape && message.visitor_shape == visitor_shape
            })
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            ends.len(),
            1,
            "body cascade destruction must publish exactly one mapped sensor end"
        );
        assert_eq!(ends[0].sensor_entity, Some(sensor));
        assert_eq!(ends[0].visitor_entity, Some(visitor));
    }
    step_fixed(&mut app, 1);
    {
        let messages = app.world().resource::<Messages<BoxddSensorEndMessage>>();
        assert_eq!(
            end_reader
                .read(messages)
                .filter(|message| {
                    message.sensor_shape == sensor_shape && message.visitor_shape == visitor_shape
                })
                .count(),
            0,
            "the body-cascade sensor end must not be republished"
        );
    }

    let recreated = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.25),
            PhysicsMaterial {
                enable_sensor_events: true,
                ..Default::default()
            },
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 2);
    let recreated_shape = app
        .world()
        .entity(recreated)
        .get::<BoxddShape>()
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(recreated)
        .insert(Collider::rectangle(0.3, 0.3));
    step_fixed(&mut app, 1);
    {
        let messages = app.world().resource::<Messages<BoxddSensorEndMessage>>();
        let ends = end_reader
            .read(messages)
            .filter(|message| {
                message.sensor_shape == sensor_shape && message.visitor_shape == recreated_shape
            })
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            ends.len(),
            1,
            "descriptor recreation must publish exactly one mapped sensor end"
        );
        assert_eq!(ends[0].sensor_entity, Some(sensor));
        assert_eq!(ends[0].visitor_entity, Some(recreated));
    }
    step_fixed(&mut app, 1);
    let messages = app.world().resource::<Messages<BoxddSensorEndMessage>>();
    assert_eq!(
        end_reader
            .read(messages)
            .filter(|message| {
                message.sensor_shape == sensor_shape && message.visitor_shape == recreated_shape
            })
            .count(),
        0,
        "the descriptor-recreation sensor end must not be republished"
    );
}

#[test]
fn contact_hit_messages_publish_a_live_contact_id() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        event_interests: BoxddEventInterests::NONE.with_contacts(true),
        ..Default::default()
    });
    let contact_material = PhysicsMaterial {
        enable_contact_events: true,
        enable_hit_events: true,
        ..Default::default()
    };
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            contact_material,
            Transform::default(),
        ))
        .id();
    let projectile = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            BodySettings::bullet(),
            Collider::circle(0.25),
            contact_material,
            LinearVelocity(Vec2::new(0.0, -20.0)),
            Transform::from_xyz(0.0, 4.0, 0.0),
        ))
        .id();

    let hit = (0..60)
        .find_map(|_| {
            step_fixed(&mut app, 1);
            read_messages::<BoxddContactHitMessage>(&app)
                .into_iter()
                .find(|message| {
                    matches_pair(message.entity_a, message.entity_b, ground, projectile)
                })
        })
        .expect("expected a contact hit message mapped to both Bevy entities");
    assert!(
        app.world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .contact_is_valid(hit.contact_id)
            .unwrap(),
        "the hit message should expose the current completed-step contact id"
    );
}

#[test]
fn sensor_messages_include_begin_end_entity_mappings() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        event_interests: BoxddEventInterests::NONE.with_sensors(true),
        ..Default::default()
    });
    let sensor_material = PhysicsMaterial {
        is_sensor: true,
        enable_sensor_events: true,
        ..Default::default()
    };
    let visitor_material = PhysicsMaterial {
        enable_sensor_events: true,
        ..Default::default()
    };

    let sensor = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(0.5, 0.5),
            sensor_material,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let visitor = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            BodySettings::bullet(),
            Collider::circle(0.2),
            visitor_material,
            LinearVelocity(Vec2::new(4.0, 0.0)),
            Transform::from_xyz(-2.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 120);

    let begins = read_messages::<BoxddSensorBeginMessage>(&app);
    assert!(
        begins.iter().any(|message| {
            message.sensor_entity == Some(sensor) && message.visitor_entity == Some(visitor)
        }),
        "expected sensor begin message mapped to sensor and visitor entities, got {begins:?}"
    );

    let ends = read_messages::<BoxddSensorEndMessage>(&app);
    assert!(
        ends.iter().any(|message| {
            message.sensor_entity == Some(sensor) && message.visitor_entity == Some(visitor)
        }),
        "expected sensor end message mapped to sensor and visitor entities, got {ends:?}"
    );
}

#[test]
fn invalid_shape_inputs_emit_recoverable_error_messages() {
    let mut app = app_with_settings(BoxddPhysicsSettings::default());

    let invalid_collider = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.0),
            Transform::from_xyz(-1.0, 1.0, 0.0),
        ))
        .id();
    let invalid_material = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.25),
            PhysicsMaterial {
                density: -1.0,
                ..Default::default()
            },
            Transform::from_xyz(1.0, 1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let errors = read_messages::<BoxddErrorMessage>(&app);
    for entity in [invalid_collider, invalid_material] {
        assert!(
            errors.iter().any(|message| {
                message.operation == BoxddOperation::CreateShape
                    && message.entity == Some(entity)
                    && matches!(
                        message.error,
                        BoxddPluginError::Api(boxdd::Error::InvalidArgument { .. })
                    )
            }),
            "expected a recoverable CreateShape error for {entity:?}, got {errors:?}"
        );
        assert!(!app.world().entity(entity).contains::<BoxddShape>());
    }
}

#[test]
fn invalid_shape_replacement_preserves_old_native_shape_until_retry_succeeds() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.5),
            Transform::default(),
        ))
        .id();
    step_fixed(&mut app, 1);
    let old_id = app.world().entity(entity).get::<BoxddShape>().unwrap().id();

    app.world_mut()
        .entity_mut(entity)
        .insert(Collider::circle(-1.0));
    step_fixed(&mut app, 1);

    assert_eq!(
        app.world().entity(entity).get::<BoxddShape>().unwrap().id(),
        old_id,
        "failed replacement must retain the authoritative projection"
    );
    assert!(
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .shape_body_id(old_id)
            .is_ok(),
        "failed replacement must leave the old native shape live"
    );
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .iter()
            .any(|message| {
                message.entity == Some(entity) && message.operation == BoxddOperation::ReplaceShape
            })
    );

    app.world_mut()
        .entity_mut(entity)
        .insert(Collider::circle(0.75));
    step_fixed(&mut app, 1);
    let new_id = app.world().entity(entity).get::<BoxddShape>().unwrap().id();
    assert_ne!(new_id, old_id);
    assert_eq!(
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .shape_body_id(old_id)
            .unwrap_err(),
        BoxddPluginError::Api(boxdd::Error::InvalidShapeId)
    );
}

#[test]
fn reparenting_shape_to_new_body_replaces_it_in_the_same_fixed_step() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let old_body = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::default()))
        .id();
    let collider = app
        .world_mut()
        .spawn((
            Collider::circle(0.5),
            ChildOf(old_body),
            Transform::default(),
        ))
        .id();

    step_fixed(&mut app, 1);
    let old_shape = app
        .world()
        .entity(collider)
        .get::<BoxddShape>()
        .unwrap()
        .id();

    let new_body = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::default()))
        .id();
    app.world_mut()
        .entity_mut(collider)
        .insert(ChildOf(new_body));

    step_fixed(&mut app, 1);

    let new_shape = app
        .world()
        .entity(collider)
        .get::<BoxddShape>()
        .unwrap()
        .id();
    assert_ne!(new_shape, old_shape);
    {
        let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
        assert_eq!(context.shape_owner_entity(new_shape), Some(new_body));
        assert_eq!(
            context.shape_body_id(old_shape).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidShapeId)
        );
    }
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .iter()
            .all(|message| {
                message.entity != Some(collider)
                    || message.operation != BoxddOperation::ReplaceShape
            }),
        "same-frame reparenting must not report a transient replacement failure"
    );
}

#[test]
fn physics_context_ray_query_maps_hits_to_entities() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let context = app.world().non_send::<BoxddPhysicsContext>();
    let result = context
        .cast_ray_closest_entity_with_stats(
            boxdd::Position::from([0.0_f32, 2.0]),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
        )
        .unwrap();
    assert!(result.node_visits > 0);
    assert!(result.leaf_visits > 0);
    assert_eq!(result.hit.and_then(|hit| hit.entity), Some(ground));

    let hit = context
        .cast_ray_closest_entity(
            boxdd::Position::from([0.0_f32, 2.0]),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
        )
        .unwrap()
        .expect("expected the ray to hit the plugin-created ground");

    assert!(hit.hit.hit, "expected the native hit flag to be set");
    assert_eq!(hit.entity, Some(ground));
}

#[test]
fn physics_context_ray_query_all_reuses_entity_hit_buffer() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let mut hits = Vec::new();
    context
        .cast_ray_all_entities_into(
            boxdd::Position::from([0.0_f32, 2.0]),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap();

    assert!(
        hits.iter()
            .any(|hit| hit.hit.hit && hit.entity == Some(ground)),
        "expected all-ray helper to map at least one hit to the ground entity, got {hits:?}"
    );
    let hit_count = hits.len();
    let error = context
        .cast_ray_all_entities_into(
            boxdd::Position::new(boxdd::WorldScalar::NAN, 2.0),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        BoxddPluginError::Api(boxdd::Error::InvalidArgument { .. })
    ));
    assert_eq!(hits.len(), hit_count);
    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "fallible all-ray helper should preserve the caller buffer on error"
    );

    context
        .cast_ray_all_entities_into(
            boxdd::Position::from([10.0_f32, 2.0]),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap();
    assert!(hits.is_empty(), "missed rays should clear stale hits");
}

#[test]
fn physics_context_overlap_aabb_maps_hits_to_entities() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let hits = context
        .overlap_aabb_entities(
            boxdd::Position::ZERO,
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [2.0, 1.0]).unwrap(),
            boxdd::QueryFilter::default(),
        )
        .unwrap();

    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "expected overlap helper to map a hit to the ground entity, got {hits:?}"
    );
}

#[test]
fn physics_context_overlap_aabb_reuses_entity_hit_buffer() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let mut hits = Vec::new();
    context
        .overlap_aabb_entities_into(
            boxdd::Position::ZERO,
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [2.0, 1.0]).unwrap(),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap();

    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "expected overlap helper to map a hit to the ground entity, got {hits:?}"
    );
    let hit_count = hits.len();
    let error = boxdd::Aabb::new([1.0_f32, 1.0], [-1.0, -1.0]).unwrap_err();
    assert!(matches!(error, boxdd::Error::InvalidArgument { .. }));
    assert_eq!(hits.len(), hit_count);
    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "fallible overlap helper should preserve the caller buffer on error"
    );

    context
        .overlap_aabb_entities_into(
            boxdd::Position::ZERO,
            boxdd::Aabb::from_center_half_extents([10.0_f32, 10.0], [1.0, 1.0]).unwrap(),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap();
    assert!(
        hits.is_empty(),
        "missed overlap queries should clear stale hits"
    );
}

#[test]
fn physics_context_overlap_aabb_honors_query_filter() {
    const PLAYER: u64 = 0x0002;
    const TERRAIN: u64 = 0x0004;

    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let player = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.4),
            PhysicsMaterial {
                filter: boxdd::Filter {
                    category_bits: PLAYER,
                    ..Default::default()
                },
                ..Default::default()
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let terrain = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.4),
            PhysicsMaterial {
                filter: boxdd::Filter {
                    category_bits: TERRAIN,
                    ..Default::default()
                },
                ..Default::default()
            },
            Transform::from_xyz(0.25, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let hits = context
        .overlap_aabb_entities(
            boxdd::Position::ZERO,
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]).unwrap(),
            boxdd::QueryFilter::default().mask(PLAYER),
        )
        .unwrap();

    assert!(
        hits.iter().any(|hit| hit.entity == Some(player)),
        "expected filtered overlap to include the player shape, got {hits:?}"
    );
    assert!(
        hits.iter().all(|hit| hit.entity != Some(terrain)),
        "expected filtered overlap to exclude terrain shape, got {hits:?}"
    );
}

#[test]
fn physics_context_collects_debug_draw_commands() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::rectangle(2.0, 0.25),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let mut commands = Vec::new();
    context
        .debug_draw_collect_into(&mut commands, boxdd::DebugDrawOptions::default())
        .unwrap();

    assert!(
        !commands.is_empty(),
        "expected debug draw collection to emit commands for plugin-created shapes"
    );
}

#[test]
fn disabled_physics_context_helpers_return_typed_errors() {
    let world = bevy_ecs::world::World::new();
    let mut context = BoxddPhysicsContext::disabled(&world);
    let expected = BoxddPluginError::ContextDisabled {
        reason: BoxddContextDisabledReason::Explicit,
    };

    assert_eq!(context.set_gravity(Vec2::ZERO).unwrap_err(), expected);
    assert_eq!(context.enable_sleeping(true).unwrap_err(), expected);
    assert_eq!(context.enable_warm_starting(true).unwrap_err(), expected);
    assert_eq!(context.enable_continuous(true).unwrap_err(), expected);
    assert_eq!(
        context.snapshot().unwrap_err(),
        BoxddSnapshotError::ContextDisabled {
            reason: BoxddContextDisabledReason::Explicit,
        }
    );

    assert_eq!(
        context
            .cast_ray_closest_entity(
                boxdd::Position::ZERO,
                Vec2::new(1.0, 0.0),
                boxdd::QueryFilter::default(),
            )
            .unwrap_err(),
        expected
    );

    assert_eq!(
        context
            .cast_ray_closest_entity_with_stats(
                boxdd::Position::ZERO,
                Vec2::new(1.0, 0.0),
                boxdd::QueryFilter::default(),
            )
            .unwrap_err(),
        expected
    );

    assert_eq!(
        context
            .cast_ray_all_entities(
                boxdd::Position::ZERO,
                Vec2::new(1.0, 0.0),
                boxdd::QueryFilter::default(),
            )
            .unwrap_err(),
        expected
    );

    assert_eq!(
        context
            .overlap_aabb_entities(
                boxdd::Position::ZERO,
                boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]).unwrap(),
                boxdd::QueryFilter::default(),
            )
            .unwrap_err(),
        expected
    );

    assert_eq!(
        context
            .debug_draw_collect(boxdd::DebugDrawOptions::default())
            .unwrap_err(),
        expected
    );
}

#[test]
fn physics_context_ray_query_all_allocating_helper_maps_entities() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let hits = context
        .cast_ray_all_entities(
            boxdd::Position::from([0.0_f32, 2.0]),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
        )
        .unwrap();

    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "expected allocating all-ray helper to map a hit to the ground entity, got {hits:?}"
    );
}

#[test]
fn physics_context_debug_draw_allocating_helper_returns_commands() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::circle(0.5),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    step_fixed(&mut app, 1);

    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    let commands = context
        .debug_draw_collect(boxdd::DebugDrawOptions::default())
        .unwrap();

    assert!(
        !commands.is_empty(),
        "expected allocating debug draw helper to return commands"
    );
}

#[test]
fn native_ray_query_still_available_for_advanced_users() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let ground = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::rectangle(2.0, 0.25),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let context = app.world().non_send::<BoxddPhysicsContext>();
    let world = context.world().expect("physics world should be available");
    let hit = world
        .query()
        .unwrap()
        .cast_ray_closest(
            boxdd::Position::from([0.0_f32, 2.0]),
            boxdd::Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
        )
        .expect("ray query should succeed")
        .expect("expected the ray to hit the plugin-created ground");

    assert!(hit.hit);
    assert_eq!(context.shape_entity(hit.shape_id), Some(ground));
}

#[test]
fn kinematic_body_transform_drives_native_body_in_fixed_update() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let platform = app
        .world_mut()
        .spawn((
            RigidBody::Kinematic,
            Collider::rectangle(1.0, 0.2),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    app.world_mut()
        .entity_mut(platform)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec2::new(2.0, 1.5).extend(0.0);

    step_fixed(&mut app, 1);

    let body = app
        .world()
        .entity(platform)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let position = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .body_transform(body)
        .unwrap()
        .position();

    assert_eq!(position, boxdd::Position::from([2.0_f32, 1.5]));
}

#[test]
fn distance_joint_descriptor_creates_native_joint() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .expect("joint component should be inserted")
        .id();
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(
        context.joint_type(joint).unwrap(),
        boxdd::JointType::Distance
    );
    assert_eq!(context.joint_entity(joint), Some(joint_entity));
}

#[test]
fn joint_threshold_events_map_native_ids_to_bevy_entities() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        event_interests: BoxddEventInterests::NONE.with_joints(true),
        ..Default::default()
    });
    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(3.0, 2.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(5.0, 2.0, 0.0)))
        .id();
    let descriptor = JointDescriptor::distance(
        body_a,
        body_b,
        world_position(3.0, 2.0),
        world_position(5.0, 2.0),
    )
    .with_event_thresholds(0.0, f32::MAX);
    let joint_entity = app.world_mut().spawn(descriptor).id();

    step_fixed(&mut app, 1);
    let joint_id = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let mut messages = read_messages::<BoxddJointEventMessage>(&app);
    for _ in 0..240 {
        if !messages.is_empty() {
            break;
        }
        step_fixed(&mut app, 1);
        messages = read_messages::<BoxddJointEventMessage>(&app);
    }

    assert!(
        messages
            .iter()
            .any(|message| message.joint_id == joint_id && message.entity == Some(joint_entity)),
        "expected a zero-threshold joint event mapped to its Bevy entity"
    );
}

#[test]
fn revolute_joint_descriptor_creates_native_joint() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(0.0, 1.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::revolute(
            body_a,
            body_b,
            world_position(0.0, 0.5),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .expect("joint component should be inserted")
        .id();
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(
        context.joint_type(joint).unwrap(),
        boxdd::JointType::Revolute
    );
}

#[test]
fn changing_joint_descriptor_recreates_native_joint() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    let first_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    app.world_mut()
        .entity_mut(joint_entity)
        .insert(JointDescriptor::revolute(
            body_a,
            body_a,
            world_position(0.5, 0.0),
        ));
    step_fixed(&mut app, 1);
    assert_eq!(
        app.world()
            .entity(joint_entity)
            .get::<BoxddJoint>()
            .unwrap()
            .id(),
        first_joint,
        "failed replacement must retain the authoritative joint"
    );
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .iter()
            .any(|message| {
                message.entity == Some(joint_entity)
                    && message.operation == BoxddOperation::ReplaceJoint
            })
    );

    app.world_mut()
        .entity_mut(joint_entity)
        .insert(JointDescriptor::revolute(
            body_a,
            body_b,
            world_position(0.5, 0.0),
        ));
    step_fixed(&mut app, 1);

    let second_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_ne!(first_joint, second_joint);
    assert_eq!(
        context.joint_type(first_joint).unwrap_err(),
        BoxddPluginError::Api(boxdd::Error::InvalidJointId)
    );
    assert_eq!(
        context.joint_type(second_joint).unwrap(),
        boxdd::JointType::Revolute
    );
}

#[test]
fn retargeting_joint_to_new_body_replaces_it_in_the_same_fixed_step() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::default()))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    let old_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    let body_c = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::default()))
        .id();
    app.world_mut()
        .entity_mut(joint_entity)
        .insert(JointDescriptor::distance(
            body_a,
            body_c,
            world_position(0.0, 0.0),
            world_position(2.0, 0.0),
        ));

    step_fixed(&mut app, 1);

    let new_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    assert_ne!(new_joint, old_joint);
    {
        let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
        assert_eq!(
            context.joint_endpoint_entities(new_joint),
            Some((body_a, body_c))
        );
        assert_eq!(
            context.joint_type(old_joint).unwrap_err(),
            BoxddPluginError::Api(boxdd::Error::InvalidJointId)
        );
    }
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .iter()
            .all(|message| {
                message.entity != Some(joint_entity)
                    || message.operation != BoxddOperation::ReplaceJoint
            }),
        "same-frame retargeting must not report a transient replacement failure"
    );
}

#[test]
fn joint_created_after_bevy_transform_change_uses_fresh_native_transform() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();

    step_fixed(&mut app, 1);

    app.world_mut()
        .entity_mut(body_a)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec2::new(2.0, 0.0).extend(0.0);
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(2.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let local_frame_a = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .joint_local_frame_a(joint)
        .unwrap();

    assert_eq!(local_frame_a.position(), boxdd::Vec2::new(0.0, 0.0));
}

#[test]
fn joint_replacement_uses_the_same_frame_bevy_transform() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });
    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    let old_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    app.world_mut()
        .entity_mut(body_a)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec2::new(2.0, 0.0).extend(0.0);
    app.world_mut()
        .entity_mut(joint_entity)
        .insert(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(2.0, 0.0),
            world_position(1.0, 0.0),
        ));

    step_fixed(&mut app, 1);
    let new_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    assert_ne!(new_joint, old_joint);
    let first_local_frame = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .joint_local_frame_a(new_joint)
        .unwrap();
    assert_eq!(first_local_frame.position(), boxdd::Vec2::new(0.0, 0.0));

    step_fixed(&mut app, 1);
    let second_local_frame = app
        .world_mut()
        .non_send_mut::<BoxddPhysicsContext>()
        .joint_local_frame_a(new_joint)
        .unwrap();
    assert_eq!(second_local_frame.position(), boxdd::Vec2::new(0.0, 0.0));
}

#[test]
fn removing_joint_descriptor_destroys_native_joint() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    app.world_mut()
        .entity_mut(joint_entity)
        .remove::<JointDescriptor>();
    step_fixed(&mut app, 1);

    assert!(!app.world().entity(joint_entity).contains::<BoxddJoint>());
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(context.joint_entity(joint), None);
    assert_eq!(
        context.joint_type(joint).unwrap_err(),
        BoxddPluginError::Api(boxdd::Error::InvalidJointId)
    );
}

#[test]
fn joint_missing_endpoint_body_emits_recoverable_error() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let missing_body = app.world_mut().spawn_empty().id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::revolute(
            body_a,
            missing_body,
            world_position(0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let errors = read_messages::<BoxddErrorMessage>(&app);
    assert!(
        errors.iter().any(|message| {
            message.operation == BoxddOperation::CreateJoint
                && message.entity == Some(joint_entity)
                && message.error == BoxddPluginError::Api(boxdd::Error::InvalidBodyId)
        }),
        "expected recoverable CreateJoint error for missing endpoint, got {errors:?}"
    );
    assert!(!app.world().entity(joint_entity).contains::<BoxddJoint>());
}

#[test]
fn removing_endpoint_body_removes_dependent_joint() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let body_b = app
        .world_mut()
        .spawn((RigidBody::Dynamic, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);
    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();

    app.world_mut().entity_mut(body_b).remove::<RigidBody>();
    step_fixed(&mut app, 1);

    assert!(!app.world().entity(joint_entity).contains::<BoxddJoint>());
    let mut context = app.world_mut().non_send_mut::<BoxddPhysicsContext>();
    assert_eq!(context.joint_entity(joint), None);
    assert_eq!(
        context.joint_type(joint).unwrap_err(),
        BoxddPluginError::Api(boxdd::Error::InvalidJointId)
    );
}

#[test]
fn removing_both_endpoint_bodies_in_one_batch_retires_shared_dependents_once() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let body_a = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Collider::circle(0.5),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let body_b = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Collider::circle(0.5),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();
    let joint_entity = app
        .world_mut()
        .spawn(JointDescriptor::distance(
            body_a,
            body_b,
            world_position(0.0, 0.0),
            world_position(1.0, 0.0),
        ))
        .id();
    step_fixed(&mut app, 1);

    app.world_mut().entity_mut(body_a).remove::<RigidBody>();
    app.world_mut().entity_mut(body_b).remove::<RigidBody>();
    step_fixed(&mut app, 1);

    for body in [body_a, body_b] {
        let entity = app.world().entity(body);
        assert!(!entity.contains::<BoxddBody>());
        assert!(!entity.contains::<BoxddShape>());
    }
    assert!(!app.world().entity(joint_entity).contains::<BoxddJoint>());
    let context = app.world().non_send::<BoxddPhysicsContext>();
    let counters = context.world().unwrap().counters().unwrap();
    assert_eq!(counters.body_count, 0);
    assert_eq!(counters.shape_count, 0);
    assert_eq!(counters.joint_count, 0);
}
