use bevy_app::{App, FixedUpdate};
use bevy_boxdd::prelude::*;
use bevy_ecs::message::{Message, Messages};
use bevy_ecs::prelude::Entity;
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(BoxddPhysicsContext: Send, Sync);

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
    let hit = context
        .try_cast_ray_closest_entity(
            Vec2::new(0.0, 2.0),
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
        .try_cast_ray_all_entities_into(
            Vec2::new(0.0, 2.0),
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
        .try_cast_ray_all_entities_into(
            Vec2::new(f32::NAN, 2.0),
            Vec2::new(0.0, -4.0),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap_err();
    assert_eq!(error, boxdd::ApiError::InvalidArgument);
    assert_eq!(hits.len(), hit_count);
    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "fallible all-ray helper should preserve the caller buffer on error"
    );

    context
        .try_cast_ray_all_entities_into(
            Vec2::new(10.0, 2.0),
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
        .try_overlap_aabb_entities(
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [2.0, 1.0]),
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
        .try_overlap_aabb_entities_into(
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [2.0, 1.0]),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap();

    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "expected overlap helper to map a hit to the ground entity, got {hits:?}"
    );
    let hit_count = hits.len();
    let error = context
        .try_overlap_aabb_entities_into(
            boxdd::Aabb::new([1.0_f32, 1.0], [-1.0, -1.0]),
            boxdd::QueryFilter::default(),
            &mut hits,
        )
        .unwrap_err();
    assert_eq!(error, boxdd::ApiError::InvalidArgument);
    assert_eq!(hits.len(), hit_count);
    assert!(
        hits.iter().any(|hit| hit.entity == Some(ground)),
        "fallible overlap helper should preserve the caller buffer on error"
    );

    context
        .try_overlap_aabb_entities_into(
            boxdd::Aabb::from_center_half_extents([10.0_f32, 10.0], [1.0, 1.0]),
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
        .try_overlap_aabb_entities(
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]),
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
        .try_debug_draw_collect_into(&mut commands, boxdd::DebugDrawOptions::default())
        .unwrap();

    assert!(
        !commands.is_empty(),
        "expected debug draw collection to emit commands for plugin-created shapes"
    );
}

#[test]
fn disabled_physics_context_helpers_return_empty_results() {
    let mut context = BoxddPhysicsContext::disabled();

    let closest = context
        .try_cast_ray_closest_entity(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            boxdd::QueryFilter::default(),
        )
        .unwrap();
    assert!(closest.is_none());

    let all_hits = context
        .try_cast_ray_all_entities(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            boxdd::QueryFilter::default(),
        )
        .unwrap();
    assert!(all_hits.is_empty());

    let shape_hits = context
        .try_overlap_aabb_entities(
            boxdd::Aabb::from_center_half_extents([0.0_f32, 0.0], [1.0, 1.0]),
            boxdd::QueryFilter::default(),
        )
        .unwrap();
    assert!(shape_hits.is_empty());

    let commands = context
        .try_debug_draw_collect(boxdd::DebugDrawOptions::default())
        .unwrap();
    assert!(commands.is_empty());
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
        .try_cast_ray_all_entities(
            Vec2::new(0.0, 2.0),
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
        .try_debug_draw_collect(boxdd::DebugDrawOptions::default())
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
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .expect("joint component should be inserted")
        .id();
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(
        context.world().unwrap().try_joint_type(joint).unwrap(),
        boxdd::JointType::Distance
    );
    assert_eq!(context.joint_entity(joint), Some(joint_entity));
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
            Vec2::new(0.0, 0.5),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .expect("joint component should be inserted")
        .id();
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(
        context.world().unwrap().try_joint_type(joint).unwrap(),
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
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
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
            body_b,
            Vec2::new(0.5, 0.0),
        ));
    step_fixed(&mut app, 1);

    let second_joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let context = app.world().non_send::<BoxddPhysicsContext>();
    let world = context.world().unwrap();
    assert_ne!(first_joint, second_joint);
    assert_eq!(
        world.try_joint_type(first_joint).unwrap_err(),
        boxdd::ApiError::InvalidJointId
    );
    assert_eq!(
        world.try_joint_type(second_joint).unwrap(),
        boxdd::JointType::Revolute
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
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let joint = app
        .world()
        .entity(joint_entity)
        .get::<BoxddJoint>()
        .unwrap()
        .id();
    let context = app.world().non_send::<BoxddPhysicsContext>();
    let local_frame_a = context
        .world()
        .unwrap()
        .try_joint_local_frame_a(joint)
        .unwrap();

    assert_eq!(local_frame_a.position(), boxdd::Vec2::new(0.0, 0.0));
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
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
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
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(context.joint_entity(joint), None);
    assert_eq!(
        context.world().unwrap().try_joint_type(joint).unwrap_err(),
        boxdd::ApiError::InvalidJointId
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
            Vec2::new(0.0, 0.0),
        ))
        .id();

    step_fixed(&mut app, 1);

    let errors = read_messages::<BoxddErrorMessage>(&app);
    assert!(
        errors.iter().any(|message| {
            message.operation == BoxddOperation::CreateJoint
                && message.entity == Some(joint_entity)
                && message.error == BoxddPluginError::Api(boxdd::ApiError::InvalidBodyId)
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
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
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
    let context = app.world().non_send::<BoxddPhysicsContext>();
    assert_eq!(context.joint_entity(joint), None);
    assert_eq!(
        context.world().unwrap().try_joint_type(joint).unwrap_err(),
        boxdd::ApiError::InvalidJointId
    );
}
