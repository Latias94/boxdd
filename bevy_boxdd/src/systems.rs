//! Fixed-update systems registered by [`crate::BoxddPhysicsPlugin`].

use crate::components::{
    AngularImpulse, AngularVelocity, BodySettings, BoxddBody, BoxddShape, Collider, LinearImpulse,
    LinearVelocity, PhysicsMaterial, RigidBody, TransformSyncMode,
};
use crate::errors::report_error;
use crate::math::{apply_boxdd_transform, to_boxdd_angle, to_boxdd_translation, to_boxdd_vec2};
use crate::messages::{
    BoxddBodyMoveMessage, BoxddContactBeginMessage, BoxddContactEndMessage, BoxddContactHitMessage,
    BoxddErrorMessage, BoxddOperation, BoxddPluginError, BoxddSensorBeginMessage,
    BoxddSensorEndMessage,
};
use crate::resources::{
    BoxddPhysicsContext, BoxddPhysicsSettings, ShapeDescriptor, ShapeLocalTransform,
};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Commands, Entity, NonSendMut, Query, Res, Without};
use bevy_math::Vec2 as BevyVec2;
use bevy_time::{Fixed, Time};
use bevy_transform::components::Transform;
use boxdd::{
    ApiError, ApiResult, BodyDef, BodyId, Capsule as BoxddCapsule, Circle as BoxddCircle,
    Polygon as BoxddPolygon, Segment as BoxddSegment, ShapeDef, ShapeId,
};

type MissingBodyItem<'a> = (
    Entity,
    &'a RigidBody,
    Option<&'a BodySettings>,
    Option<&'a Transform>,
    Option<&'a LinearVelocity>,
    Option<&'a AngularVelocity>,
);

type MissingShapeItem<'a> = (
    Entity,
    Option<&'a BoxddBody>,
    Option<&'a ChildOf>,
    &'a Collider,
    Option<&'a PhysicsMaterial>,
    Option<&'a Transform>,
);

type TrackedShapeItem<'a> = (
    Entity,
    &'a BoxddShape,
    Option<&'a Collider>,
    Option<&'a PhysicsMaterial>,
    Option<&'a Transform>,
    Option<&'a BoxddBody>,
    Option<&'a ChildOf>,
);

type BodyControlItem<'a> = (
    Entity,
    &'a BoxddBody,
    Option<&'a LinearVelocity>,
    Option<&'a AngularVelocity>,
    Option<&'a LinearImpulse>,
    Option<&'a AngularImpulse>,
);

type BodyTransformItem<'a> = (
    Entity,
    &'a BoxddBody,
    &'a Transform,
    Option<&'a TransformSyncMode>,
    Option<&'a RigidBody>,
);

type BodyTransformMutItem<'a> = (
    Entity,
    &'a BoxddBody,
    &'a mut Transform,
    Option<&'a TransformSyncMode>,
    Option<&'a RigidBody>,
);

/// Creates native Box2D bodies for entities with [`RigidBody`] but no [`BoxddBody`].
pub fn create_missing_bodies(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<MissingBodyItem<'_>, Without<BoxddBody>>,
) {
    if context.world().is_none() {
        return;
    }

    for (entity, rigid_body, body_settings, transform, linear_velocity, angular_velocity) in &bodies
    {
        let body_settings = body_settings.copied().unwrap_or_default();
        if let Err(error) = body_settings.validate() {
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::CreateBody,
                    entity: Some(entity),
                    error: error.into(),
                },
            );
            continue;
        }

        let mut def = BodyDef::builder()
            .body_type((*rigid_body).into())
            .gravity_scale(body_settings.gravity_scale)
            .linear_damping(body_settings.linear_damping)
            .angular_damping(body_settings.angular_damping)
            .enable_sleep(body_settings.sleep_enabled)
            .bullet(body_settings.bullet);

        if let Some(transform) = transform {
            def = def
                .position(to_boxdd_translation(transform.translation))
                .angle(to_boxdd_angle(transform.rotation));
        }

        if let Some(linear_velocity) = linear_velocity {
            def = def.linear_velocity(to_boxdd_vec2(linear_velocity.0));
        }

        if let Some(angular_velocity) = angular_velocity {
            def = def.angular_velocity(angular_velocity.0);
        }

        let result = {
            let world = context.world_mut().expect("checked above");
            world.try_create_body_id(def.build()).and_then(|body_id| {
                apply_body_settings_to_world(world, body_id, *rigid_body, body_settings)?;
                Ok(body_id)
            })
        };

        match result {
            Ok(body_id) => {
                context.insert_body(entity, body_id);
                commands.entity(entity).insert(BoxddBody(body_id));
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::CreateBody,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Applies changed or persistent runtime body settings to native bodies.
pub fn apply_body_settings(
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<(
        Entity,
        &BoxddBody,
        Option<&RigidBody>,
        Option<&BodySettings>,
    )>,
) {
    if context.world().is_none() {
        return;
    }

    for (entity, body, rigid_body, body_settings) in &bodies {
        let body_settings = body_settings.copied().unwrap_or_default();
        let rigid_body = rigid_body.copied().unwrap_or_default();
        let result = apply_body_settings_to_world(
            context.world_mut().expect("checked above"),
            body.0,
            rigid_body,
            body_settings,
        );
        apply_control_result(
            &settings,
            &mut errors,
            entity,
            BoxddOperation::ApplyBodySettings,
            result,
        );
    }
}

/// Creates native Box2D shapes for entities with [`Collider`] but no [`BoxddShape`].
///
/// Colliders may live on the body entity itself or on a child entity of a body.
pub fn create_missing_shapes(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    colliders: Query<MissingShapeItem<'_>, Without<BoxddShape>>,
    bodies: Query<&BoxddBody>,
) {
    if context.world().is_none() {
        return;
    }

    for (entity, own_body, parent, collider, material, transform) in &colliders {
        let Some((body_entity, body)) = resolve_collider_body(entity, own_body, parent, &bodies)
        else {
            continue;
        };
        let local_transform = if own_body.is_some() {
            ShapeLocalTransform::IDENTITY
        } else {
            ShapeLocalTransform::from_transform(transform)
        };
        let descriptor = ShapeDescriptor {
            collider: *collider,
            material: material.copied().unwrap_or_default(),
            local_transform,
        };
        let result = descriptor
            .collider
            .validate()
            .and_then(|()| descriptor.material.validate())
            .and_then(|()| {
                let shape_def = descriptor.material.shape_def();
                create_shape(
                    context.world_mut().expect("checked above"),
                    body.0,
                    descriptor.collider,
                    descriptor.local_transform,
                    &shape_def,
                )
            });

        match result {
            Ok(shape_id) => {
                context.insert_shape(entity, body_entity, descriptor, shape_id);
                commands.entity(entity).insert(BoxddShape(shape_id));
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::CreateShape,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Destroys or recreates native shapes when collider entities are removed or changed.
pub fn cleanup_removed_colliders(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    colliders: Query<TrackedShapeItem<'_>>,
    bodies: Query<&BoxddBody>,
) {
    if context.world().is_none() {
        return;
    }

    let stale_shape_entities = context
        .entity_to_shape
        .iter()
        .filter_map(|(entity, shape_id)| {
            colliders
                .get(*entity)
                .is_err()
                .then_some((*entity, *shape_id))
        })
        .collect::<Vec<_>>();

    for (entity, shape_id) in stale_shape_entities {
        context
            .world_mut()
            .expect("checked above")
            .destroy_shape_id(shape_id, true);
        context.remove_shape(entity, shape_id);
    }

    let mut shapes_to_recreate = Vec::new();
    for (entity, shape, collider, material, transform, own_body, parent) in &colliders {
        let current_body_entity = collider
            .is_some()
            .then(|| {
                resolve_collider_body(entity, own_body, parent, &bodies).map(|(entity, _)| entity)
            })
            .flatten();
        let tracked_body_entity = context.shape_body_entity(entity);
        let local_transform = if own_body.is_some() {
            ShapeLocalTransform::IDENTITY
        } else {
            ShapeLocalTransform::from_transform(transform)
        };
        let current_descriptor = collider.map(|collider| ShapeDescriptor {
            collider: *collider,
            material: material.copied().unwrap_or_default(),
            local_transform,
        });
        let tracked_descriptor = context.shape_descriptor(entity);

        if collider.is_some()
            && current_body_entity.is_some()
            && current_body_entity == tracked_body_entity
            && current_descriptor == tracked_descriptor
        {
            continue;
        }

        shapes_to_recreate.push((entity, shape.0));
    }

    for (entity, shape_id) in shapes_to_recreate {
        context
            .world_mut()
            .expect("checked above")
            .destroy_shape_id(shape_id, true);
        context.remove_shape(entity, shape_id);
        commands.entity(entity).remove::<BoxddShape>();
    }
}

/// Destroys native bodies when their Bevy body entities are removed or no longer have [`RigidBody`].
///
/// Shapes owned by the removed body are detached from their Bevy entities too.
pub fn cleanup_removed_bodies(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<(Entity, &BoxddBody, Option<&RigidBody>)>,
    shapes: Query<Option<&BoxddShape>>,
) {
    if context.world().is_none() {
        return;
    }

    let stale = context
        .entity_to_body
        .iter()
        .filter_map(|(entity, body_id)| {
            let should_remove = bodies
                .get(*entity)
                .map(|(_, _, rigid_body)| rigid_body.is_none())
                .unwrap_or(true);
            should_remove.then_some((*entity, *body_id))
        })
        .map(|(entity, body_id)| {
            let shape_entities = context
                .shape_to_body_entity
                .iter()
                .filter_map(|(shape_entity, body_entity)| {
                    (*body_entity == entity)
                        .then(|| {
                            shapes
                                .get(*shape_entity)
                                .ok()
                                .flatten()
                                .copied()
                                .map(|shape| (*shape_entity, shape))
                        })
                        .flatten()
                })
                .collect::<Vec<_>>();
            (entity, body_id, shape_entities)
        })
        .collect::<Vec<_>>();

    for (entity, body_id, shape_entities) in stale {
        let result = context
            .world_mut()
            .expect("checked above")
            .try_destroy_body_id(body_id);

        match result {
            Ok(()) => {
                context.remove_body(entity, body_id);
                commands.entity(entity).remove::<BoxddBody>();
                for (shape_entity, _) in shape_entities {
                    commands.entity(shape_entity).remove::<BoxddShape>();
                }
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::DestroyBody,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

fn resolve_collider_body<'a>(
    collider_entity: Entity,
    own_body: Option<&'a BoxddBody>,
    parent: Option<&ChildOf>,
    bodies: &'a Query<'_, '_, &BoxddBody>,
) -> Option<(Entity, &'a BoxddBody)> {
    if let Some(body) = own_body {
        return Some((collider_entity, body));
    }

    let parent = parent?.parent();
    bodies.get(parent).ok().map(|body| (parent, body))
}

/// Applies velocity and one-shot impulse components to native bodies.
pub fn apply_body_controls(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    controls: Query<BodyControlItem<'_>>,
) {
    if context.world().is_none() {
        return;
    }

    for (entity, body, linear_velocity, angular_velocity, linear_impulse, angular_impulse) in
        &controls
    {
        if let Some(linear_velocity) = linear_velocity {
            let result = if linear_velocity.0.is_finite() {
                context
                    .world_mut()
                    .expect("checked above")
                    .try_set_body_linear_velocity(body.0, to_boxdd_vec2(linear_velocity.0))
            } else {
                Err(ApiError::InvalidArgument)
            };
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
        }

        if let Some(angular_velocity) = angular_velocity {
            let result = if angular_velocity.0.is_finite() {
                context
                    .world_mut()
                    .expect("checked above")
                    .try_set_body_angular_velocity(body.0, angular_velocity.0)
            } else {
                Err(ApiError::InvalidArgument)
            };
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
        }

        if let Some(linear_impulse) = linear_impulse {
            let result = if linear_impulse.impulse.is_finite() {
                context
                    .world_mut()
                    .expect("checked above")
                    .try_body_apply_linear_impulse_to_center(
                        body.0,
                        to_boxdd_vec2(linear_impulse.impulse),
                        linear_impulse.wake,
                    )
            } else {
                Err(ApiError::InvalidArgument)
            };
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
            commands.entity(entity).remove::<LinearImpulse>();
        }

        if let Some(angular_impulse) = angular_impulse {
            let result = if angular_impulse.impulse.is_finite() {
                context
                    .world_mut()
                    .expect("checked above")
                    .try_body_apply_angular_impulse(
                        body.0,
                        angular_impulse.impulse,
                        angular_impulse.wake,
                    )
            } else {
                Err(ApiError::InvalidArgument)
            };
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
            commands.entity(entity).remove::<AngularImpulse>();
        }
    }
}

/// Writes Bevy transforms into Box2D for bodies using [`TransformSyncMode::BevyToPhysics`].
pub fn sync_bevy_transforms_to_boxdd(
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<BodyTransformItem<'_>>,
) {
    if context.world().is_none() {
        return;
    }

    for (entity, body, transform, sync_mode, rigid_body) in &bodies {
        if effective_sync_mode(sync_mode, rigid_body) != TransformSyncMode::BevyToPhysics {
            continue;
        }

        let result = context
            .world_mut()
            .expect("checked above")
            .try_set_body_position_and_rotation(
                body.0,
                to_boxdd_translation(transform.translation),
                to_boxdd_angle(transform.rotation),
            );

        if let Err(error) = result {
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::SyncTransform,
                    entity: Some(entity),
                    error: error.into(),
                },
            );
        }
    }
}

/// Advances the Box2D world by Bevy's fixed timestep.
pub fn step_world(
    mut context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    time: Res<Time<Fixed>>,
    mut errors: MessageWriter<BoxddErrorMessage>,
) {
    let Some(world) = context.world_mut() else {
        return;
    };

    let configured_step = settings
        .fixed_timestep_seconds
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .unwrap_or(1.0 / 60.0) as f32;
    let time_step = if time.delta_secs() > 0.0 {
        time.delta_secs()
    } else {
        configured_step
    };

    match world.try_step(time_step, settings.sub_step_count) {
        Ok(()) => {
            context.last_step_failed = false;
        }
        Err(error) => {
            context.last_step_failed = true;
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::StepWorld,
                    entity: None,
                    error: error.into(),
                },
            );
        }
    }
}

/// Publishes body, contact, and sensor messages produced by the last successful step.
#[allow(clippy::too_many_arguments)]
pub fn publish_physics_messages(
    context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut body_moves: MessageWriter<BoxddBodyMoveMessage>,
    mut contact_begin: MessageWriter<BoxddContactBeginMessage>,
    mut contact_end: MessageWriter<BoxddContactEndMessage>,
    mut contact_hit: MessageWriter<BoxddContactHitMessage>,
    mut sensor_begin: MessageWriter<BoxddSensorBeginMessage>,
    mut sensor_end: MessageWriter<BoxddSensorEndMessage>,
) {
    if context.last_step_failed {
        return;
    }

    let Some(world) = context.world() else {
        return;
    };

    match world.try_body_events() {
        Ok(events) => {
            for event in events {
                body_moves.write(BoxddBodyMoveMessage {
                    body_id: event.body_id,
                    entity: context.body_entity(event.body_id),
                    transform: event.transform,
                    fell_asleep: event.fell_asleep,
                });
            }
        }
        Err(error) => report_error(
            &settings,
            &mut errors,
            BoxddErrorMessage {
                operation: BoxddOperation::ReadEvents,
                entity: None,
                error: error.into(),
            },
        ),
    }

    match world.try_contact_events() {
        Ok(events) => {
            for event in events.begin {
                contact_begin.write(BoxddContactBeginMessage {
                    shape_a: event.shape_a,
                    shape_b: event.shape_b,
                    entity_a: context.shape_entity(event.shape_a),
                    entity_b: context.shape_entity(event.shape_b),
                    contact_id: event.contact_id,
                });
            }

            for event in events.end {
                contact_end.write(BoxddContactEndMessage {
                    shape_a: event.shape_a,
                    shape_b: event.shape_b,
                    entity_a: context.shape_entity(event.shape_a),
                    entity_b: context.shape_entity(event.shape_b),
                });
            }

            for event in events.hit {
                contact_hit.write(BoxddContactHitMessage {
                    shape_a: event.shape_a,
                    shape_b: event.shape_b,
                    entity_a: context.shape_entity(event.shape_a),
                    entity_b: context.shape_entity(event.shape_b),
                    point: event.point,
                    normal: event.normal,
                    approach_speed: event.approach_speed,
                });
            }
        }
        Err(error) => report_error(
            &settings,
            &mut errors,
            BoxddErrorMessage {
                operation: BoxddOperation::ReadEvents,
                entity: None,
                error: error.into(),
            },
        ),
    }

    match world.try_sensor_events() {
        Ok(events) => {
            for event in events.begin {
                sensor_begin.write(BoxddSensorBeginMessage {
                    sensor_shape: event.sensor_shape,
                    visitor_shape: event.visitor_shape,
                    sensor_entity: context.shape_entity(event.sensor_shape),
                    visitor_entity: context.shape_entity(event.visitor_shape),
                });
            }

            for event in events.end {
                sensor_end.write(BoxddSensorEndMessage {
                    sensor_shape: event.sensor_shape,
                    visitor_shape: event.visitor_shape,
                    sensor_entity: context.shape_entity(event.sensor_shape),
                    visitor_entity: context.shape_entity(event.visitor_shape),
                });
            }
        }
        Err(error) => report_error(
            &settings,
            &mut errors,
            BoxddErrorMessage {
                operation: BoxddOperation::ReadEvents,
                entity: None,
                error: error.into(),
            },
        ),
    }
}

/// Writes Box2D transforms into Bevy for bodies using [`TransformSyncMode::PhysicsToBevy`].
pub fn sync_boxdd_transforms_to_bevy(
    context: NonSendMut<BoxddPhysicsContext>,
    settings: Res<BoxddPhysicsSettings>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut bodies: Query<BodyTransformMutItem<'_>>,
) {
    if context.last_step_failed || context.world().is_none() {
        return;
    }

    for (entity, body, mut transform, sync_mode, rigid_body) in &mut bodies {
        if effective_sync_mode(sync_mode, rigid_body) != TransformSyncMode::PhysicsToBevy {
            continue;
        }

        let result = context
            .world()
            .expect("checked above")
            .try_body_transform(body.0);

        match result {
            Ok(boxdd_transform) => apply_boxdd_transform(&mut transform, boxdd_transform),
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::SyncTransform,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

fn apply_body_settings_to_world(
    world: &mut boxdd::World,
    body_id: BodyId,
    rigid_body: RigidBody,
    settings: BodySettings,
) -> ApiResult<()> {
    settings.validate()?;
    {
        let mut body = world.try_body(body_id)?;
        body.try_set_body_type(rigid_body.into())?;
        body.try_set_gravity_scale(settings.gravity_scale)?;
        body.try_set_linear_damping(settings.linear_damping)?;
        body.try_set_angular_damping(settings.angular_damping)?;
        body.try_enable_sleep(settings.sleep_enabled)?;
        body.try_set_bullet(settings.bullet)?;
    }
    world.try_set_body_motion_locks(body_id, settings.motion_locks)
}

fn create_shape(
    world: &mut boxdd::World,
    body_id: BodyId,
    collider: Collider,
    local_transform: ShapeLocalTransform,
    shape_def: &ShapeDef,
) -> ApiResult<ShapeId> {
    match collider {
        Collider::Circle { radius, center } => {
            let circle = BoxddCircle::new(
                to_boxdd_vec2(transform_local_point(local_transform, center)),
                radius,
            );
            world.try_create_circle_shape_for(body_id, shape_def, &circle)
        }
        Collider::Capsule {
            point1,
            point2,
            radius,
        } => {
            let capsule = BoxddCapsule::new(
                to_boxdd_vec2(transform_local_point(local_transform, point1)),
                to_boxdd_vec2(transform_local_point(local_transform, point2)),
                radius,
            );
            world.try_create_capsule_shape_for(body_id, shape_def, &capsule)
        }
        Collider::Segment { point1, point2 } => {
            let segment = BoxddSegment::new(
                to_boxdd_vec2(transform_local_point(local_transform, point1)),
                to_boxdd_vec2(transform_local_point(local_transform, point2)),
            );
            world.try_create_segment_shape_for(body_id, shape_def, &segment)
        }
        Collider::Rectangle { half_extents } => {
            let polygon = if local_transform == ShapeLocalTransform::IDENTITY {
                BoxddPolygon::try_box_polygon(half_extents.x, half_extents.y)?
            } else {
                BoxddPolygon::try_offset_box_polygon(
                    half_extents.x,
                    half_extents.y,
                    to_boxdd_local_transform(local_transform),
                )?
            };
            world.try_create_polygon_shape_for(body_id, shape_def, &polygon)
        }
        Collider::RoundedRectangle {
            half_extents,
            radius,
        } => {
            let polygon = if local_transform == ShapeLocalTransform::IDENTITY {
                BoxddPolygon::try_rounded_box_polygon(half_extents.x, half_extents.y, radius)?
            } else {
                BoxddPolygon::try_offset_rounded_box_polygon(
                    half_extents.x,
                    half_extents.y,
                    radius,
                    to_boxdd_local_transform(local_transform),
                )?
            };
            world.try_create_polygon_shape_for(body_id, shape_def, &polygon)
        }
        Collider::ConvexPolygon {
            vertices,
            count,
            radius,
        } => {
            let points = vertices[..count as usize]
                .iter()
                .map(|point| to_boxdd_vec2(transform_local_point(local_transform, *point)));
            let polygon = BoxddPolygon::try_from_points(points, radius)?;
            world.try_create_polygon_shape_for(body_id, shape_def, &polygon)
        }
    }
}

fn apply_control_result(
    settings: &BoxddPhysicsSettings,
    errors: &mut MessageWriter<'_, BoxddErrorMessage>,
    entity: Entity,
    operation: BoxddOperation,
    result: ApiResult<()>,
) {
    if let Err(error) = result {
        report_error(
            settings,
            errors,
            BoxddErrorMessage {
                operation,
                entity: Some(entity),
                error: BoxddPluginError::Api(error),
            },
        );
    }
}

fn effective_sync_mode(
    mode: Option<&TransformSyncMode>,
    rigid_body: Option<&RigidBody>,
) -> TransformSyncMode {
    mode.copied().unwrap_or(match rigid_body.copied() {
        Some(RigidBody::Static | RigidBody::Kinematic) => TransformSyncMode::BevyToPhysics,
        Some(RigidBody::Dynamic) | None => TransformSyncMode::PhysicsToBevy,
    })
}

fn to_boxdd_local_transform(value: ShapeLocalTransform) -> boxdd::Transform {
    boxdd::Transform::from_pos_angle(to_boxdd_vec2(value.translation), value.angle)
}

fn transform_local_point(transform: ShapeLocalTransform, point: BevyVec2) -> BevyVec2 {
    let (sin, cos) = transform.angle.sin_cos();
    BevyVec2::new(cos * point.x - sin * point.y, sin * point.x + cos * point.y)
        + transform.translation
}
