use super::{
    BodyDescriptor, BoxddEcsWorldBindingState, BoxddErrorPolicy, BoxddPhysicsContext,
    ShapeDescriptor, identity::IdentityGraph,
};
use crate::components::{JointDescriptor, JointKind};
use crate::messages::{
    BoxddErrorMessage, BoxddOperation, BoxddPluginError, BoxddSnapshotError,
    BoxddSnapshotObjectKind, BoxddSnapshotRestoreMessage, BoxddSnapshotRestoreTicket,
};
use crate::origin::BoxddWorldOrigin;
use bevy_ecs::{
    message::Messages,
    prelude::{Entity, World as EcsWorld},
    world::WorldId,
};
use boxdd::{
    BodyId, Error as BoxddError, JointId, JointType, Result as BoxddResult, ShapeId, Snapshot,
    SnapshotRestore,
};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

/// A plugin-owned snapshot of both native physics state and ECS identity metadata.
///
/// This capability has no public constructor and is deliberately not cloneable. It can only be
/// restored into the exact [`BoxddPhysicsContext`] whose native world captured it.
pub struct BoxddPhysicsSnapshot {
    owner_world: WorldId,
    native: Snapshot,
    bodies: HashMap<BodyId, SnapshotBody>,
    shapes: HashMap<ShapeId, SnapshotShape>,
    joints: HashMap<JointId, SnapshotJoint>,
}

impl std::fmt::Debug for BoxddPhysicsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxddPhysicsSnapshot")
            .field("owner_world", &self.owner_world)
            .field("bodies", &self.bodies.len())
            .field("shapes", &self.shapes.len())
            .field("joints", &self.joints.len())
            .finish_non_exhaustive()
    }
}

pub(super) struct PendingSnapshotRestore {
    ticket: BoxddSnapshotRestoreTicket,
    snapshot: BoxddPhysicsSnapshot,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RestoreContextSlotDisposition {
    Vacant,
    Replaced,
}

impl RestoreContextSlotDisposition {
    fn current(ecs_world: &EcsWorld) -> Self {
        if ecs_world.contains_non_send::<BoxddPhysicsContext>() {
            Self::Replaced
        } else {
            Self::Vacant
        }
    }

    fn finish(self, ecs_world: &mut EcsWorld, context: BoxddPhysicsContext) {
        match self {
            Self::Vacant => ecs_world.insert_non_send(context),
            Self::Replaced => {
                if let Some(mut binding) = ecs_world.get_resource_mut::<BoxddEcsWorldBindingState>()
                {
                    binding.invalidate();
                }
            }
        }
    }
}

impl BoxddSnapshotError {
    fn indicates_terminal_world(&self) -> bool {
        matches!(
            self,
            Self::Api(
                BoxddError::WorldPoisoned
                    | BoxddError::WorldDestroyed
                    | BoxddError::SnapshotCommitPanicked
            )
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct SnapshotShape {
    entity: Entity,
    body_entity: Entity,
    descriptor: ShapeDescriptor,
}

#[derive(Copy, Clone, Debug)]
struct SnapshotBody {
    entity: Entity,
    descriptor: BodyDescriptor,
}

#[derive(Copy, Clone, Debug)]
struct SnapshotJoint {
    entity: Entity,
    descriptor: JointDescriptor,
}

#[derive(Copy, Clone, Debug)]
struct ExistingProjections {
    entity: Entity,
    body: bool,
    shape: bool,
    joint: bool,
}

struct PreparedProjectionCommit {
    remove_bodies: Vec<Entity>,
    remove_shapes: Vec<Entity>,
    remove_joints: Vec<Entity>,
    bodies: Vec<(Entity, BodyId)>,
    shapes: Vec<(Entity, ShapeId)>,
    joints: Vec<(Entity, JointId)>,
}

struct RestoreHostBinding {
    owner_world: WorldId,
    committed_world_origin: boxdd::Position,
    world_origin_revision: u64,
    context_identity: Arc<()>,
}

impl RestoreHostBinding {
    fn capture(context: &BoxddPhysicsContext) -> Self {
        Self {
            owner_world: context.owner_world,
            committed_world_origin: context.committed_world_origin,
            world_origin_revision: context.world_origin_revision,
            context_identity: Arc::clone(&context.context_identity),
        }
    }

    fn remains_valid(&self, ecs_world: &EcsWorld) -> bool {
        if ecs_world.id() != self.owner_world
            || ecs_world.contains_non_send::<BoxddPhysicsContext>()
        {
            return false;
        }
        let Some(origin) = ecs_world.get_resource::<BoxddWorldOrigin>() else {
            return false;
        };
        let Some(binding) = ecs_world.get_resource::<BoxddEcsWorldBindingState>() else {
            return false;
        };

        // A pending rebase is future intent. Only the committed frame participates in the
        // transaction binding; the remaining fixed-update sets already wait for it to settle.
        origin.active() == self.committed_world_origin
            && origin.revision() == self.world_origin_revision
            && binding.valid
            && binding.owner_world == Some(self.owner_world)
            && binding.committed_world_origin == self.committed_world_origin
            && binding.world_origin_revision == self.world_origin_revision
            && binding
                .context_identity
                .as_ref()
                .is_some_and(|identity| Arc::ptr_eq(identity, &self.context_identity))
    }
}

impl BoxddPhysicsContext {
    /// Captures native state together with the plugin's authoritative ECS identity graph.
    pub fn snapshot(&mut self) -> Result<BoxddPhysicsSnapshot, BoxddSnapshotError> {
        if self.world.is_none() {
            return Err(BoxddSnapshotError::ContextDisabled {
                reason: self
                    .disabled_reason
                    .unwrap_or(crate::BoxddContextDisabledReason::Explicit),
            });
        }
        self.validate_identity_graph()?;

        let mut bodies = HashMap::new();
        bodies
            .try_reserve(self.graph.entity_to_body.len())
            .map_err(|_| BoxddError::SnapshotAllocationFailed)?;
        for (&entity, &id) in &self.graph.entity_to_body {
            let descriptor = *self
                .graph
                .body_descriptors
                .get(&entity)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            bodies.insert(id, SnapshotBody { entity, descriptor });
        }

        let mut shapes = HashMap::new();
        shapes
            .try_reserve(self.graph.entity_to_shape.len())
            .map_err(|_| BoxddError::SnapshotAllocationFailed)?;
        for (&entity, &id) in &self.graph.entity_to_shape {
            shapes.insert(
                id,
                SnapshotShape {
                    entity,
                    body_entity: self.graph.shape_to_body_entity[&entity],
                    descriptor: self.graph.shape_descriptors[&entity],
                },
            );
        }

        let mut joints = HashMap::new();
        joints
            .try_reserve(self.graph.entity_to_joint.len())
            .map_err(|_| BoxddError::SnapshotAllocationFailed)?;
        for (&entity, &id) in &self.graph.entity_to_joint {
            joints.insert(
                id,
                SnapshotJoint {
                    entity,
                    descriptor: self.graph.joint_descriptors[&entity],
                },
            );
        }

        let native = self
            .world
            .as_ref()
            .expect("validated live world above")
            .snapshot()?;
        Ok(BoxddPhysicsSnapshot {
            owner_world: self.owner_world,
            native,
            bodies,
            shapes,
            joints,
        })
    }

    /// Queues a plugin snapshot for atomic restoration in the next fixed restore phase.
    ///
    /// The request owns `snapshot` and is applied after world-origin rebasing but before identity
    /// reconciliation, native cleanup, and stepping. This ordering ensures authored ECS removals
    /// are observed by cleanup before the restored native objects can participate in a step.
    /// There is one pending slot per context; wait for the corresponding
    /// [`BoxddSnapshotRestoreMessage`] before queuing another request. Removing or replacing this
    /// context cancels its context-local pending request and its ticket produces no message.
    pub fn queue_snapshot_restore(
        &mut self,
        snapshot: BoxddPhysicsSnapshot,
    ) -> Result<BoxddSnapshotRestoreTicket, BoxddSnapshotError> {
        if !self.is_enabled() {
            return Err(BoxddSnapshotError::ContextDisabled {
                reason: self
                    .disabled_reason
                    .unwrap_or(crate::BoxddContextDisabledReason::Explicit),
            });
        }
        if snapshot.owner_world != self.owner_world {
            return Err(BoxddSnapshotError::WrongEcsWorld {
                expected: snapshot.owner_world,
                actual: self.owner_world,
            });
        }
        if self.pending_snapshot_restore.is_some() {
            return Err(BoxddSnapshotError::RestoreAlreadyQueued);
        }
        let ticket = BoxddSnapshotRestoreTicket::new(
            Arc::clone(&self.context_identity),
            self.next_snapshot_restore_ticket,
        );
        self.next_snapshot_restore_ticket = self
            .next_snapshot_restore_ticket
            .checked_add(1)
            .ok_or(BoxddSnapshotError::RestoreTicketExhausted)?;
        self.pending_snapshot_restore = Some(PendingSnapshotRestore {
            ticket: ticket.clone(),
            snapshot,
        });
        Ok(ticket)
    }

    /// Cancels the pending restore identified by `ticket` and returns its owned snapshot.
    ///
    /// A queued restore can remain pending while the world origin or context binding is unsettled.
    /// Cancellation lets the caller recover that snapshot without replacing the context. A ticket
    /// from another context, an already completed request, or a different pending request returns
    /// `None` and leaves the slot unchanged. Cancellation does not emit a
    /// [`BoxddSnapshotRestoreMessage`].
    pub fn cancel_snapshot_restore(
        &mut self,
        ticket: &BoxddSnapshotRestoreTicket,
    ) -> Option<BoxddPhysicsSnapshot> {
        if !self
            .pending_snapshot_restore
            .as_ref()
            .is_some_and(|pending| pending.ticket.eq(ticket))
        {
            return None;
        }
        self.pending_snapshot_restore
            .take()
            .map(|pending| pending.snapshot)
    }

    fn restore_snapshot_now(
        ecs_world: &mut EcsWorld,
        snapshot: &BoxddPhysicsSnapshot,
    ) -> Result<(), BoxddSnapshotError> {
        let mut context = ecs_world
            .remove_non_send::<Self>()
            .ok_or(BoxddSnapshotError::ContextUnavailable)?;
        let actual_world = ecs_world.id();
        if context.owner_world != actual_world {
            let error = BoxddSnapshotError::WrongEcsWorld {
                expected: context.owner_world,
                actual: actual_world,
            };
            RestoreContextSlotDisposition::current(ecs_world).finish(ecs_world, context);
            return Err(error);
        }
        if snapshot.owner_world != actual_world {
            let error = BoxddSnapshotError::WrongEcsWorld {
                expected: snapshot.owner_world,
                actual: actual_world,
            };
            RestoreContextSlotDisposition::current(ecs_world).finish(ecs_world, context);
            return Err(error);
        }
        if let Err(error) = validate_snapshot_entities(ecs_world, snapshot) {
            RestoreContextSlotDisposition::current(ecs_world).finish(ecs_world, context);
            return Err(error);
        }
        let existing = match collect_existing_projections(ecs_world) {
            Ok(existing) => existing,
            Err(error) => {
                RestoreContextSlotDisposition::current(ecs_world).finish(ecs_world, context);
                return Err(error.into());
            }
        };
        let mut native_commit_started = false;
        let mut terminal_cleanup_completed = false;
        let result = catch_unwind(AssertUnwindSafe(|| {
            let result = context.restore_snapshot_inner(
                ecs_world,
                snapshot,
                &existing,
                &mut native_commit_started,
            );
            let terminal_failure = result
                .as_ref()
                .is_err_and(|error| native_commit_started || error.indicates_terminal_world());
            if terminal_failure {
                terminal_cleanup_completed = catch_unwind(AssertUnwindSafe(|| {
                    context.disable_after_restore_failure(ecs_world, &existing, snapshot)
                }))
                .unwrap_or_else(|payload| {
                    suppress_panic_payload(payload);
                    false
                });
            }
            result
        }));
        let terminal_failure = match &result {
            Ok(Ok(())) => false,
            Ok(Err(error)) => native_commit_started || error.indicates_terminal_world(),
            Err(_) => native_commit_started,
        };
        if terminal_failure && !terminal_cleanup_completed {
            // `commit_with` terminalizes before resuming a host panic. Keep cleanup panic-safe so
            // the detached context is either reinserted disabled or relinquished to a replacement.
            let _cleanup_completed = catch_unwind(AssertUnwindSafe(|| {
                context.disable_after_restore_failure(ecs_world, &existing, snapshot)
            }))
            .unwrap_or_else(|payload| {
                suppress_panic_payload(payload);
                false
            });
        }
        RestoreContextSlotDisposition::current(ecs_world).finish(ecs_world, context);

        match result {
            Ok(result) => result,
            Err(payload) => resume_unwind(payload),
        }
    }

    fn validate_identity_graph(&mut self) -> BoxddResult<()> {
        let (world, graph) = (&mut self.world, &self.graph);
        let world = world.as_mut().ok_or(BoxddError::WorldDestroyed)?;
        if graph.entity_to_body.len() != graph.body_to_entity.len()
            || graph.entity_to_body.len() != graph.body_descriptors.len()
            || graph.entity_to_shape.len() != graph.shape_to_entity.len()
            || graph.entity_to_shape.len() != graph.shape_to_body_entity.len()
            || graph.entity_to_shape.len() != graph.shape_descriptors.len()
            || graph.entity_to_joint.len() != graph.joint_to_entity.len()
            || graph.entity_to_joint.len() != graph.joint_descriptors.len()
        {
            return Err(BoxddError::SnapshotManifestMismatch);
        }

        let counters = world.counters()?;
        if usize::try_from(counters.body_count).ok() != Some(graph.entity_to_body.len())
            || usize::try_from(counters.shape_count).ok() != Some(graph.entity_to_shape.len())
            || usize::try_from(counters.joint_count).ok() != Some(graph.entity_to_joint.len())
        {
            return Err(BoxddError::SnapshotManifestMismatch);
        }

        for (&entity, &body_id) in &graph.entity_to_body {
            if graph.body_to_entity.get(&body_id) != Some(&entity)
                || !graph.body_descriptors.contains_key(&entity)
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            world.body(body_id)?.transform()?;
        }

        for (&entity, &shape_id) in &graph.entity_to_shape {
            if graph.shape_to_entity.get(&shape_id) != Some(&entity)
                || !graph.shape_descriptors.contains_key(&entity)
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            let body_entity = *graph
                .shape_to_body_entity
                .get(&entity)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            let body_id = graph
                .authoritative_body(body_entity)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            if world.shape(shape_id)?.body_id()? != body_id {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }

        for (&entity, &joint_id) in &graph.entity_to_joint {
            if graph.joint_to_entity.get(&joint_id) != Some(&entity) {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            let descriptor = graph
                .joint_descriptors
                .get(&entity)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            let body_a = graph
                .authoritative_body(descriptor.entity_a)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            let body_b = graph
                .authoritative_body(descriptor.entity_b)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            let joint = world.joint(joint_id)?;
            let expected_type = match descriptor.kind {
                JointKind::Distance(_) => JointType::Distance,
                JointKind::Revolute(_) => JointType::Revolute,
            };
            if joint.body_a_id()? != body_a
                || joint.body_b_id()? != body_b
                || joint.joint_type()? != expected_type
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }

        Ok(())
    }

    fn restore_snapshot_inner(
        &mut self,
        ecs_world: &mut EcsWorld,
        snapshot: &BoxddPhysicsSnapshot,
        existing: &[ExistingProjections],
        native_commit_started: &mut bool,
    ) -> Result<(), BoxddSnapshotError> {
        let restore_binding = RestoreHostBinding::capture(self);
        let reason = self
            .disabled_reason
            .unwrap_or(crate::BoxddContextDisabledReason::Explicit);
        let world = self
            .world
            .as_mut()
            .ok_or(BoxddSnapshotError::ContextDisabled { reason })?;
        let prepared_restore = world.prepare_restore(&snapshot.native)?;
        let prepared_graph = IdentityGraph::prepare_restore(snapshot, prepared_restore.mappings())?;
        let prepared_projections =
            PreparedProjectionCommit::prepare(ecs_world, existing, &prepared_graph)?;
        let graph = &mut self.graph;
        let last_step_failed = &mut self.last_step_failed;
        let mut world_binding_changed = false;
        *native_commit_started = true;
        let result = prepared_restore.commit_with(|_| {
            *graph = prepared_graph;
            prepared_projections.commit(ecs_world)?;
            validate_projection_graph(ecs_world, graph)?;
            if !restore_binding.remains_valid(ecs_world) {
                world_binding_changed = true;
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            *last_step_failed = true;
            Ok(())
        });

        match result {
            Ok(_) => Ok(()),
            Err(BoxddError::SnapshotCommitPanicked) => Err(BoxddSnapshotError::RestorePanicked),
            Err(BoxddError::SnapshotManifestMismatch) if world_binding_changed => {
                Err(BoxddSnapshotError::WorldBindingChanged)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn disable_after_restore_failure(
        &mut self,
        ecs_world: &mut EcsWorld,
        existing: &[ExistingProjections],
        snapshot: &BoxddPhysicsSnapshot,
    ) -> bool {
        self.world = None;
        self.disabled_reason = Some(crate::BoxddContextDisabledReason::SnapshotRestoreFailed);
        self.graph = IdentityGraph::default();
        self.last_step_failed = true;
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return remove_projections_not_authorized_by_replacement(ecs_world);
        }
        remove_all_projections(ecs_world, existing);
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return remove_projections_not_authorized_by_replacement(ecs_world);
        }
        remove_snapshot_projections(ecs_world, snapshot);
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return remove_projections_not_authorized_by_replacement(ecs_world);
        }
        remove_remaining_projections(ecs_world)
    }
}

/// Executes at most one queued request before projection reconciliation and native cleanup.
///
/// This is an exclusive system because restore temporarily detaches the non-send context while it
/// commits native state and ECS projections as one transaction.
pub(crate) fn apply_pending_snapshot_restore(ecs_world: &mut EcsWorld) {
    let actual_world = ecs_world.id();
    let binding = ecs_world
        .get_resource::<BoxddEcsWorldBindingState>()
        .cloned()
        .unwrap_or_default();
    let origin = ecs_world.get_resource::<BoxddWorldOrigin>().copied();

    let pending = {
        let Some(mut context) = ecs_world.get_non_send_mut::<BoxddPhysicsContext>() else {
            return;
        };
        let context_belongs_to_world = context.owner_world == actual_world;
        if context_belongs_to_world
            && (!origin
                .as_ref()
                .is_some_and(|origin| origin.pending().is_none())
                || !binding.allows_origin(actual_world, origin.as_ref())
                || !binding.allows_context(actual_world, &context))
        {
            return;
        }
        context.pending_snapshot_restore.take()
    };
    let Some(pending) = pending else {
        return;
    };
    let outcome_messages = ecs_world
        .remove_resource::<Messages<BoxddSnapshotRestoreMessage>>()
        .unwrap_or_default();

    let restore = catch_unwind(AssertUnwindSafe(|| {
        BoxddPhysicsContext::restore_snapshot_now(ecs_world, &pending.snapshot)
    }));
    let result = match restore {
        Ok(result) => result,
        Err(payload) => {
            publish_snapshot_restore_outcome(
                ecs_world,
                outcome_messages,
                BoxddSnapshotRestoreMessage {
                    ticket: pending.ticket.clone(),
                    result: Err(BoxddSnapshotError::RestorePanicked),
                },
            );
            resume_unwind(payload);
        }
    };
    publish_snapshot_restore_outcome(
        ecs_world,
        outcome_messages,
        BoxddSnapshotRestoreMessage {
            ticket: pending.ticket,
            result,
        },
    );
    let Err(error) = result else {
        return;
    };

    let message = BoxddErrorMessage {
        operation: BoxddOperation::RestoreSnapshot,
        entity: None,
        error: BoxddPluginError::Snapshot(error),
    };
    match ecs_world
        .get_resource::<BoxddErrorPolicy>()
        .copied()
        .unwrap_or_default()
    {
        BoxddErrorPolicy::MessageOnly => {
            ecs_world.write_message(message);
        }
        BoxddErrorPolicy::MessageAndLog => {
            log::error!("{message:?}");
            ecs_world.write_message(message);
        }
        BoxddErrorPolicy::Panic => panic!("{message:?}"),
    }
}

fn publish_snapshot_restore_outcome(
    ecs_world: &mut EcsWorld,
    mut messages: Messages<BoxddSnapshotRestoreMessage>,
    outcome: BoxddSnapshotRestoreMessage,
) {
    // Restore owns the channel for the transaction so component hooks cannot reset Bevy's
    // monotonic reader sequence. Preserve messages written through a temporary replacement too.
    if let Some(mut replacement) =
        ecs_world.remove_resource::<Messages<BoxddSnapshotRestoreMessage>>()
    {
        messages.write_batch(replacement.drain());
    }
    let _message_id = messages.write(outcome);
    ecs_world.insert_resource(messages);
}

impl IdentityGraph {
    fn prepare_restore(
        snapshot: &BoxddPhysicsSnapshot,
        restore: &SnapshotRestore,
    ) -> BoxddResult<Self> {
        if restore.body_mappings().len() != snapshot.bodies.len()
            || restore.shape_mappings().len() != snapshot.shapes.len()
            || restore.joint_mappings().len() != snapshot.joints.len()
            || restore.chain_mappings().len() != 0
        {
            return Err(BoxddError::SnapshotManifestMismatch);
        }

        let mut prepared = Self {
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            body_descriptors: HashMap::new(),
            retired_body_to_entity: HashMap::new(),
            entity_to_shape: HashMap::new(),
            shape_to_entity: HashMap::new(),
            retired_shape_to_entity: HashMap::new(),
            shape_to_body_entity: HashMap::new(),
            shape_descriptors: HashMap::new(),
            entity_to_joint: HashMap::new(),
            joint_to_entity: HashMap::new(),
            retired_joint_to_entity: HashMap::new(),
            joint_descriptors: HashMap::new(),
        };
        reserve_map(&mut prepared.entity_to_body, snapshot.bodies.len())?;
        reserve_map(&mut prepared.body_to_entity, snapshot.bodies.len())?;
        reserve_map(&mut prepared.body_descriptors, snapshot.bodies.len())?;
        reserve_map(&mut prepared.entity_to_shape, snapshot.shapes.len())?;
        reserve_map(&mut prepared.shape_to_entity, snapshot.shapes.len())?;
        reserve_map(&mut prepared.shape_to_body_entity, snapshot.shapes.len())?;
        reserve_map(&mut prepared.shape_descriptors, snapshot.shapes.len())?;
        reserve_map(&mut prepared.entity_to_joint, snapshot.joints.len())?;
        reserve_map(&mut prepared.joint_to_entity, snapshot.joints.len())?;
        reserve_map(&mut prepared.joint_descriptors, snapshot.joints.len())?;

        for (snapshot_id, restored_id) in restore.body_mappings() {
            let body = snapshot
                .bodies
                .get(&snapshot_id)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            if prepared
                .entity_to_body
                .insert(body.entity, restored_id)
                .is_some()
                || prepared
                    .body_to_entity
                    .insert(restored_id, body.entity)
                    .is_some()
                || prepared
                    .body_descriptors
                    .insert(body.entity, body.descriptor)
                    .is_some()
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
        for (snapshot_id, restored_id) in restore.shape_mappings() {
            let shape = snapshot
                .shapes
                .get(&snapshot_id)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            if prepared
                .entity_to_shape
                .insert(shape.entity, restored_id)
                .is_some()
                || prepared
                    .shape_to_entity
                    .insert(restored_id, shape.entity)
                    .is_some()
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            prepared
                .shape_to_body_entity
                .insert(shape.entity, shape.body_entity);
            prepared
                .shape_descriptors
                .insert(shape.entity, shape.descriptor);
        }
        for (snapshot_id, restored_id) in restore.joint_mappings() {
            let joint = snapshot
                .joints
                .get(&snapshot_id)
                .ok_or(BoxddError::SnapshotManifestMismatch)?;
            if prepared
                .entity_to_joint
                .insert(joint.entity, restored_id)
                .is_some()
                || prepared
                    .joint_to_entity
                    .insert(restored_id, joint.entity)
                    .is_some()
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
            prepared
                .joint_descriptors
                .insert(joint.entity, joint.descriptor);
        }

        if prepared.entity_to_body.len() != snapshot.bodies.len()
            || prepared.body_to_entity.len() != snapshot.bodies.len()
            || prepared.body_descriptors.len() != snapshot.bodies.len()
            || prepared.entity_to_shape.len() != snapshot.shapes.len()
            || prepared.shape_to_entity.len() != snapshot.shapes.len()
            || prepared.shape_to_body_entity.len() != snapshot.shapes.len()
            || prepared.shape_descriptors.len() != snapshot.shapes.len()
            || prepared.entity_to_joint.len() != snapshot.joints.len()
            || prepared.joint_to_entity.len() != snapshot.joints.len()
            || prepared.joint_descriptors.len() != snapshot.joints.len()
        {
            return Err(BoxddError::SnapshotManifestMismatch);
        }
        for (&shape_entity, &body_entity) in &prepared.shape_to_body_entity {
            if !prepared.entity_to_shape.contains_key(&shape_entity)
                || !prepared.entity_to_body.contains_key(&body_entity)
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
        for (&joint_entity, descriptor) in &prepared.joint_descriptors {
            if !prepared.entity_to_joint.contains_key(&joint_entity)
                || !prepared.entity_to_body.contains_key(&descriptor.entity_a)
                || !prepared.entity_to_body.contains_key(&descriptor.entity_b)
            {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
        Ok(prepared)
    }
}

fn reserve_map<K, V>(map: &mut HashMap<K, V>, additional: usize) -> BoxddResult<()>
where
    K: Eq + std::hash::Hash,
{
    map.try_reserve(additional)
        .map_err(|_| BoxddError::SnapshotAllocationFailed)
}

fn validate_snapshot_entities(
    ecs_world: &EcsWorld,
    snapshot: &BoxddPhysicsSnapshot,
) -> Result<(), BoxddSnapshotError> {
    for body in snapshot.bodies.values() {
        if ecs_world.get_entity(body.entity).is_err() {
            return Err(BoxddSnapshotError::EntityMissing {
                entity: body.entity,
                kind: BoxddSnapshotObjectKind::Body,
            });
        }
    }
    for shape in snapshot.shapes.values() {
        if ecs_world.get_entity(shape.entity).is_err() {
            return Err(BoxddSnapshotError::EntityMissing {
                entity: shape.entity,
                kind: BoxddSnapshotObjectKind::Shape,
            });
        }
    }
    for joint in snapshot.joints.values() {
        if ecs_world.get_entity(joint.entity).is_err() {
            return Err(BoxddSnapshotError::EntityMissing {
                entity: joint.entity,
                kind: BoxddSnapshotObjectKind::Joint,
            });
        }
    }
    Ok(())
}

fn collect_existing_projections(ecs_world: &EcsWorld) -> BoxddResult<Vec<ExistingProjections>> {
    let count = ecs_world
        .iter_entities()
        .filter(|entity| {
            entity.contains::<crate::BoxddBody>()
                || entity.contains::<crate::BoxddShape>()
                || entity.contains::<crate::BoxddJoint>()
        })
        .count();
    let mut existing = Vec::new();
    existing
        .try_reserve_exact(count)
        .map_err(|_| BoxddError::SnapshotAllocationFailed)?;
    existing.extend(ecs_world.iter_entities().filter_map(|entity| {
        let body = entity.contains::<crate::BoxddBody>();
        let shape = entity.contains::<crate::BoxddShape>();
        let joint = entity.contains::<crate::BoxddJoint>();
        (body || shape || joint).then_some(ExistingProjections {
            entity: entity.id(),
            body,
            shape,
            joint,
        })
    }));
    Ok(existing)
}

impl PreparedProjectionCommit {
    fn prepare(
        ecs_world: &EcsWorld,
        existing: &[ExistingProjections],
        prepared: &IdentityGraph,
    ) -> BoxddResult<Self> {
        let mut commit = Self {
            remove_bodies: Vec::new(),
            remove_shapes: Vec::new(),
            remove_joints: Vec::new(),
            bodies: Vec::new(),
            shapes: Vec::new(),
            joints: Vec::new(),
        };
        reserve_projection_vec(
            &mut commit.remove_bodies,
            existing
                .iter()
                .filter(|projection| {
                    projection.body && !prepared.entity_to_body.contains_key(&projection.entity)
                })
                .count(),
        )?;
        reserve_projection_vec(
            &mut commit.remove_shapes,
            existing
                .iter()
                .filter(|projection| {
                    projection.shape && !prepared.entity_to_shape.contains_key(&projection.entity)
                })
                .count(),
        )?;
        reserve_projection_vec(
            &mut commit.remove_joints,
            existing
                .iter()
                .filter(|projection| {
                    projection.joint && !prepared.entity_to_joint.contains_key(&projection.entity)
                })
                .count(),
        )?;
        reserve_projection_vec(&mut commit.bodies, prepared.entity_to_body.len())?;
        reserve_projection_vec(&mut commit.shapes, prepared.entity_to_shape.len())?;
        reserve_projection_vec(&mut commit.joints, prepared.entity_to_joint.len())?;

        for projection in existing {
            ecs_world
                .get_entity(projection.entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?;
            if projection.body && !prepared.entity_to_body.contains_key(&projection.entity) {
                commit.remove_bodies.push(projection.entity);
            }
            if projection.shape && !prepared.entity_to_shape.contains_key(&projection.entity) {
                commit.remove_shapes.push(projection.entity);
            }
            if projection.joint && !prepared.entity_to_joint.contains_key(&projection.entity) {
                commit.remove_joints.push(projection.entity);
            }
        }

        for (&entity, &id) in &prepared.entity_to_body {
            ecs_world
                .get_entity(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?;
            commit.bodies.push((entity, id));
        }
        for (&entity, &id) in &prepared.entity_to_shape {
            ecs_world
                .get_entity(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?;
            commit.shapes.push((entity, id));
        }
        for (&entity, &id) in &prepared.entity_to_joint {
            ecs_world
                .get_entity(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?;
            commit.joints.push((entity, id));
        }
        commit
            .remove_bodies
            .sort_unstable_by_key(|entity| entity.to_bits());
        commit
            .remove_shapes
            .sort_unstable_by_key(|entity| entity.to_bits());
        commit
            .remove_joints
            .sort_unstable_by_key(|entity| entity.to_bits());
        commit
            .bodies
            .sort_unstable_by_key(|(entity, _)| entity.to_bits());
        commit
            .shapes
            .sort_unstable_by_key(|(entity, _)| entity.to_bits());
        commit
            .joints
            .sort_unstable_by_key(|(entity, _)| entity.to_bits());
        Ok(commit)
    }

    fn commit(self, ecs_world: &mut EcsWorld) -> BoxddResult<()> {
        for entity in self.remove_bodies {
            if let Ok(mut entity) = ecs_world.get_entity_mut(entity) {
                entity.remove::<crate::BoxddBody>();
            }
        }
        for entity in self.remove_shapes {
            if let Ok(mut entity) = ecs_world.get_entity_mut(entity) {
                entity.remove::<crate::BoxddShape>();
            }
        }
        for entity in self.remove_joints {
            if let Ok(mut entity) = ecs_world.get_entity_mut(entity) {
                entity.remove::<crate::BoxddJoint>();
            }
        }
        for (entity, id) in self.bodies {
            ecs_world
                .get_entity_mut(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?
                .insert(crate::BoxddBody::new(id));
        }
        for (entity, id) in self.shapes {
            ecs_world
                .get_entity_mut(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?
                .insert(crate::BoxddShape::new(id));
        }
        for (entity, id) in self.joints {
            ecs_world
                .get_entity_mut(entity)
                .map_err(|_| BoxddError::SnapshotManifestMismatch)?
                .insert(crate::BoxddJoint::new(id));
        }
        Ok(())
    }
}

fn validate_projection_graph(ecs_world: &EcsWorld, graph: &IdentityGraph) -> BoxddResult<()> {
    let mut body_count = 0_usize;
    let mut shape_count = 0_usize;
    let mut joint_count = 0_usize;

    for entity in ecs_world.iter_entities() {
        if let Some(body) = entity.get::<crate::BoxddBody>() {
            body_count += 1;
            if graph.entity_to_body.get(&entity.id()) != Some(&body.id()) {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
        if let Some(shape) = entity.get::<crate::BoxddShape>() {
            shape_count += 1;
            if graph.entity_to_shape.get(&entity.id()) != Some(&shape.id()) {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
        if let Some(joint) = entity.get::<crate::BoxddJoint>() {
            joint_count += 1;
            if graph.entity_to_joint.get(&entity.id()) != Some(&joint.id()) {
                return Err(BoxddError::SnapshotManifestMismatch);
            }
        }
    }

    if body_count != graph.entity_to_body.len()
        || shape_count != graph.entity_to_shape.len()
        || joint_count != graph.entity_to_joint.len()
    {
        return Err(BoxddError::SnapshotManifestMismatch);
    }
    Ok(())
}

fn reserve_projection_vec<T>(values: &mut Vec<T>, additional: usize) -> BoxddResult<()> {
    values
        .try_reserve_exact(additional)
        .map_err(|_| BoxddError::SnapshotAllocationFailed)
}

fn remove_all_projections(ecs_world: &mut EcsWorld, existing: &[ExistingProjections]) {
    for projection in existing {
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        if projection.body {
            remove_projection::<crate::BoxddBody>(ecs_world, projection.entity);
        }
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        if projection.shape {
            remove_projection::<crate::BoxddShape>(ecs_world, projection.entity);
        }
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        if projection.joint {
            remove_projection::<crate::BoxddJoint>(ecs_world, projection.entity);
        }
    }
}

fn remove_snapshot_projections(ecs_world: &mut EcsWorld, snapshot: &BoxddPhysicsSnapshot) {
    for body in snapshot.bodies.values() {
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        remove_projection::<crate::BoxddBody>(ecs_world, body.entity);
    }
    for shape in snapshot.shapes.values() {
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        remove_projection::<crate::BoxddShape>(ecs_world, shape.entity);
    }
    for joint in snapshot.joints.values() {
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return;
        }
        remove_projection::<crate::BoxddJoint>(ecs_world, joint.entity);
    }
}

fn remove_projection<T: bevy_ecs::component::Component>(ecs_world: &mut EcsWorld, entity: Entity) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut entity) = ecs_world.get_entity_mut(entity) {
            entity.remove::<T>();
        }
    }));
    if let Err(payload) = result {
        suppress_panic_payload(payload);
    }
}

fn remove_projections_not_authorized_by_replacement(ecs_world: &mut EcsWorld) -> bool {
    const CLEANUP_PASSES: usize = 4;

    for _ in 0..CLEANUP_PASSES {
        let Ok(existing) = collect_existing_projections(ecs_world) else {
            return false;
        };
        for projection in existing {
            if projection.body && body_projection_is_unauthorized(ecs_world, projection.entity) {
                remove_projection::<crate::BoxddBody>(ecs_world, projection.entity);
            }
            if projection.shape && shape_projection_is_unauthorized(ecs_world, projection.entity) {
                remove_projection::<crate::BoxddShape>(ecs_world, projection.entity);
            }
            if projection.joint && joint_projection_is_unauthorized(ecs_world, projection.entity) {
                remove_projection::<crate::BoxddJoint>(ecs_world, projection.entity);
            }
        }

        let Some(context) = ecs_world.get_non_send::<BoxddPhysicsContext>() else {
            return false;
        };
        if validate_projection_graph(ecs_world, &context.graph).is_ok() {
            return true;
        }
    }

    false
}

fn body_projection_is_unauthorized(ecs_world: &EcsWorld, entity: Entity) -> bool {
    let Some(context) = ecs_world.get_non_send::<BoxddPhysicsContext>() else {
        return false;
    };
    ecs_world
        .get::<crate::BoxddBody>(entity)
        .is_some_and(|projection| context.body_projection(entity, projection).is_none())
}

fn shape_projection_is_unauthorized(ecs_world: &EcsWorld, entity: Entity) -> bool {
    let Some(context) = ecs_world.get_non_send::<BoxddPhysicsContext>() else {
        return false;
    };
    ecs_world
        .get::<crate::BoxddShape>(entity)
        .is_some_and(|projection| context.shape_projection(entity, projection).is_none())
}

fn joint_projection_is_unauthorized(ecs_world: &EcsWorld, entity: Entity) -> bool {
    let Some(context) = ecs_world.get_non_send::<BoxddPhysicsContext>() else {
        return false;
    };
    ecs_world
        .get::<crate::BoxddJoint>(entity)
        .is_some_and(|projection| context.joint_projection(entity, projection).is_none())
}

fn suppress_panic_payload(payload: Box<dyn std::any::Any + Send + 'static>) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        std::mem::forget(secondary);
    }
}

fn remove_remaining_projections(ecs_world: &mut EcsWorld) -> bool {
    const CLEANUP_PASSES: usize = 4;

    for _ in 0..CLEANUP_PASSES {
        if RestoreContextSlotDisposition::current(ecs_world)
            == RestoreContextSlotDisposition::Replaced
        {
            return true;
        }
        let Ok(existing) = collect_existing_projections(ecs_world) else {
            return false;
        };
        if existing.is_empty() {
            return true;
        }
        remove_all_projections(ecs_world, &existing);
    }

    if RestoreContextSlotDisposition::current(ecs_world) == RestoreContextSlotDisposition::Replaced
    {
        return true;
    }
    collect_existing_projections(ecs_world).is_ok_and(|existing| existing.is_empty())
}
