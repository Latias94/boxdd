//! Fixed-update systems registered by [`crate::BoxddPhysicsPlugin`].

use crate::components::{
    AngularImpulse, AngularVelocity, BodySettings, BoxddBody, BoxddJoint, BoxddShape, Collider,
    JointDescriptor, LinearImpulse, LinearVelocity, PhysicsMaterial, RigidBody, TransformSyncMode,
};
use crate::errors::report_error;
use crate::math::to_boxdd_vec2;
use crate::messages::{
    BoxddBodyMoveMessage, BoxddContactBeginMessage, BoxddContactEndMessage, BoxddContactHitMessage,
    BoxddErrorMessage, BoxddJointEventMessage, BoxddOperation, BoxddPluginError,
    BoxddSensorBeginMessage, BoxddSensorEndMessage, WorldOriginRebased,
};
use crate::origin::{BoxddWorldOrigin, BoxddWorldOriginError};
use crate::resources::{
    BodyDescriptor, BoxddEcsWorldBindingState, BoxddErrorPolicy, BoxddEventInterests,
    BoxddPhysicsContext, BoxddStepSettings, ShapeDescriptor, ShapeLocalTransform, StepEventErrors,
};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{
    Added, Changed, Commands, DetectChanges, Entity, Local, NonSend, NonSendMut, Or, ParamSet,
    Query, Ref, RemovedComponents, Res, ResMut, With, Without,
};
use bevy_ecs::system::SystemParam;
use bevy_ecs::world::WorldId;
use bevy_time::{Fixed, Time};
use bevy_transform::components::Transform;
use boxdd::{BodyBuilder, Error as BoxddError, Result as BoxddResult, WorldTransform};
use std::sync::Arc;

type MissingBodyItem<'a> = (
    Entity,
    Ref<'a, RigidBody>,
    Option<Ref<'a, ChildOf>>,
    Option<&'a BodySettings>,
    Option<&'a Transform>,
    Option<&'a LinearVelocity>,
    Option<&'a AngularVelocity>,
);

type MissingShapeItem<'a> = (
    Entity,
    Option<&'a ChildOf>,
    &'a Collider,
    Option<&'a PhysicsMaterial>,
    Option<&'a Transform>,
);

type TrackedShapeItem<'a> = (
    Option<&'a Collider>,
    Option<&'a PhysicsMaterial>,
    Option<&'a Transform>,
    Option<&'a ChildOf>,
);

type MissingJointItem<'a> = (Entity, &'a JointDescriptor);

type TrackedJointItem<'a> = Option<&'a JointDescriptor>;

#[derive(SystemParam)]
pub(crate) struct ContextBindingGuard<'w> {
    actual_world: WorldId,
    binding: Res<'w, BoxddEcsWorldBindingState>,
}

impl ContextBindingGuard<'_> {
    fn allows(&self, context: &BoxddPhysicsContext) -> bool {
        self.binding.allows_context(self.actual_world, context)
    }
}

type BodyControlItem<'a> = (
    Entity,
    &'a BoxddBody,
    Option<&'a LinearVelocity>,
    Option<&'a AngularVelocity>,
    Option<&'a LinearImpulse>,
    Option<&'a AngularImpulse>,
);

type BodyControlMembershipChanged = Or<(
    Added<BoxddBody>,
    Added<LinearVelocity>,
    Added<AngularVelocity>,
    Added<LinearImpulse>,
    Added<AngularImpulse>,
)>;

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

type BodyTransformMembershipChanged = Or<(Added<BoxddBody>, Added<Transform>)>;

type RebaseFailure = (boxdd::Position, Option<Entity>, BoxddWorldOriginError);

#[derive(Default)]
pub(crate) struct EntityOrderCache {
    initialized: bool,
    entities: Vec<Entity>,
}

#[derive(Default)]
pub(crate) struct ProjectionReconcileState {
    context_identity: Option<Arc<()>>,
    needs_full_scan: bool,
}

impl ProjectionReconcileState {
    fn invalidate(&mut self) {
        self.needs_full_scan = true;
    }

    fn begin(&mut self, context_identity: &Arc<()>) -> bool {
        let context_changed = self
            .context_identity
            .as_ref()
            .is_none_or(|previous| !Arc::ptr_eq(previous, context_identity));
        if context_changed {
            self.context_identity = Some(Arc::clone(context_identity));
        }
        let needs_full_scan = std::mem::take(&mut self.needs_full_scan);
        context_changed || needs_full_scan
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldBindingFailure {
    ContextUnavailable,
    WrongWorld { expected: WorldId, actual: WorldId },
    WorldOriginUnavailable,
    WorldOriginStateMismatch,
}

/// Validates the non-send context before any ECS or native physics operation runs.
pub(crate) fn validate_context_world_binding(
    actual_world: WorldId,
    context: Option<NonSend<BoxddPhysicsContext>>,
    origin: Option<Res<BoxddWorldOrigin>>,
    mut binding: ResMut<BoxddEcsWorldBindingState>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut last_failure: Local<Option<WorldBindingFailure>>,
) {
    let failure = match (context.as_deref(), origin.as_deref()) {
        (None, _) => Some(WorldBindingFailure::ContextUnavailable),
        (Some(context), _) if context.owner_world() != actual_world => {
            Some(WorldBindingFailure::WrongWorld {
                expected: context.owner_world(),
                actual: actual_world,
            })
        }
        (Some(_), None) => Some(WorldBindingFailure::WorldOriginUnavailable),
        (Some(context), Some(origin)) if !context.world_origin_matches(origin) => {
            Some(WorldBindingFailure::WorldOriginStateMismatch)
        }
        (Some(_), Some(_)) => None,
    };
    match (failure, context.as_deref(), origin.as_deref()) {
        (None, Some(context), Some(origin)) => binding.validate(actual_world, origin, context),
        _ => binding.invalidate(),
    }

    if *last_failure == failure {
        return;
    }
    *last_failure = failure;

    let Some(failure) = failure else {
        return;
    };
    let error = match failure {
        WorldBindingFailure::ContextUnavailable => BoxddPluginError::ContextUnavailable,
        WorldBindingFailure::WrongWorld { expected, actual } => {
            BoxddPluginError::WrongEcsWorld { expected, actual }
        }
        WorldBindingFailure::WorldOriginUnavailable => BoxddPluginError::WorldOriginUnavailable,
        WorldBindingFailure::WorldOriginStateMismatch => BoxddPluginError::WorldOriginStateMismatch,
    };
    report_error(
        &settings,
        &mut errors,
        BoxddErrorMessage {
            operation: BoxddOperation::ValidateWorldBinding,
            entity: None,
            error,
        },
    );
}

pub(crate) fn context_world_binding_is_valid(
    actual_world: WorldId,
    binding: Res<BoxddEcsWorldBindingState>,
    origin: Option<Res<BoxddWorldOrigin>>,
) -> bool {
    binding.allows_origin(actual_world, origin.as_deref())
}

/// Reconciles opaque runtime projections from the context's authoritative maps.
///
/// Components may be moved between entities through safe ECS operations. Such a move never
/// transfers native ownership: the projection is restored on the authoritative entity and
/// removed from every other entity before any physics mutation system runs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn reconcile_identity_projections(
    mut commands: Commands,
    context: Option<NonSend<BoxddPhysicsContext>>,
    binding: ContextBindingGuard<'_>,
    body_projections: Query<(Entity, &BoxddBody)>,
    shape_projections: Query<(Entity, &BoxddShape)>,
    joint_projections: Query<(Entity, &BoxddJoint)>,
    changed_body_projections: Query<(Entity, &BoxddBody), Changed<BoxddBody>>,
    changed_shape_projections: Query<(Entity, &BoxddShape), Changed<BoxddShape>>,
    changed_joint_projections: Query<(Entity, &BoxddJoint), Changed<BoxddJoint>>,
    mut removed_bodies: RemovedComponents<BoxddBody>,
    mut removed_shapes: RemovedComponents<BoxddShape>,
    mut removed_joints: RemovedComponents<BoxddJoint>,
    mut state: Local<ProjectionReconcileState>,
) {
    let Some(context) = context else {
        state.invalidate();
        removed_bodies.clear();
        removed_shapes.clear();
        removed_joints.clear();
        return;
    };
    if !binding.allows(&context) {
        state.invalidate();
        removed_bodies.clear();
        removed_shapes.clear();
        removed_joints.clear();
        return;
    }

    // Immutable projection changes and removals are sufficient after a baseline scan. A context
    // replacement or an invalid binding discards that baseline and rebuilds it once.
    if !state.begin(context.identity_token()) {
        for (entity, actual) in &changed_body_projections {
            if context.body_projection(entity, actual).is_some() {
                continue;
            }
            if let Some(expected) = context.authoritative_body(entity) {
                commands.entity(entity).insert(BoxddBody::new(expected));
            } else {
                commands.entity(entity).remove::<BoxddBody>();
            }
        }
        for entity in removed_bodies.read() {
            let Some(expected) = context.authoritative_body(entity) else {
                continue;
            };
            let projection_matches = matches!(
                body_projections.get(entity),
                Ok((_, actual)) if context.body_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddBody::new(expected));
            }
        }

        for (entity, actual) in &changed_shape_projections {
            if context.shape_projection(entity, actual).is_some() {
                continue;
            }
            if let Some(expected) = context.authoritative_shape(entity) {
                commands.entity(entity).insert(BoxddShape::new(expected));
            } else {
                commands.entity(entity).remove::<BoxddShape>();
            }
        }
        for entity in removed_shapes.read() {
            let Some(expected) = context.authoritative_shape(entity) else {
                continue;
            };
            let projection_matches = matches!(
                shape_projections.get(entity),
                Ok((_, actual)) if context.shape_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddShape::new(expected));
            }
        }

        for (entity, actual) in &changed_joint_projections {
            if context.joint_projection(entity, actual).is_some() {
                continue;
            }
            if let Some(expected) = context.authoritative_joint(entity) {
                commands.entity(entity).insert(BoxddJoint::new(expected));
            } else {
                commands.entity(entity).remove::<BoxddJoint>();
            }
        }
        for entity in removed_joints.read() {
            let Some(expected) = context.authoritative_joint(entity) else {
                continue;
            };
            let projection_matches = matches!(
                joint_projections.get(entity),
                Ok((_, actual)) if context.joint_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddJoint::new(expected));
            }
        }
        return;
    }

    removed_bodies.clear();
    removed_shapes.clear();
    removed_joints.clear();

    let mut valid_body_projections = 0;
    for (entity, actual) in &body_projections {
        if context.body_projection(entity, actual).is_some() {
            valid_body_projections += 1;
        } else if context.authoritative_body(entity).is_none() {
            commands.entity(entity).remove::<BoxddBody>();
        }
    }
    for_each_authoritative_if_incomplete(
        valid_body_projections,
        context.tracked_bodies(),
        |(entity, expected)| {
            if context.authoritative_body(entity) != Some(expected) {
                return;
            }
            let projection_matches = matches!(
                body_projections.get(entity),
                Ok((_, actual)) if context.body_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddBody::new(expected));
            }
        },
    );

    let mut valid_shape_projections = 0;
    for (entity, actual) in &shape_projections {
        if context.shape_projection(entity, actual).is_some() {
            valid_shape_projections += 1;
        } else if context.authoritative_shape(entity).is_none() {
            commands.entity(entity).remove::<BoxddShape>();
        }
    }
    for_each_authoritative_if_incomplete(
        valid_shape_projections,
        context.tracked_shapes(),
        |(entity, expected)| {
            if context.authoritative_shape(entity) != Some(expected) {
                return;
            }
            let projection_matches = matches!(
                shape_projections.get(entity),
                Ok((_, actual)) if context.shape_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddShape::new(expected));
            }
        },
    );

    let mut valid_joint_projections = 0;
    for (entity, actual) in &joint_projections {
        if context.joint_projection(entity, actual).is_some() {
            valid_joint_projections += 1;
        } else if context.authoritative_joint(entity).is_none() {
            commands.entity(entity).remove::<BoxddJoint>();
        }
    }
    for_each_authoritative_if_incomplete(
        valid_joint_projections,
        context.tracked_joints(),
        |(entity, expected)| {
            if context.authoritative_joint(entity) != Some(expected) {
                return;
            }
            let projection_matches = matches!(
                joint_projections.get(entity),
                Ok((_, actual)) if context.joint_projection(entity, actual) == Some(expected)
            );
            if !projection_matches && let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(BoxddJoint::new(expected));
            }
        },
    );
}

/// Stages and atomically commits a requested world-origin rebase.
///
/// A failed request remains pending and prevents the physics pipeline from
/// advancing until the caller fixes, replaces, or cancels it.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn apply_pending_world_origin_rebase(
    actual_world: WorldId,
    mut context: NonSendMut<BoxddPhysicsContext>,
    mut origin: ResMut<BoxddWorldOrigin>,
    mut binding: ResMut<BoxddEcsWorldBindingState>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut rebased: MessageWriter<WorldOriginRebased>,
    mut last_failure: Local<Option<RebaseFailure>>,
    mut bodies: ParamSet<(
        Query<(Entity, &Transform), With<RigidBody>>,
        Query<&mut Transform, With<RigidBody>>,
    )>,
) {
    if !binding.allows_context(actual_world, &context)
        || !binding.allows_origin(actual_world, Some(&origin))
    {
        return;
    }

    let Some(target) = origin.pending() else {
        *last_failure = None;
        return;
    };

    let previous = origin.active();
    let staged = (|| {
        let revision = origin.next_revision().map_err(|error| (None, error))?;
        let target_frame = BoxddWorldOrigin::new(target).map_err(|error| (None, error))?;
        let mut translations = Vec::new();

        for (entity, transform) in &bodies.p0() {
            let absolute = origin
                .checked_local_transform_to_world(*transform)
                .map(WorldTransform::position)
                .map_err(|error| (Some(entity), error))?;
            let local = target_frame
                .checked_absolute_to_local(absolute)
                .map_err(|error| (Some(entity), error))?;
            translations.push((entity, local));
        }

        Ok::<_, (Option<Entity>, BoxddWorldOriginError)>((revision, translations))
    })();

    match staged {
        Ok((revision, translations)) => {
            let mut mutable_transforms = bodies.p1();
            for (entity, translation) in translations {
                let Ok(mut transform) = mutable_transforms.get_mut(entity) else {
                    unreachable!("a staged rigid body cannot leave the query during one system");
                };
                transform.translation.x = translation.x;
                transform.translation.y = translation.y;
            }
            origin.commit_rebase(target, revision);
            context.commit_world_origin(target, revision);
            binding.validate(actual_world, &origin, &context);
            *last_failure = None;
            rebased.write(WorldOriginRebased {
                previous,
                current: target,
                revision,
            });
        }
        Err((entity, error)) => {
            let failure = (target, entity, error);
            if last_failure.as_ref() == Some(&failure) {
                return;
            }
            *last_failure = Some(failure);
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::RebaseWorldOrigin,
                    entity,
                    error: error.into(),
                },
            );
        }
    }
}

pub(crate) fn world_origin_is_settled(origin: Option<Res<BoxddWorldOrigin>>) -> bool {
    origin.is_some_and(|origin| origin.pending().is_none())
}

/// Creates native Box2D bodies for authored entities that have no authoritative body mapping.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_missing_bodies(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    origin: Res<BoxddWorldOrigin>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<MissingBodyItem<'_>, Without<BoxddBody>>,
    mut order: Local<Vec<Entity>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }
    let Some(foundation) = context.foundation() else {
        return;
    };

    refill_entity_order(&mut order, bodies.iter().map(|item| item.0));
    for entity in order.iter().copied() {
        let Ok((
            entity,
            rigid_body,
            parent,
            body_settings,
            transform,
            linear_velocity,
            angular_velocity,
        )) = bodies.get(entity)
        else {
            continue;
        };
        if context.authoritative_body(entity).is_some() {
            continue;
        }

        if let Some(parent) = parent {
            if rigid_body.is_added()
                || rigid_body.is_changed()
                || parent.is_added()
                || parent.is_changed()
            {
                report_error(
                    &settings,
                    &mut errors,
                    BoxddErrorMessage {
                        operation: BoxddOperation::ValidateBodyHierarchy,
                        entity: Some(entity),
                        error: BoxddPluginError::RigidBodyChildOf {
                            parent: parent.parent(),
                        },
                    },
                );
            }
            continue;
        }

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

        let world_transform = match transform {
            Some(transform) => origin.checked_local_transform_to_world(*transform),
            None => WorldTransform::new(origin.active(), boxdd::Rot::IDENTITY)
                .map_err(|_| BoxddWorldOriginError::InvalidOrigin),
        };
        let world_transform = match world_transform {
            Ok(transform) => transform,
            Err(error) => {
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
        };

        let mut def = BodyBuilder::from(foundation.body_def())
            .body_type((*rigid_body).into())
            .gravity_scale(body_settings.gravity_scale)
            .linear_damping(body_settings.linear_damping)
            .angular_damping(body_settings.angular_damping)
            .enable_sleep(body_settings.sleep_enabled)
            .bullet(body_settings.bullet)
            .motion_locks(body_settings.motion_locks)
            .position(world_transform.position())
            .angle(world_transform.rotation().angle());

        if let Some(linear_velocity) = linear_velocity {
            def = def.linear_velocity(to_boxdd_vec2(linear_velocity.0));
        }

        if let Some(angular_velocity) = angular_velocity {
            def = def.angular_velocity(angular_velocity.0);
        }

        let result = def.build().and_then(|def| {
            context.create_body(
                entity,
                def,
                BodyDescriptor {
                    rigid_body: *rigid_body,
                    settings: body_settings,
                },
            )
        });

        match result {
            Ok(body_id) => {
                commands.entity(entity).insert(BoxddBody::new(body_id));
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
pub(crate) fn apply_body_settings(
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<(Entity, &BoxddBody, &RigidBody, Option<&BodySettings>)>,
    mut order: Local<Vec<Entity>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    refill_entity_order(
        &mut order,
        bodies
            .iter()
            .filter_map(|(entity, _, rigid_body, body_settings)| {
                let descriptor = BodyDescriptor {
                    rigid_body: *rigid_body,
                    settings: body_settings.copied().unwrap_or_default(),
                };
                (context.body_descriptor(entity) != Some(descriptor)).then_some(entity)
            }),
    );
    for entity in order.iter().copied() {
        let Ok((entity, body, rigid_body, body_settings)) = bodies.get(entity) else {
            continue;
        };
        let body_settings = body_settings.copied().unwrap_or_default();
        let descriptor = BodyDescriptor {
            rigid_body: *rigid_body,
            settings: body_settings,
        };
        if context.body_descriptor(entity) == Some(descriptor) {
            continue;
        }
        let result = context.apply_body_settings(entity, body, *rigid_body, body_settings);
        apply_control_result(
            &settings,
            &mut errors,
            entity,
            BoxddOperation::ApplyBodySettings,
            result,
        );
    }
}

/// Creates native Box2D shapes for colliders that have no authoritative shape mapping.
///
/// Colliders may live on the body entity itself or on a child entity of a body.
pub(crate) fn create_missing_shapes(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    colliders: Query<MissingShapeItem<'_>, Without<BoxddShape>>,
    mut order: Local<Vec<Entity>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    refill_entity_order(&mut order, colliders.iter().map(|item| item.0));
    for entity in order.iter().copied() {
        let Ok((entity, parent, collider, material, transform)) = colliders.get(entity) else {
            continue;
        };
        if context.authoritative_shape(entity).is_some() {
            continue;
        }

        let Some(body_entity) = resolve_collider_body(&context, entity, parent) else {
            continue;
        };
        let local_transform = if body_entity == entity {
            ShapeLocalTransform::IDENTITY
        } else {
            ShapeLocalTransform::from_transform(transform)
        };
        let descriptor = ShapeDescriptor {
            collider: *collider,
            material: material.copied().unwrap_or_default(),
            local_transform,
        };
        let result = context.create_shape(entity, body_entity, descriptor);

        match result {
            Ok(shape_id) => {
                commands.entity(entity).insert(BoxddShape::new(shape_id));
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

/// Destroys native shapes whose collider entities or descriptors no longer exist.
pub(crate) fn cleanup_removed_colliders(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    colliders: Query<TrackedShapeItem<'_>>,
    mut tracked: Local<Vec<(Entity, boxdd::ShapeId)>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    tracked.clear();
    tracked.extend(
        context
            .tracked_shapes()
            .filter(|(entity, shape_id)| context.authoritative_shape(*entity) == Some(*shape_id))
            .filter(|(entity, _)| matches!(colliders.get(*entity), Err(_) | Ok((None, _, _, _)))),
    );
    tracked.sort_unstable_by_key(|(entity, _)| entity.to_bits());

    for (entity, old_id) in tracked.iter().copied() {
        let removed = matches!(colliders.get(entity), Err(_) | Ok((None, _, _, _)));
        if !removed {
            continue;
        }

        match context.destroy_shape(entity, old_id) {
            Ok(()) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.remove::<BoxddShape>();
                }
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::DestroyShape,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Recreates native shapes after bodies and their current transforms are ready.
pub(crate) fn replace_changed_shapes(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    colliders: Query<TrackedShapeItem<'_>>,
    mut tracked: Local<Vec<(Entity, boxdd::ShapeId)>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    tracked.clear();
    tracked.extend(
        context
            .tracked_shapes()
            .filter(|(entity, shape_id)| context.authoritative_shape(*entity) == Some(*shape_id))
            .filter(|(entity, _)| {
                let Ok((Some(collider), material, transform, parent)) = colliders.get(*entity)
                else {
                    return false;
                };
                let Some(body_entity) = resolve_collider_body(&context, *entity, parent) else {
                    return true;
                };
                let descriptor = ShapeDescriptor {
                    collider: *collider,
                    material: material.copied().unwrap_or_default(),
                    local_transform: if body_entity == *entity {
                        ShapeLocalTransform::IDENTITY
                    } else {
                        ShapeLocalTransform::from_transform(transform)
                    },
                };
                context.shape_body_entity(*entity) != Some(body_entity)
                    || context.shape_descriptor(*entity) != Some(descriptor)
            }),
    );
    tracked.sort_unstable_by_key(|(entity, _)| entity.to_bits());

    for (entity, old_id) in tracked.iter().copied() {
        let Ok((Some(collider), material, transform, parent)) = colliders.get(entity) else {
            continue;
        };
        let Some(body_entity) = resolve_collider_body(&context, entity, parent) else {
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::ReplaceShape,
                    entity: Some(entity),
                    error: BoxddError::InvalidBodyId.into(),
                },
            );
            continue;
        };
        let descriptor = ShapeDescriptor {
            collider: *collider,
            material: material.copied().unwrap_or_default(),
            local_transform: if body_entity == entity {
                ShapeLocalTransform::IDENTITY
            } else {
                ShapeLocalTransform::from_transform(transform)
            },
        };
        if context.shape_body_entity(entity) == Some(body_entity)
            && context.shape_descriptor(entity) == Some(descriptor)
        {
            continue;
        }

        match context.replace_shape(entity, old_id, body_entity, descriptor) {
            Ok(new_id) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.insert(BoxddShape::new(new_id));
                }
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::ReplaceShape,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Destroys native joints whose authored descriptors no longer exist.
pub(crate) fn cleanup_removed_joints(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    joints: Query<TrackedJointItem<'_>>,
    mut tracked: Local<Vec<(Entity, boxdd::JointId)>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    tracked.clear();
    tracked.extend(
        context
            .tracked_joints()
            .filter(|(entity, joint_id)| context.authoritative_joint(*entity) == Some(*joint_id))
            .filter(|(entity, _)| joints.get(*entity).ok().flatten().is_none()),
    );
    tracked.sort_unstable_by_key(|(entity, _)| entity.to_bits());

    for (entity, old_id) in tracked.iter().copied() {
        if joints.get(entity).ok().flatten().is_some() {
            continue;
        }

        match context.destroy_joint(entity, old_id) {
            Ok(()) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.remove::<BoxddJoint>();
                }
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::DestroyJoint,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Recreates native joints after endpoint bodies and their current transforms are ready.
pub(crate) fn replace_changed_joints(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    joints: Query<TrackedJointItem<'_>>,
    mut tracked: Local<Vec<(Entity, boxdd::JointId)>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    tracked.clear();
    tracked.extend(
        context
            .tracked_joints()
            .filter(|(entity, joint_id)| context.authoritative_joint(*entity) == Some(*joint_id))
            .filter(|(entity, _)| {
                joints
                    .get(*entity)
                    .ok()
                    .flatten()
                    .is_some_and(|descriptor| {
                        context.joint_descriptor(*entity) != Some(*descriptor)
                    })
            }),
    );
    tracked.sort_unstable_by_key(|(entity, _)| entity.to_bits());

    for (entity, old_id) in tracked.iter().copied() {
        let Some(descriptor) = joints.get(entity).ok().flatten().copied() else {
            continue;
        };
        if context.joint_descriptor(entity) == Some(descriptor) {
            continue;
        }

        match context.replace_joint(entity, old_id, descriptor) {
            Ok(new_id) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.insert(BoxddJoint::new(new_id));
                }
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::ReplaceJoint,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

/// Destroys native bodies when their Bevy body entities are removed or no longer have [`RigidBody`].
///
/// Shapes owned by the removed body are detached from their Bevy entities too.
pub(crate) fn cleanup_removed_bodies(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<(Option<&RigidBody>, Option<&ChildOf>)>,
    mut stale: Local<Vec<(Entity, boxdd::BodyId)>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    stale.clear();
    stale.extend(context.tracked_bodies().filter_map(|(entity, body_id)| {
        if context.authoritative_body(entity) != Some(body_id) {
            return None;
        }

        let should_remove = bodies
            .get(entity)
            .map(|(body, parent)| body.is_none() || parent.is_some())
            .unwrap_or(true);
        should_remove.then_some((entity, body_id))
    }));
    stale.sort_unstable_by_key(|(entity, _)| entity.to_bits());
    if stale.is_empty() {
        return;
    }

    let dependents = match context.body_dependents_for_batch(&stale) {
        Ok(dependents) => dependents,
        Err(error) => {
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::DestroyBody,
                    entity: stale.first().map(|(entity, _)| *entity),
                    error: error.into(),
                },
            );
            return;
        }
    };

    for ((entity, body_id), dependents) in stale.iter().copied().zip(dependents) {
        let result = context.destroy_body(entity, body_id, dependents);

        match result {
            Ok(dependents) => {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.remove::<BoxddBody>();
                }
                for (shape_entity, _) in dependents.shapes {
                    if let Ok(mut entity_commands) = commands.get_entity(shape_entity) {
                        entity_commands.remove::<BoxddShape>();
                    }
                }
                for (joint_entity, _) in dependents.joints {
                    if let Ok(mut entity_commands) = commands.get_entity(joint_entity) {
                        entity_commands.remove::<BoxddJoint>();
                    }
                }
            }
            Err(error) => {
                report_error(
                    &settings,
                    &mut errors,
                    BoxddErrorMessage {
                        operation: BoxddOperation::DestroyBody,
                        entity: Some(entity),
                        error: error.into(),
                    },
                );

                // The batch assigns each shared dependent to exactly one stale body. Once a
                // destruction fails, later native cascades could retire a dependent still owned
                // by the failed entry in the identity graph. Recompute the remaining batch on the
                // next fixed update instead of consuming a stale dependency snapshot.
                break;
            }
        }
    }
}

/// Creates native Box2D joints for descriptors that have no authoritative joint mapping.
pub(crate) fn create_missing_joints(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    joints: Query<MissingJointItem<'_>, Without<BoxddJoint>>,
    mut order: Local<Vec<Entity>>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    refill_entity_order(&mut order, joints.iter().map(|item| item.0));
    for entity in order.iter().copied() {
        let Ok((entity, descriptor)) = joints.get(entity) else {
            continue;
        };
        if context.authoritative_joint(entity).is_some() {
            continue;
        }

        let result = context.create_joint(entity, *descriptor);

        match result {
            Ok(joint_id) => {
                commands.entity(entity).insert(BoxddJoint::new(joint_id));
            }
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::CreateJoint,
                    entity: Some(entity),
                    error: error.into(),
                },
            ),
        }
    }
}

fn resolve_collider_body(
    context: &BoxddPhysicsContext,
    collider_entity: Entity,
    parent: Option<&ChildOf>,
) -> Option<Entity> {
    if context.authoritative_body(collider_entity).is_some() {
        return Some(collider_entity);
    }

    let parent = parent?.parent();
    context.authoritative_body(parent).map(|_| parent)
}

/// Applies velocity and one-shot impulse components to native bodies.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_body_controls(
    mut commands: Commands,
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    controls: Query<BodyControlItem<'_>>,
    membership_changes: Query<Entity, BodyControlMembershipChanged>,
    mut order: Local<EntityOrderCache>,
) {
    refresh_entity_order(
        &mut order,
        controls.iter().map(|item| item.0),
        membership_changes.iter(),
        |entity| {
            controls.get(entity).is_ok_and(
                |(_, _, linear_velocity, angular_velocity, linear_impulse, angular_impulse)| {
                    linear_velocity.is_some()
                        || angular_velocity.is_some()
                        || linear_impulse.is_some()
                        || angular_impulse.is_some()
                },
            )
        },
    );

    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    for entity in order.entities.iter().copied() {
        let Ok((entity, body, linear_velocity, angular_velocity, linear_impulse, angular_impulse)) =
            controls.get(entity)
        else {
            continue;
        };
        if let Some(linear_velocity) = linear_velocity {
            let result = context.set_body_linear_velocity(entity, body, linear_velocity.0);
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
        }

        if let Some(angular_velocity) = angular_velocity {
            let result = context.set_body_angular_velocity(entity, body, angular_velocity.0);
            apply_control_result(
                &settings,
                &mut errors,
                entity,
                BoxddOperation::ApplyBodyControl,
                result,
            );
        }

        if let Some(linear_impulse) = linear_impulse {
            let result = context.apply_body_linear_impulse(
                entity,
                body,
                linear_impulse.impulse,
                linear_impulse.wake,
            );
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
            let result = context.apply_body_angular_impulse(
                entity,
                body,
                angular_impulse.impulse,
                angular_impulse.wake,
            );
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_bevy_transforms_to_boxdd(
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    origin: Res<BoxddWorldOrigin>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    bodies: Query<BodyTransformItem<'_>>,
    membership_changes: Query<Entity, BodyTransformMembershipChanged>,
    mut order: Local<EntityOrderCache>,
) {
    refresh_entity_order(
        &mut order,
        bodies.iter().map(|item| item.0),
        membership_changes.iter(),
        |entity| bodies.get(entity).is_ok(),
    );

    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    for entity in order.entities.iter().copied() {
        let Ok((entity, body, transform, sync_mode, rigid_body)) = bodies.get(entity) else {
            continue;
        };
        if effective_sync_mode(sync_mode, rigid_body) != TransformSyncMode::BevyToPhysics {
            continue;
        }

        let world_transform = match origin.checked_local_transform_to_world(*transform) {
            Ok(transform) => transform,
            Err(error) => {
                report_error(
                    &settings,
                    &mut errors,
                    BoxddErrorMessage {
                        operation: BoxddOperation::SyncTransform,
                        entity: Some(entity),
                        error: error.into(),
                    },
                );
                continue;
            }
        };
        let result = context.set_body_transform(
            entity,
            body,
            world_transform.position(),
            world_transform.rotation().angle(),
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

/// Advances Box2D and publishes the borrowed event views before the completed step retires.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_and_publish_physics_messages(
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    step_settings: Res<BoxddStepSettings>,
    interests: Res<BoxddEventInterests>,
    time: Res<Time<Fixed>>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut body_moves: MessageWriter<BoxddBodyMoveMessage>,
    mut contact_begin: MessageWriter<BoxddContactBeginMessage>,
    mut contact_end: MessageWriter<BoxddContactEndMessage>,
    mut contact_hit: MessageWriter<BoxddContactHitMessage>,
    mut joint_events: MessageWriter<BoxddJointEventMessage>,
    mut sensor_begin: MessageWriter<BoxddSensorBeginMessage>,
    mut sensor_end: MessageWriter<BoxddSensorEndMessage>,
) {
    if !binding.allows(&context) || !context.is_enabled() {
        return;
    }

    let time_step = if time.delta_secs() > 0.0 {
        time.delta_secs()
    } else {
        step_settings.fallback_timestep_seconds
    };

    let result = context.step_with_events(
        time_step,
        step_settings.sub_step_count,
        |completed, entities| {
            materialize_then_publish(
                || {
                    // Materialize every requested family before publishing any message so a
                    // malformed native family cannot produce a partial Bevy message batch.
                    let body = interests
                        .body_moves
                        .then(|| completed.body_events())
                        .transpose()?;
                    let contact = interests
                        .contacts
                        .then(|| completed.contact_events())
                        .transpose()?;
                    let joint = interests
                        .joints
                        .then(|| completed.joint_events())
                        .transpose()?;
                    let sensor = interests
                        .sensors
                        .then(|| completed.sensor_events())
                        .transpose()?;
                    Ok((body, contact, joint, sensor))
                },
                |(body, contact, joint, sensor)| {
                    if let Some(body) = body {
                        for event in &body {
                            body_moves.write(BoxddBodyMoveMessage {
                                body_id: event.body_id,
                                entity: entities.body(event.body_id),
                                transform: event.transform,
                                fell_asleep: event.fell_asleep,
                            });
                        }
                    }

                    if let Some(contact) = contact {
                        for event in contact.begin() {
                            contact_begin.write(BoxddContactBeginMessage {
                                shape_a: event.shape_a,
                                shape_b: event.shape_b,
                                entity_a: entities.shape(event.shape_a),
                                entity_b: entities.shape(event.shape_b),
                                contact_id: event.contact_id,
                            });
                        }
                        for event in contact.end() {
                            contact_end.write(BoxddContactEndMessage {
                                shape_a: event.shape_a,
                                shape_b: event.shape_b,
                                entity_a: entities.shape(event.shape_a),
                                entity_b: entities.shape(event.shape_b),
                                contact_id: event.contact_id,
                            });
                        }
                        for event in contact.hit() {
                            contact_hit.write(BoxddContactHitMessage {
                                shape_a: event.shape_a,
                                shape_b: event.shape_b,
                                entity_a: entities.shape(event.shape_a),
                                entity_b: entities.shape(event.shape_b),
                                contact_id: event.contact_id,
                                point: event.point,
                                normal: event.normal,
                                approach_speed: event.approach_speed,
                            });
                        }
                    }

                    if let Some(joint) = joint {
                        for event in &joint {
                            joint_events.write(BoxddJointEventMessage {
                                joint_id: event.joint_id,
                                entity: entities.joint(event.joint_id),
                            });
                        }
                    }

                    if let Some(sensor) = sensor {
                        for event in sensor.begin() {
                            sensor_begin.write(BoxddSensorBeginMessage {
                                sensor_shape: event.sensor_shape,
                                visitor_shape: event.visitor_shape,
                                sensor_entity: entities.shape(event.sensor_shape),
                                visitor_entity: entities.shape(event.visitor_shape),
                            });
                        }
                        for event in sensor.end() {
                            sensor_end.write(BoxddSensorEndMessage {
                                sensor_shape: event.sensor_shape,
                                visitor_shape: event.visitor_shape,
                                sensor_entity: entities.shape(event.sensor_shape),
                                visitor_entity: entities.shape(event.visitor_shape),
                            });
                        }
                    }
                },
            )
        },
    );

    let Some(StepEventErrors { step, read_events }) = result else {
        return;
    };
    for (operation, error) in [
        (BoxddOperation::StepWorld, step),
        (BoxddOperation::ReadEvents, read_events),
    ] {
        if let Some(error) = error {
            report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation,
                    entity: None,
                    error: error.into(),
                },
            );
        }
    }
}

/// Writes Box2D transforms into Bevy for bodies using [`TransformSyncMode::PhysicsToBevy`].
pub(crate) fn sync_boxdd_transforms_to_bevy(
    mut context: NonSendMut<BoxddPhysicsContext>,
    binding: ContextBindingGuard<'_>,
    settings: Res<BoxddErrorPolicy>,
    origin: Res<BoxddWorldOrigin>,
    mut errors: MessageWriter<BoxddErrorMessage>,
    mut bodies: Query<BodyTransformMutItem<'_>>,
) {
    if !binding.allows(&context) || context.last_step_failed || !context.is_enabled() {
        return;
    }

    for (entity, body, mut transform, sync_mode, rigid_body) in &mut bodies {
        if effective_sync_mode(sync_mode, rigid_body) != TransformSyncMode::PhysicsToBevy {
            continue;
        }

        let result = context
            .body_projection(entity, body)
            .ok_or(BoxddPluginError::Api(BoxddError::InvalidBodyId))
            .and_then(|body_id| context.body_transform(body_id));

        match result {
            Ok(boxdd_transform) => {
                if let Err(error) =
                    origin.checked_apply_world_transform(&mut transform, boxdd_transform)
                {
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
            Err(error) => report_error(
                &settings,
                &mut errors,
                BoxddErrorMessage {
                    operation: BoxddOperation::SyncTransform,
                    entity: Some(entity),
                    error,
                },
            ),
        }
    }
}

fn apply_control_result(
    settings: &BoxddErrorPolicy,
    errors: &mut MessageWriter<'_, BoxddErrorMessage>,
    entity: Entity,
    operation: BoxddOperation,
    result: BoxddResult<()>,
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

fn refill_entity_order(order: &mut Vec<Entity>, entities: impl Iterator<Item = Entity>) {
    order.clear();
    order.extend(entities);
    order.sort_unstable_by_key(|entity| entity.to_bits());
}

fn refresh_entity_order(
    order: &mut EntityOrderCache,
    initial_entities: impl Iterator<Item = Entity>,
    changed_entities: impl Iterator<Item = Entity>,
    mut is_active: impl FnMut(Entity) -> bool,
) {
    if !order.initialized {
        order
            .entities
            .extend(initial_entities.filter(|entity| is_active(*entity)));
        order
            .entities
            .sort_unstable_by_key(|entity| entity.to_bits());
        order.entities.dedup();
        order.initialized = true;
    } else {
        order.entities.retain(|entity| is_active(*entity));
    }

    for entity in changed_entities {
        if !is_active(entity) {
            continue;
        }
        let key = entity.to_bits();
        if let Err(index) = order
            .entities
            .binary_search_by_key(&key, |candidate| candidate.to_bits())
        {
            order.entities.insert(index, entity);
        }
    }
}

fn for_each_authoritative_if_incomplete<I>(
    valid_projection_count: usize,
    authoritative: I,
    visit: impl FnMut(I::Item),
) where
    I: ExactSizeIterator,
{
    if valid_projection_count != authoritative.len() {
        authoritative.for_each(visit);
    }
}

fn materialize_then_publish<T, E>(
    materialize: impl FnOnce() -> Result<T, E>,
    publish: impl FnOnce(T),
) -> Result<(), E> {
    let batch = materialize()?;
    publish(batch);
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{
        EntityOrderCache, ProjectionReconcileState, context_world_binding_is_valid,
        for_each_authoritative_if_incomplete, materialize_then_publish, refresh_entity_order,
    };
    use crate::{
        BoxddBody, BoxddPhysicsContext, BoxddPhysicsPlugin, BoxddPhysicsSettings, BoxddShape,
        BoxddWorldOrigin, Collider, JointDescriptor, RigidBody,
    };
    use bevy_app::{App, FixedUpdate};
    use bevy_ecs::{
        system::{IntoSystem, System},
        world::World,
    };
    use bevy_transform::components::Transform;
    use std::{cell::Cell, sync::Arc};

    #[test]
    fn context_binding_condition_remains_send_after_initialization() {
        let mut world = World::new();
        let mut system = IntoSystem::into_system(context_world_binding_is_valid);

        system.initialize(&mut world);

        assert!(system.is_send());
    }

    #[test]
    fn event_family_failure_prevents_partial_batch_publication() {
        for failed_family in 0..4 {
            let getter_calls = Cell::new(0);
            let publish_calls = Cell::new(0);

            let result = materialize_then_publish(
                || {
                    let mut families = [false; 4];
                    for (family, materialized) in families.iter_mut().enumerate() {
                        getter_calls.set(getter_calls.get() + 1);
                        if family == failed_family {
                            return Err(family);
                        }
                        *materialized = true;
                    }
                    Ok(families)
                },
                |_| publish_calls.set(publish_calls.get() + 1),
            );

            assert_eq!(result, Err(failed_family));
            assert_eq!(getter_calls.get(), failed_family + 1);
            assert_eq!(publish_calls.get(), 0);
        }
    }

    #[test]
    fn steady_projection_reconciliation_skips_the_authoritative_scan() {
        let visited = Cell::new(0);
        for_each_authoritative_if_incomplete(4_096, 0..4_096, |_| visited.set(visited.get() + 1));
        assert_eq!(visited.get(), 0);

        for_each_authoritative_if_incomplete(4_095, 0..4_096, |_| visited.set(visited.get() + 1));
        assert_eq!(visited.get(), 4_096);
    }

    #[test]
    fn projection_reconciliation_rescans_only_after_invalidations_or_context_replacement() {
        let first_context = Arc::new(());
        let replacement_context = Arc::new(());
        let mut state = ProjectionReconcileState::default();

        assert!(state.begin(&first_context));
        assert!(!state.begin(&first_context));

        state.invalidate();
        assert!(state.begin(&first_context));
        assert!(!state.begin(&first_context));

        assert!(state.begin(&replacement_context));
        assert!(!state.begin(&replacement_context));
    }

    #[test]
    fn cached_entity_order_scans_the_full_membership_only_once() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let removed = world.spawn_empty().id();
        let third = world.spawn_empty().id();
        let added = world.spawn_empty().id();
        let full_scan_visits = Cell::new(0);
        let mut order = EntityOrderCache::default();

        refresh_entity_order(
            &mut order,
            [third, first, removed].into_iter().inspect(|_| {
                full_scan_visits.set(full_scan_visits.get() + 1);
            }),
            std::iter::empty(),
            |_| true,
        );
        assert_eq!(full_scan_visits.get(), 3);

        refresh_entity_order(
            &mut order,
            [first, removed, third].into_iter().inspect(|_| {
                full_scan_visits.set(full_scan_visits.get() + 1);
            }),
            std::iter::once(added),
            |entity| entity != removed,
        );
        assert_eq!(full_scan_visits.get(), 3);

        let mut expected = vec![first, third, added];
        expected.sort_unstable_by_key(|entity| entity.to_bits());
        assert_eq!(order.entities, expected);
    }

    #[test]
    fn projection_reconciliation_continues_while_an_origin_rebase_is_pending() {
        let foundation = boxdd::Foundation::initialize_default().unwrap();
        let mut app = App::new();
        app.add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings::default(),
        ));
        let authoritative_entity = app
            .world_mut()
            .spawn((RigidBody::Static, Transform::default()))
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        let body_id = app
            .world()
            .entity(authoritative_entity)
            .get::<BoxddBody>()
            .unwrap()
            .id();

        app.world_mut()
            .spawn((RigidBody::Static, Transform::from_xyz(f32::MAX, 0.0, 0.0)));
        app.world_mut()
            .resource_mut::<BoxddWorldOrigin>()
            .request_rebase(boxdd::Position::from([-f32::MAX, 0.0]))
            .unwrap();
        app.world_mut()
            .entity_mut(authoritative_entity)
            .remove::<BoxddBody>();

        app.world_mut().run_schedule(FixedUpdate);

        assert!(
            app.world()
                .resource::<BoxddWorldOrigin>()
                .pending()
                .is_some()
        );
        assert_eq!(
            app.world()
                .entity(authoritative_entity)
                .get::<BoxddBody>()
                .unwrap()
                .id(),
            body_id
        );
    }

    #[test]
    fn failed_body_cleanup_does_not_consume_the_remaining_shared_joint_batch() {
        let foundation = boxdd::Foundation::initialize_default().unwrap();
        let mut app = App::new();
        app.add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings::default(),
        ));
        let body_a = app
            .world_mut()
            .spawn((RigidBody::Static, Transform::default()))
            .id();
        let body_b = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::from_translation(bevy_math::Vec3::X),
            ))
            .id();
        app.world_mut().spawn(JointDescriptor::distance(
            body_a,
            body_b,
            boxdd::Position::ZERO,
            boxdd::Position::from([1.0, 0.0]),
        ));
        app.world_mut().run_schedule(FixedUpdate);

        let body_a_id = app.world().entity(body_a).get::<BoxddBody>().unwrap().id();
        let body_b_id = app.world().entity(body_b).get::<BoxddBody>().unwrap().id();
        let (failed_entity, failed_id, remaining_entity, remaining_id) =
            if body_a.to_bits() < body_b.to_bits() {
                (body_a, body_a_id, body_b, body_b_id)
            } else {
                (body_b, body_b_id, body_a, body_a_id)
            };
        app.world_mut()
            .non_send_mut::<BoxddPhysicsContext>()
            .plugin_world_mut()
            .unwrap()
            .body(failed_id)
            .unwrap()
            .destroy()
            .unwrap();

        app.world_mut().entity_mut(body_a).remove::<RigidBody>();
        app.world_mut().entity_mut(body_b).remove::<RigidBody>();
        app.world_mut().run_schedule(FixedUpdate);

        assert!(app.world().entity(failed_entity).contains::<BoxddBody>());
        assert!(app.world().entity(remaining_entity).contains::<BoxddBody>());
        let context = app.world().non_send::<BoxddPhysicsContext>();
        assert_eq!(context.body_entity(remaining_id), Some(remaining_entity));
        assert_eq!(context.world().unwrap().counters().unwrap().body_count, 1);
    }

    #[test]
    fn duplicate_shape_projection_cannot_destroy_the_authoritative_shape() {
        let foundation = boxdd::Foundation::initialize_default().unwrap();
        let mut app = App::new();
        app.add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings::default(),
        ));
        let authoritative_entity = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::circle(0.5),
                Transform::default(),
            ))
            .id();
        app.world_mut().run_schedule(FixedUpdate);

        let shape_id = app
            .world()
            .entity(authoritative_entity)
            .get::<BoxddShape>()
            .unwrap()
            .id();
        let body_id = app
            .world()
            .entity(authoritative_entity)
            .get::<BoxddBody>()
            .unwrap()
            .id();
        let duplicate_entity = app.world_mut().spawn(BoxddShape::new(shape_id)).id();

        app.world_mut().run_schedule(FixedUpdate);

        assert!(
            !app.world()
                .entity(duplicate_entity)
                .contains::<BoxddShape>()
        );
        let authoritative_projection = app
            .world()
            .entity(authoritative_entity)
            .get::<BoxddShape>()
            .unwrap();
        assert_eq!(authoritative_projection.id(), shape_id);
        let context = app.world().non_send::<BoxddPhysicsContext>();
        assert_eq!(context.shape_entity(shape_id), Some(authoritative_entity));
        assert_eq!(
            context.shape_owner_entity(shape_id),
            Some(authoritative_entity)
        );
        assert_eq!(context.body_entity(body_id), Some(authoritative_entity));
        let counters = context.world().unwrap().counters().unwrap();
        assert_eq!(counters.body_count, 1);
        assert_eq!(counters.shape_count, 1);
    }
}
