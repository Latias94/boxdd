use bevy_app::{App, FixedUpdate};
use bevy_boxdd::prelude::*;
use bevy_ecs::message::{Message, Messages};
use bevy_ecs::prelude::Entity;
use bevy_math::Vec2;
use bevy_transform::components::Transform;

fn step_fixed(app: &mut App, steps: usize) {
    for _ in 0..steps {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn app_with_settings(settings: BoxddPhysicsSettings) -> App {
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::new(settings));
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

#[test]
fn contact_messages_include_entity_mappings() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
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

    step_fixed(&mut app, 4);

    let contacts = read_messages::<BoxddContactBeginMessage>(&app);
    assert!(
        contacts.iter().any(|message| matches_pair(
            message.entity_a,
            message.entity_b,
            ground,
            box_entity
        )),
        "expected at least one contact begin message mapped to both Bevy entities, got {contacts:?}"
    );
}

#[test]
fn sensor_messages_include_begin_end_entity_mappings() {
    let mut app = app_with_settings(BoxddPhysicsSettings {
        gravity: Vec2::ZERO,
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
                    && message.error == BoxddPluginError::Api(boxdd::ApiError::InvalidArgument)
            }),
            "expected a recoverable CreateShape error for {entity:?}, got {errors:?}"
        );
        assert!(!app.world().entity(entity).contains::<BoxddShape>());
    }
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
    let world = context.world().expect("physics world should be available");
    let hit = world.cast_ray_closest(
        boxdd::Vec2::new(0.0, 2.0),
        boxdd::Vec2::new(0.0, -4.0),
        boxdd::QueryFilter::default(),
    );

    assert!(hit.hit, "expected the ray to hit the plugin-created ground");
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
    let context = app.world().non_send::<BoxddPhysicsContext>();
    let position = context
        .world()
        .unwrap()
        .try_body_transform(body)
        .unwrap()
        .position();

    assert_eq!(position, boxdd::Vec2::new(2.0, 1.5));
}
