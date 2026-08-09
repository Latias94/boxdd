//! Bevy messages emitted by the physics plugin.

use crate::origin::BoxddWorldOriginError;
use bevy_ecs::{
    prelude::{Entity, Message},
    world::WorldId,
};
use boxdd::{
    BodyId, ContactId, Error as BoxddError, JointId, Position as BoxddPosition, ShapeId,
    Vec2 as BoxddVec2, WorldTransform as BoxddWorldTransform,
};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Reason a [`crate::BoxddPhysicsContext`] no longer contains a native world.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BoxddContextDisabledReason {
    /// The application explicitly constructed a disabled context.
    Explicit,
    /// Native world creation failed while the plugin was being installed.
    StartupWorldCreationFailed,
    /// Snapshot commit entered its terminal failure path.
    SnapshotRestoreFailed,
    /// A native replacement could not roll its provisional object back safely.
    LifecycleTransactionFailed,
}

/// Kind of plugin-owned object referenced by a snapshot restore error.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BoxddSnapshotObjectKind {
    /// A native body and its [`crate::BoxddBody`] projection.
    Body,
    /// A native shape and its [`crate::BoxddShape`] projection.
    Shape,
    /// A native joint and its [`crate::BoxddJoint`] projection.
    Joint,
}

/// Failure returned by plugin-level snapshot capture or restore.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoxddSnapshotError {
    /// The Bevy world does not contain a [`crate::BoxddPhysicsContext`] resource.
    #[error("the Bevy world has no BoxddPhysicsContext")]
    ContextUnavailable,
    /// The context or snapshot belongs to another Bevy world.
    #[error("physics state belongs to Bevy world {expected:?}, not {actual:?}")]
    WrongEcsWorld {
        /// World that owns the context or snapshot.
        expected: WorldId,
        /// World passed to the operation.
        actual: WorldId,
    },
    /// The context has no live native Box2D world.
    #[error("the BoxddPhysicsContext is disabled: {reason:?}")]
    ContextDisabled {
        /// Terminal or explicit reason recorded by the context.
        reason: BoxddContextDisabledReason,
    },
    /// An entity owned by the captured identity graph has since been despawned.
    #[error("snapshot {kind:?} entity {entity:?} no longer exists")]
    EntityMissing {
        /// Missing entity.
        entity: Entity,
        /// Kind of native object projected by the entity.
        kind: BoxddSnapshotObjectKind,
    },
    /// Another restore request is already waiting for the fixed restore phase.
    #[error("a snapshot restore request is already queued")]
    RestoreAlreadyQueued,
    /// The context has exhausted its monotonic snapshot restore ticket space.
    #[error("the snapshot restore ticket space is exhausted")]
    RestoreTicketExhausted,
    /// A projection hook changed the context binding or world origin during native commit.
    #[error("the Bevy world binding changed during the snapshot restore transaction")]
    WorldBindingChanged,
    /// A host callback or ECS hook panicked while the restore transaction was running.
    #[error("the snapshot restore transaction panicked")]
    RestorePanicked,
    /// The underlying safe Box2D operation failed.
    #[error(transparent)]
    Api(#[from] BoxddError),
}

/// Opaque identifier for one queued snapshot restore request.
///
/// The owning context identity is part of the ticket, so tickets remain unambiguous across context
/// replacement. Observe the matching [`BoxddSnapshotRestoreMessage`] to learn whether the
/// fixed-step transaction succeeded.
#[derive(Clone)]
pub struct BoxddSnapshotRestoreTicket {
    context_identity: Arc<()>,
    sequence: u64,
}

impl BoxddSnapshotRestoreTicket {
    pub(crate) fn new(context_identity: Arc<()>, sequence: u64) -> Self {
        Self {
            context_identity,
            sequence,
        }
    }
}

impl std::fmt::Debug for BoxddSnapshotRestoreTicket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("BoxddSnapshotRestoreTicket")
            .field(&self.sequence)
            .finish()
    }
}

impl PartialEq for BoxddSnapshotRestoreTicket {
    fn eq(&self, other: &Self) -> bool {
        self.sequence == other.sequence
            && Arc::ptr_eq(&self.context_identity, &other.context_identity)
    }
}

impl Eq for BoxddSnapshotRestoreTicket {}

impl Hash for BoxddSnapshotRestoreTicket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.context_identity), state);
        self.sequence.hash(state);
    }
}

impl Ord for BoxddSnapshotRestoreTicket {
    fn cmp(&self, other: &Self) -> Ordering {
        Arc::as_ptr(&self.context_identity)
            .addr()
            .cmp(&Arc::as_ptr(&other.context_identity).addr())
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

impl PartialOrd for BoxddSnapshotRestoreTicket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Outcome emitted after a queued snapshot restore reaches the fixed physics pipeline.
///
/// Restore requests run in [`crate::BoxddPhysicsSet::Restore`], after origin rebasing and before
/// identity reconciliation, cleanup, and stepping. A request queued after that set has run waits
/// for the next `FixedUpdate`. A failed pending origin rebase keeps the request queued until the
/// origin is settled. While the owning context remains installed and this fixed-update set keeps
/// running, every accepted ticket that reaches the restore phase emits exactly one outcome.
/// This guarantee assumes application code leaves the plugin-managed `Messages` resource installed
/// outside the restore transaction.
/// [`crate::BoxddPhysicsContext::cancel_snapshot_restore`] and removing or replacing the context
/// cancel a context-local pending request without emitting a message.
/// [`BoxddSnapshotError::RestorePanicked`] is emitted before the original host panic resumes.
#[derive(Message, Clone, Debug, Eq, PartialEq)]
pub struct BoxddSnapshotRestoreMessage {
    /// Ticket returned by [`crate::BoxddPhysicsContext::queue_snapshot_restore`].
    pub ticket: BoxddSnapshotRestoreTicket,
    /// Result of the one native/ECS restore transaction.
    pub result: Result<(), BoxddSnapshotError>,
}

/// Recoverable plugin error type routed through [`BoxddErrorMessage`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoxddPluginError {
    /// Error reported by the safe `boxdd` API.
    #[error(transparent)]
    Api(#[from] BoxddError),
    /// Coordinate conversion or world-origin rebase failed.
    #[error(transparent)]
    WorldOrigin(#[from] BoxddWorldOriginError),
    /// Snapshot capture or restore failed at the plugin boundary.
    #[error(transparent)]
    Snapshot(#[from] BoxddSnapshotError),
    /// The plugin's non-send physics context resource is missing.
    #[error("the Bevy world has no BoxddPhysicsContext")]
    ContextUnavailable,
    /// The physics context was moved into a different Bevy world.
    #[error("physics context belongs to Bevy world {expected:?}, not {actual:?}")]
    WrongEcsWorld {
        /// World that owns the context.
        expected: WorldId,
        /// World currently running the physics pipeline.
        actual: WorldId,
    },
    /// The public world-origin resource was removed after the physics context was created.
    #[error("the Bevy world has no BoxddWorldOrigin resource")]
    WorldOriginUnavailable,
    /// The public world-origin resource no longer matches the context's committed frame.
    #[error("the BoxddWorldOrigin resource was replaced outside the transactional rebase path")]
    WorldOriginStateMismatch,
    /// The context has no native world and cannot execute the requested operation.
    #[error("the physics context is disabled: {reason:?}")]
    ContextDisabled {
        /// Terminal or explicit reason recorded by the context.
        reason: BoxddContextDisabledReason,
    },
    /// A rigid-body entity used unsupported Bevy hierarchy semantics.
    #[error("rigid-body entities cannot have a ChildOf parent {parent:?}")]
    RigidBodyChildOf {
        /// Parent attached to the rejected rigid-body entity.
        parent: Entity,
    },
}

/// Plugin operation associated with a recoverable error message.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BoxddOperation {
    /// Creating the native Box2D world.
    CreateWorld,
    /// Validating that the physics context belongs to the current Bevy world.
    ValidateWorldBinding,
    /// Rejecting a rigid body that has a [`bevy_ecs::hierarchy::ChildOf`] parent.
    ValidateBodyHierarchy,
    /// Creating a native body from a [`crate::RigidBody`] entity.
    CreateBody,
    /// Creating a native shape from a [`crate::Collider`] entity.
    CreateShape,
    /// Creating a native joint from a [`crate::JointDescriptor`] entity.
    CreateJoint,
    /// Destroying a native body after ECS removal or descriptor invalidation.
    DestroyBody,
    /// Destroying a native shape after ECS removal or descriptor invalidation.
    DestroyShape,
    /// Replacing a native shape after its descriptor or owning body changes.
    ReplaceShape,
    /// Destroying a native joint after ECS removal or descriptor invalidation.
    DestroyJoint,
    /// Replacing a native joint after its descriptor changes.
    ReplaceJoint,
    /// Applying velocity or one-shot impulse components.
    ApplyBodyControl,
    /// Applying changed body settings.
    ApplyBodySettings,
    /// Configuring Bevy's fixed timestep resource.
    ConfigureFixedTimestep,
    /// Rebasing Bevy-local transforms to a new absolute world origin.
    RebaseWorldOrigin,
    /// Synchronizing transforms between Bevy and Box2D.
    SyncTransform,
    /// Stepping the native Box2D world.
    StepWorld,
    /// Reading body, contact, joint, or sensor events after a step.
    ReadEvents,
    /// Applying a queued snapshot restore in the fixed physics pipeline.
    RestoreSnapshot,
}

/// Notification emitted after an atomic world-origin rebase succeeds.
#[derive(Message, Copy, Clone, Debug, PartialEq)]
pub struct WorldOriginRebased {
    /// Absolute origin used before the rebase.
    pub previous: BoxddPosition,
    /// Newly active absolute origin.
    pub current: BoxddPosition,
    /// Monotonic revision assigned to this committed rebase.
    pub revision: u64,
}

/// Recoverable plugin error routed through Bevy messages.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddErrorMessage {
    /// Operation that produced the error.
    pub operation: BoxddOperation,
    /// Entity associated with the operation, when one exists.
    pub entity: Option<Entity>,
    /// The underlying plugin error.
    pub error: BoxddPluginError,
}

/// Body transform notification emitted after a successful physics step.
#[derive(Message, Clone, Debug)]
pub struct BoxddBodyMoveMessage {
    /// Native body id that moved.
    pub body_id: BodyId,
    /// Bevy entity mapped to the body id, if owned by this plugin.
    pub entity: Option<Entity>,
    /// Current Box2D world transform.
    pub transform: BoxddWorldTransform,
    /// Whether the body fell asleep during the step.
    pub fell_asleep: bool,
}

/// Contact begin notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddContactBeginMessage {
    /// First native shape in the contact pair.
    pub shape_a: ShapeId,
    /// Second native shape in the contact pair.
    pub shape_b: ShapeId,
    /// Bevy entity mapped to `shape_a`, if owned by this plugin.
    pub entity_a: Option<Entity>,
    /// Bevy entity mapped to `shape_b`, if owned by this plugin.
    pub entity_b: Option<Entity>,
    /// Native contact id for this completed-step epoch; it expires when the next step begins.
    pub contact_id: ContactId,
}

/// Contact end notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddContactEndMessage {
    /// First native shape in the contact pair.
    pub shape_a: ShapeId,
    /// Second native shape in the contact pair.
    pub shape_b: ShapeId,
    /// Bevy entity mapped to `shape_a`, if owned by this plugin.
    pub entity_a: Option<Entity>,
    /// Bevy entity mapped to `shape_b`, if owned by this plugin.
    pub entity_b: Option<Entity>,
    /// Native contact id for this completed-step epoch; ended contacts may already be non-live.
    pub contact_id: ContactId,
}

/// High-speed contact hit notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, PartialEq)]
pub struct BoxddContactHitMessage {
    /// First native shape in the contact pair.
    pub shape_a: ShapeId,
    /// Second native shape in the contact pair.
    pub shape_b: ShapeId,
    /// Bevy entity mapped to `shape_a`, if owned by this plugin.
    pub entity_a: Option<Entity>,
    /// Bevy entity mapped to `shape_b`, if owned by this plugin.
    pub entity_b: Option<Entity>,
    /// Native contact id for this completed-step epoch; it expires when the next step begins.
    pub contact_id: ContactId,
    /// Contact point reported by Box2D.
    pub point: BoxddPosition,
    /// Contact normal reported by Box2D.
    pub normal: BoxddVec2,
    /// Relative approach speed for the hit.
    pub approach_speed: f32,
}

/// Joint-threshold notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddJointEventMessage {
    /// Native joint that emitted the event.
    pub joint_id: JointId,
    /// Bevy entity mapped to `joint_id`, if owned by this plugin.
    pub entity: Option<Entity>,
}

/// Sensor overlap begin notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddSensorBeginMessage {
    /// Native sensor shape.
    pub sensor_shape: ShapeId,
    /// Native shape entering the sensor.
    pub visitor_shape: ShapeId,
    /// Bevy entity mapped to the sensor shape, if owned by this plugin.
    pub sensor_entity: Option<Entity>,
    /// Bevy entity mapped to the visitor shape, if owned by this plugin.
    pub visitor_entity: Option<Entity>,
}

/// Sensor overlap end notification emitted after a successful physics step.
#[derive(Message, Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoxddSensorEndMessage {
    /// Native sensor shape.
    pub sensor_shape: ShapeId,
    /// Native shape leaving the sensor.
    pub visitor_shape: ShapeId,
    /// Bevy entity mapped to the sensor shape, if owned by this plugin.
    pub sensor_entity: Option<Entity>,
    /// Bevy entity mapped to the visitor shape, if owned by this plugin.
    pub visitor_entity: Option<Entity>,
}

#[cfg(test)]
mod tests {
    use super::BoxddSnapshotRestoreTicket;
    use std::collections::{BTreeSet, HashSet};
    use std::sync::Arc;

    static_assertions::assert_impl_all!(BoxddSnapshotRestoreTicket: Clone, Send, Sync);
    static_assertions::assert_not_impl_any!(BoxddSnapshotRestoreTicket: Copy);

    #[test]
    fn snapshot_restore_ticket_traits_preserve_context_identity() {
        let context = Arc::new(());
        let first = BoxddSnapshotRestoreTicket::new(Arc::clone(&context), 0);
        let first_clone = first.clone();
        let next = BoxddSnapshotRestoreTicket::new(context, 1);
        let other_context = BoxddSnapshotRestoreTicket::new(Arc::new(()), 0);

        assert_eq!(first, first_clone);
        assert_ne!(first, next);
        assert_ne!(first, other_context);

        let hash_tickets = HashSet::from([first.clone(), first_clone, other_context.clone()]);
        assert_eq!(hash_tickets.len(), 2);

        let ordered_tickets = BTreeSet::from([first, next, other_context]);
        assert_eq!(ordered_tickets.len(), 3);
    }
}
