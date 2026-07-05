use bevy_app::{App, FixedUpdate};
use bevy_boxdd::prelude::*;
use bevy_ecs::prelude::Entity;
use bevy_math::Vec2;
use bevy_transform::components::Transform;

fn step_fixed(app: &mut App, steps: usize) {
    for _ in 0..steps {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

#[test]
fn plugin_creates_body_shape_and_syncs_dynamic_transform() {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
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
fn static_transform_syncs_from_bevy_to_boxdd() {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::default());

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
    let context = app.world().non_send::<BoxddPhysicsContext>();
    let position = context
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap()
        .position();
    assert_eq!(position, boxdd::Vec2::new(2.0, -1.0));
}

#[test]
fn removing_rigid_body_destroys_native_handles() {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::default());

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
fn linear_impulse_is_one_shot_component() {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
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
