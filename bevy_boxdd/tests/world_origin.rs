use bevy_app::{App, FixedUpdate};
use bevy_boxdd::prelude::*;
use bevy_ecs::message::{Message, Messages};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_transform::components::Transform;

fn step_fixed(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn physics_app() -> App {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    app
}

fn read_messages<M>(app: &App) -> Vec<M>
where
    M: Message + Clone,
{
    let messages = app.world().resource::<Messages<M>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).cloned().collect()
}

fn authored_transform(x: f32, y: f32, z: f32, angle: f32, scale: Vec3) -> Transform {
    Transform::from_xyz(x, y, z)
        .with_rotation(Quat::from_rotation_z(angle))
        .with_scale(scale)
}

#[cfg(not(feature = "double-precision"))]
#[test]
fn checked_conversions_round_trip_near_origin_in_single_precision() {
    let absolute_origin = boxdd::Position::from([1_024.0_f32, -2_048.0]);
    let origin = BoxddWorldOrigin::try_new(absolute_origin).unwrap();
    let local = Vec2::new(12.5, -0.125);

    let absolute = origin.checked_local_to_absolute(local).unwrap();
    assert_eq!(origin.checked_absolute_to_local(absolute).unwrap(), local);
    assert_eq!(
        absolute,
        absolute_origin.offset(boxdd::Vec2::new(local.x, local.y))
    );
}

#[cfg(feature = "double-precision")]
#[test]
fn double_precision_preserves_millimeter_offsets_at_ten_megameters() {
    let absolute_origin = boxdd::Position::new(10_000_000.0, -10_000_000.0);
    let origin = BoxddWorldOrigin::try_new(absolute_origin).unwrap();
    let local = Vec2::new(0.001, -0.001);
    let absolute = origin.checked_local_to_absolute(local).unwrap();

    assert!(absolute.x > absolute_origin.x);
    assert!(absolute.y < absolute_origin.y);
    let round_trip = origin.checked_absolute_to_local(absolute).unwrap();
    assert!((round_trip.x - local.x).abs() <= f32::EPSILON);
    assert!((round_trip.y - local.y).abs() <= f32::EPSILON);

    let mut app = App::new();
    app.insert_resource(origin);
    app.add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
        ..Default::default()
    }));
    let entity = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            TransformSyncMode::None,
            Transform::from_xyz(local.x, local.y, 0.0),
        ))
        .id();

    step_fixed(&mut app);

    let body = app.world().entity(entity).get::<BoxddBody>().unwrap().id();
    let native_position = app
        .world()
        .non_send::<BoxddPhysicsContext>()
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap()
        .position();
    let native_local = native_position
        .checked_relative_to(absolute_origin)
        .unwrap();
    assert!((native_local.x - local.x).abs() <= f32::EPSILON);
    assert!((native_local.y - local.y).abs() <= f32::EPSILON);

    let rebased_origin = boxdd::Position::new(10_000_001.0, -10_000_001.0);
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(rebased_origin)
        .unwrap();
    step_fixed(&mut app);

    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), rebased_origin);
    assert_eq!(origin.revision(), 1);
    let transform = app.world().entity(entity).get::<Transform>().unwrap();
    assert!((transform.translation.x - (local.x - 1.0)).abs() <= f32::EPSILON);
    assert!((transform.translation.y - (local.y + 1.0)).abs() <= f32::EPSILON);
    let native_after_rebase = app
        .world()
        .non_send::<BoxddPhysicsContext>()
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap()
        .position();
    assert_eq!(native_after_rebase, native_position);
}

#[test]
fn successful_rebase_is_atomic_across_sync_modes_and_uncreated_bodies() {
    let mut app = physics_app();
    let physics_to_bevy = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            TransformSyncMode::PhysicsToBevy,
            authored_transform(1.0, 2.0, 3.0, 0.25, Vec3::new(1.2, 1.3, 1.4)),
        ))
        .id();
    let bevy_to_physics = app
        .world_mut()
        .spawn((
            RigidBody::Kinematic,
            TransformSyncMode::BevyToPhysics,
            authored_transform(-4.0, 5.0, 6.0, -0.5, Vec3::new(0.8, 0.9, 1.0)),
        ))
        .id();
    let no_sync = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            TransformSyncMode::None,
            authored_transform(7.0, -8.0, 9.0, 0.75, Vec3::new(1.5, 1.6, 1.7)),
        ))
        .id();
    step_fixed(&mut app);

    let existing = [physics_to_bevy, bevy_to_physics, no_sync];
    let native_before = existing.map(|entity| {
        let body = app.world().entity(entity).get::<BoxddBody>().unwrap().id();
        let position = app
            .world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .try_body_transform(body)
            .unwrap()
            .position();
        (body, position)
    });

    let uncreated = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            authored_transform(11.0, 12.0, 13.0, -0.9, Vec3::new(2.0, 2.1, 2.2)),
        ))
        .id();
    let all_entities = [physics_to_bevy, bevy_to_physics, no_sync, uncreated];
    let transforms_before =
        all_entities.map(|entity| *app.world().entity(entity).get::<Transform>().unwrap());
    let target = boxdd::Position::from([100.0_f32, -50.0]);
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(target)
        .unwrap();

    step_fixed(&mut app);

    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), target);
    assert_eq!(origin.revision(), 1);
    assert_eq!(origin.pending(), None);

    for (entity, before) in all_entities.into_iter().zip(transforms_before) {
        let after = app.world().entity(entity).get::<Transform>().unwrap();
        assert_eq!(after.translation.x, before.translation.x - 100.0);
        assert_eq!(after.translation.y, before.translation.y + 50.0);
        assert_eq!(after.translation.z, before.translation.z);
        assert_eq!(after.rotation, before.rotation);
        assert_eq!(after.scale, before.scale);
    }

    for (body, position_before) in native_before {
        let position_after = app
            .world()
            .non_send::<BoxddPhysicsContext>()
            .world()
            .unwrap()
            .try_body_transform(body)
            .unwrap()
            .position();
        assert_eq!(position_after, position_before);
    }
    let new_body = app
        .world()
        .entity(uncreated)
        .get::<BoxddBody>()
        .expect("body creation must run after a successful rebase")
        .id();
    let new_position = app
        .world()
        .non_send::<BoxddPhysicsContext>()
        .world()
        .unwrap()
        .try_body_transform(new_body)
        .unwrap()
        .position();
    assert_eq!(new_position, boxdd::Position::from([11.0_f32, 12.0]));

    let events = read_messages::<WorldOriginRebased>(&app);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].previous, boxdd::Position::ZERO);
    assert_eq!(events[0].current, target);
    assert_eq!(events[0].revision, 1);
}

#[test]
fn failed_rebase_leaves_ecs_and_native_world_unchanged() {
    let mut app = physics_app();
    let existing = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            LinearVelocity(Vec2::new(3.0, 0.0)),
            Transform::from_xyz(1.0, 2.0, 0.0),
        ))
        .id();
    step_fixed(&mut app);

    let body = app
        .world()
        .entity(existing)
        .get::<BoxddBody>()
        .unwrap()
        .id();
    let native_before = app
        .world()
        .non_send::<BoxddPhysicsContext>()
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap();
    let valid_uncreated = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(5.0, 6.0, 7.0)))
        .id();
    let invalid_uncreated = app
        .world_mut()
        .spawn((RigidBody::Static, Transform::from_xyz(f32::MAX, 0.0, 8.0)))
        .id();
    let entities = [existing, valid_uncreated, invalid_uncreated];
    let transforms_before =
        entities.map(|entity| *app.world().entity(entity).get::<Transform>().unwrap());
    let target = boxdd::Position::from([-f32::MAX, 0.0]);
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(target)
        .unwrap();

    step_fixed(&mut app);

    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), boxdd::Position::ZERO);
    assert_eq!(origin.revision(), 0);
    assert_eq!(origin.pending(), Some(target));
    for (entity, before) in entities.into_iter().zip(transforms_before) {
        assert_eq!(
            app.world().entity(entity).get::<Transform>().unwrap(),
            &before
        );
    }
    let native_after = app
        .world()
        .non_send::<BoxddPhysicsContext>()
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap();
    assert_eq!(native_after.position(), native_before.position());
    assert_eq!(
        native_after.rotation().angle(),
        native_before.rotation().angle()
    );
    assert!(!app.world().entity(valid_uncreated).contains::<BoxddBody>());
    assert!(
        !app.world()
            .entity(invalid_uncreated)
            .contains::<BoxddBody>()
    );
    assert!(read_messages::<WorldOriginRebased>(&app).is_empty());

    let errors = read_messages::<BoxddErrorMessage>(&app);
    assert!(errors.iter().any(|message| {
        message.operation == BoxddOperation::RebaseWorldOrigin
            && message.entity == Some(invalid_uncreated)
            && message.error
                == BoxddPluginError::WorldOrigin(BoxddWorldOriginError::LocalPositionOutOfRange)
    }));
}

#[test]
fn invalid_physics_rotation_prevents_rebase_before_body_creation() {
    let mut app = physics_app();
    let mut transform = Transform::from_xyz(2.0, 3.0, 4.0);
    transform.rotation = Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0);
    let entity = app.world_mut().spawn((RigidBody::Dynamic, transform)).id();
    let target = boxdd::Position::from([10.0_f32, 20.0]);
    app.world_mut()
        .resource_mut::<BoxddWorldOrigin>()
        .request_rebase(target)
        .unwrap();

    step_fixed(&mut app);

    let origin = app.world().resource::<BoxddWorldOrigin>();
    assert_eq!(origin.active(), boxdd::Position::ZERO);
    assert_eq!(origin.pending(), Some(target));
    let after = app.world().entity(entity).get::<Transform>().unwrap();
    assert_eq!(after.translation, transform.translation);
    assert!(!after.rotation.is_finite());
    assert_eq!(after.scale, transform.scale);
    assert!(!app.world().entity(entity).contains::<BoxddBody>());
    assert!(
        read_messages::<BoxddErrorMessage>(&app)
            .iter()
            .any(|message| {
                message.operation == BoxddOperation::RebaseWorldOrigin
                    && message.entity == Some(entity)
                    && message.error
                        == BoxddPluginError::WorldOrigin(BoxddWorldOriginError::InvalidRotation)
            })
    );
}
