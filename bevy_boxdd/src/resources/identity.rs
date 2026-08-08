use super::{BodyDescriptor, BoxddPhysicsContext, ShapeDescriptor};
use crate::components::JointDescriptor;
use crate::messages::BoxddPluginError;
use bevy_ecs::prelude::Entity;
use boxdd::{
    BodyId, Error as BoxddError, JointId, JointType, Result as BoxddResult, ShapeId, WorldTransform,
};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct IdentityGraph {
    pub(super) entity_to_body: HashMap<Entity, BodyId>,
    pub(super) body_to_entity: HashMap<BodyId, Entity>,
    pub(super) body_descriptors: HashMap<Entity, BodyDescriptor>,
    pub(super) retired_body_to_entity: HashMap<BodyId, Entity>,
    pub(super) entity_to_shape: HashMap<Entity, ShapeId>,
    pub(super) shape_to_entity: HashMap<ShapeId, Entity>,
    pub(super) retired_shape_to_entity: HashMap<ShapeId, Entity>,
    pub(super) shape_to_body_entity: HashMap<Entity, Entity>,
    pub(super) shape_descriptors: HashMap<Entity, ShapeDescriptor>,
    pub(super) entity_to_joint: HashMap<Entity, JointId>,
    pub(super) joint_to_entity: HashMap<JointId, Entity>,
    pub(super) retired_joint_to_entity: HashMap<JointId, Entity>,
    pub(super) joint_descriptors: HashMap<Entity, JointDescriptor>,
}

#[derive(Copy, Clone)]
pub(crate) struct EventEntityLookup<'graph> {
    graph: &'graph IdentityGraph,
}

impl EventEntityLookup<'_> {
    pub(crate) fn body(self, body_id: BodyId) -> Option<Entity> {
        self.graph
            .active_body_entity(body_id)
            .or_else(|| self.graph.retired_body_to_entity.get(&body_id).copied())
    }

    pub(crate) fn shape(self, shape_id: ShapeId) -> Option<Entity> {
        self.graph
            .active_shape_entity(shape_id)
            .or_else(|| self.graph.retired_shape_to_entity.get(&shape_id).copied())
    }

    pub(crate) fn joint(self, joint_id: JointId) -> Option<Entity> {
        self.graph
            .active_joint_entity(joint_id)
            .or_else(|| self.graph.retired_joint_to_entity.get(&joint_id).copied())
    }
}

#[derive(Default)]
pub(crate) struct BodyDependents {
    pub(crate) shapes: Vec<(Entity, ShapeId)>,
    pub(crate) joints: Vec<(Entity, JointId)>,
}

impl BoxddPhysicsContext {
    /// Returns the Bevy entity mapped to a native body id.
    pub fn body_entity(&self, body_id: BodyId) -> Option<Entity> {
        self.graph.active_body_entity(body_id)
    }

    /// Returns the transform of a plugin-managed native body.
    pub fn body_transform(&mut self, body_id: BodyId) -> Result<WorldTransform, BoxddPluginError> {
        self.plugin_world()?;
        self.body_entity(body_id).ok_or(BoxddError::InvalidBodyId)?;
        Ok(self.plugin_world_mut()?.body(body_id)?.transform()?)
    }

    /// Returns the Bevy entity mapped to a native shape id.
    pub fn shape_entity(&self, shape_id: ShapeId) -> Option<Entity> {
        self.graph.active_shape_entity(shape_id)
    }

    /// Returns the Bevy body entity that owns a plugin-managed shape.
    pub fn shape_owner_entity(&self, shape_id: ShapeId) -> Option<Entity> {
        let shape_entity = self.shape_entity(shape_id)?;
        self.shape_body_entity(shape_entity)
    }

    /// Returns the Bevy entity mapped to a native joint id.
    pub fn joint_entity(&self, joint_id: JointId) -> Option<Entity> {
        self.graph.active_joint_entity(joint_id)
    }

    /// Returns the Bevy endpoint entities of a plugin-managed joint.
    pub fn joint_endpoint_entities(&self, joint_id: JointId) -> Option<(Entity, Entity)> {
        let joint_entity = self.joint_entity(joint_id)?;
        let descriptor = self.graph.joint_descriptors.get(&joint_entity)?;
        self.authoritative_body(descriptor.entity_a)?;
        self.authoritative_body(descriptor.entity_b)?;
        Some((descriptor.entity_a, descriptor.entity_b))
    }

    /// Returns the native body that owns a plugin-managed shape.
    pub fn shape_body_id(&mut self, shape_id: ShapeId) -> Result<BodyId, BoxddPluginError> {
        self.plugin_world()?;
        self.shape_entity(shape_id)
            .ok_or(BoxddError::InvalidShapeId)?;
        Ok(self.plugin_world_mut()?.shape(shape_id)?.body_id()?)
    }

    /// Returns the native endpoint bodies of a plugin-managed joint.
    pub fn joint_body_ids(
        &mut self,
        joint_id: JointId,
    ) -> Result<(BodyId, BodyId), BoxddPluginError> {
        self.plugin_world()?;
        self.joint_entity(joint_id)
            .ok_or(BoxddError::InvalidJointId)?;
        let joint = self.plugin_world_mut()?.joint(joint_id)?;
        Ok((joint.body_a_id()?, joint.body_b_id()?))
    }

    /// Returns the constraint type of a plugin-managed joint.
    pub fn joint_type(&mut self, joint_id: JointId) -> Result<JointType, BoxddPluginError> {
        self.plugin_world()?;
        self.joint_entity(joint_id)
            .ok_or(BoxddError::InvalidJointId)?;
        Ok(self.plugin_world_mut()?.joint(joint_id)?.joint_type()?)
    }

    /// Returns the local frame on body A for a plugin-managed joint.
    pub fn joint_local_frame_a(
        &mut self,
        joint_id: JointId,
    ) -> Result<boxdd::Transform, BoxddPluginError> {
        self.plugin_world()?;
        self.joint_entity(joint_id)
            .ok_or(BoxddError::InvalidJointId)?;
        Ok(self.plugin_world_mut()?.joint(joint_id)?.local_frame_a()?)
    }

    pub(crate) fn authoritative_body(&self, entity: Entity) -> Option<BodyId> {
        self.graph.authoritative_body(entity)
    }

    pub(crate) fn tracked_bodies(&self) -> impl ExactSizeIterator<Item = (Entity, BodyId)> + '_ {
        self.graph
            .entity_to_body
            .iter()
            .map(|(&entity, &id)| (entity, id))
    }

    pub(crate) fn authoritative_shape(&self, entity: Entity) -> Option<ShapeId> {
        self.graph.authoritative_shape(entity)
    }

    pub(crate) fn tracked_shapes(&self) -> impl ExactSizeIterator<Item = (Entity, ShapeId)> + '_ {
        self.graph
            .entity_to_shape
            .iter()
            .map(|(&entity, &id)| (entity, id))
    }

    pub(crate) fn authoritative_joint(&self, entity: Entity) -> Option<JointId> {
        self.graph.authoritative_joint(entity)
    }

    pub(crate) fn tracked_joints(&self) -> impl ExactSizeIterator<Item = (Entity, JointId)> + '_ {
        self.graph
            .entity_to_joint
            .iter()
            .map(|(&entity, &id)| (entity, id))
    }

    pub(crate) fn body_projection(
        &self,
        entity: Entity,
        projection: &crate::BoxddBody,
    ) -> Option<BodyId> {
        let id = self.authoritative_body(entity)?;
        (projection.id() == id).then_some(id)
    }

    pub(crate) fn shape_projection(
        &self,
        entity: Entity,
        projection: &crate::BoxddShape,
    ) -> Option<ShapeId> {
        let id = self.authoritative_shape(entity)?;
        (projection.id() == id).then_some(id)
    }

    pub(crate) fn joint_projection(
        &self,
        entity: Entity,
        projection: &crate::BoxddJoint,
    ) -> Option<JointId> {
        let id = self.authoritative_joint(entity)?;
        (projection.id() == id).then_some(id)
    }

    pub(super) fn insert_body(
        &mut self,
        entity: Entity,
        body_id: BodyId,
        descriptor: BodyDescriptor,
    ) {
        self.graph.entity_to_body.insert(entity, body_id);
        self.graph.body_to_entity.insert(body_id, entity);
        self.graph.body_descriptors.insert(entity, descriptor);
    }

    pub(super) fn remove_body(
        &mut self,
        entity: Entity,
        body_id: BodyId,
        dependents: &BodyDependents,
    ) -> bool {
        if self.authoritative_body(entity) != Some(body_id) {
            return false;
        }

        self.graph.entity_to_body.remove(&entity);
        self.graph.body_to_entity.remove(&body_id);
        self.graph.body_descriptors.remove(&entity);
        self.graph.retired_body_to_entity.insert(body_id, entity);

        for &(shape_entity, shape_id) in &dependents.shapes {
            self.remove_shape(shape_entity, shape_id);
        }
        for &(joint_entity, joint_id) in &dependents.joints {
            self.remove_joint(joint_entity, joint_id);
        }
        true
    }

    pub(crate) fn body_descriptor(&self, entity: Entity) -> Option<BodyDescriptor> {
        self.authoritative_body(entity)?;
        self.graph.body_descriptors.get(&entity).copied()
    }

    pub(super) fn set_body_descriptor(&mut self, entity: Entity, descriptor: BodyDescriptor) {
        debug_assert!(self.authoritative_body(entity).is_some());
        self.graph.body_descriptors.insert(entity, descriptor);
    }

    pub(crate) fn body_dependents_for_batch(
        &self,
        bodies: &[(Entity, BodyId)],
    ) -> BoxddResult<Vec<BodyDependents>> {
        debug_assert!(
            bodies
                .windows(2)
                .all(|pair| pair[0].0.to_bits() < pair[1].0.to_bits())
        );

        let body_index = |entity: Entity| {
            bodies
                .binary_search_by_key(&entity.to_bits(), |(candidate, _)| candidate.to_bits())
                .ok()
        };
        let mut dependents = Vec::new();
        dependents
            .try_reserve_exact(bodies.len())
            .map_err(|_| BoxddError::IdentityTrackingAllocationFailed)?;
        dependents.resize_with(bodies.len(), BodyDependents::default);

        for (&shape_entity, &owner) in &self.graph.shape_to_body_entity {
            let Some(index) = body_index(owner) else {
                continue;
            };
            let Some(shape_id) = self.authoritative_shape(shape_entity) else {
                continue;
            };
            dependents[index]
                .shapes
                .try_reserve(1)
                .map_err(|_| BoxddError::IdentityTrackingAllocationFailed)?;
            dependents[index].shapes.push((shape_entity, shape_id));
        }

        for (&joint_entity, descriptor) in &self.graph.joint_descriptors {
            let Some(joint_id) = self.authoritative_joint(joint_entity) else {
                continue;
            };
            let index = match (
                body_index(descriptor.entity_a),
                body_index(descriptor.entity_b),
            ) {
                (Some(a), Some(b)) => a.min(b),
                (Some(index), None) | (None, Some(index)) => index,
                (None, None) => continue,
            };
            dependents[index]
                .joints
                .try_reserve(1)
                .map_err(|_| BoxddError::IdentityTrackingAllocationFailed)?;
            dependents[index].joints.push((joint_entity, joint_id));
        }

        for body_dependents in &mut dependents {
            body_dependents
                .shapes
                .sort_unstable_by_key(|(entity, _)| entity.to_bits());
            body_dependents
                .joints
                .sort_unstable_by_key(|(entity, _)| entity.to_bits());
        }
        Ok(dependents)
    }

    pub(super) fn insert_shape(
        &mut self,
        entity: Entity,
        body_entity: Entity,
        descriptor: ShapeDescriptor,
        shape_id: ShapeId,
    ) {
        self.graph.entity_to_shape.insert(entity, shape_id);
        self.graph.shape_to_entity.insert(shape_id, entity);
        self.graph.shape_to_body_entity.insert(entity, body_entity);
        self.graph.shape_descriptors.insert(entity, descriptor);
    }

    pub(super) fn remove_shape(&mut self, entity: Entity, shape_id: ShapeId) -> bool {
        if self.authoritative_shape(entity) != Some(shape_id) {
            return false;
        }

        self.graph.entity_to_shape.remove(&entity);
        self.graph.shape_to_entity.remove(&shape_id);
        self.graph.retired_shape_to_entity.insert(shape_id, entity);
        self.graph.shape_to_body_entity.remove(&entity);
        self.graph.shape_descriptors.remove(&entity);
        true
    }

    pub(super) fn replace_shape_mapping(
        &mut self,
        entity: Entity,
        old_id: ShapeId,
        new_id: ShapeId,
        body_entity: Entity,
        descriptor: ShapeDescriptor,
    ) {
        debug_assert_eq!(self.authoritative_shape(entity), Some(old_id));
        self.graph.entity_to_shape.insert(entity, new_id);
        self.graph.shape_to_entity.remove(&old_id);
        self.graph.shape_to_entity.insert(new_id, entity);
        self.graph.retired_shape_to_entity.insert(old_id, entity);
        self.graph.shape_to_body_entity.insert(entity, body_entity);
        self.graph.shape_descriptors.insert(entity, descriptor);
    }

    pub(crate) fn shape_body_entity(&self, shape_entity: Entity) -> Option<Entity> {
        self.authoritative_shape(shape_entity)?;
        let body_entity = *self.graph.shape_to_body_entity.get(&shape_entity)?;
        self.authoritative_body(body_entity)?;
        Some(body_entity)
    }

    pub(crate) fn shape_descriptor(&self, shape_entity: Entity) -> Option<ShapeDescriptor> {
        self.authoritative_shape(shape_entity)?;
        self.graph.shape_descriptors.get(&shape_entity).copied()
    }

    pub(super) fn insert_joint(
        &mut self,
        entity: Entity,
        descriptor: JointDescriptor,
        joint_id: JointId,
    ) {
        self.graph.entity_to_joint.insert(entity, joint_id);
        self.graph.joint_to_entity.insert(joint_id, entity);
        self.graph.joint_descriptors.insert(entity, descriptor);
    }

    pub(super) fn remove_joint(&mut self, entity: Entity, joint_id: JointId) -> bool {
        if self.authoritative_joint(entity) != Some(joint_id) {
            return false;
        }

        self.graph.entity_to_joint.remove(&entity);
        self.graph.joint_to_entity.remove(&joint_id);
        self.graph.retired_joint_to_entity.insert(joint_id, entity);
        self.graph.joint_descriptors.remove(&entity);
        true
    }

    pub(super) fn replace_joint_mapping(
        &mut self,
        entity: Entity,
        old_id: JointId,
        new_id: JointId,
        descriptor: JointDescriptor,
    ) {
        debug_assert_eq!(self.authoritative_joint(entity), Some(old_id));
        self.graph.entity_to_joint.insert(entity, new_id);
        self.graph.joint_to_entity.remove(&old_id);
        self.graph.joint_to_entity.insert(new_id, entity);
        self.graph.retired_joint_to_entity.insert(old_id, entity);
        self.graph.joint_descriptors.insert(entity, descriptor);
    }

    pub(crate) fn joint_descriptor(&self, entity: Entity) -> Option<JointDescriptor> {
        self.authoritative_joint(entity)?;
        self.graph.joint_descriptors.get(&entity).copied()
    }
}

impl IdentityGraph {
    fn active_body_entity(&self, body_id: BodyId) -> Option<Entity> {
        let entity = *self.body_to_entity.get(&body_id)?;
        (self.entity_to_body.get(&entity) == Some(&body_id)).then_some(entity)
    }

    fn active_shape_entity(&self, shape_id: ShapeId) -> Option<Entity> {
        let entity = *self.shape_to_entity.get(&shape_id)?;
        (self.entity_to_shape.get(&entity) == Some(&shape_id)).then_some(entity)
    }

    fn active_joint_entity(&self, joint_id: JointId) -> Option<Entity> {
        let entity = *self.joint_to_entity.get(&joint_id)?;
        (self.entity_to_joint.get(&entity) == Some(&joint_id)).then_some(entity)
    }

    pub(super) fn authoritative_body(&self, entity: Entity) -> Option<BodyId> {
        let id = *self.entity_to_body.get(&entity)?;
        (self.body_to_entity.get(&id) == Some(&entity)).then_some(id)
    }

    pub(super) fn authoritative_shape(&self, entity: Entity) -> Option<ShapeId> {
        let id = *self.entity_to_shape.get(&entity)?;
        (self.shape_to_entity.get(&id) == Some(&entity)).then_some(id)
    }

    pub(super) fn authoritative_joint(&self, entity: Entity) -> Option<JointId> {
        let id = *self.entity_to_joint.get(&entity)?;
        (self.joint_to_entity.get(&id) == Some(&entity)).then_some(id)
    }

    pub(super) fn reserve_body(&mut self) -> BoxddResult<()> {
        reserve_identity_map(&mut self.entity_to_body, 1)?;
        reserve_identity_map(&mut self.body_to_entity, 1)?;
        reserve_identity_map(&mut self.body_descriptors, 1)
    }

    pub(super) fn reserve_shape(&mut self) -> BoxddResult<()> {
        reserve_identity_map(&mut self.entity_to_shape, 1)?;
        reserve_identity_map(&mut self.shape_to_entity, 1)?;
        reserve_identity_map(&mut self.shape_to_body_entity, 1)?;
        reserve_identity_map(&mut self.shape_descriptors, 1)
    }

    pub(super) fn reserve_joint(&mut self) -> BoxddResult<()> {
        reserve_identity_map(&mut self.entity_to_joint, 1)?;
        reserve_identity_map(&mut self.joint_to_entity, 1)?;
        reserve_identity_map(&mut self.joint_descriptors, 1)
    }

    pub(super) fn reserve_retired_body_dependents(
        &mut self,
        dependents: &BodyDependents,
    ) -> BoxddResult<()> {
        reserve_identity_map(&mut self.retired_body_to_entity, 1)?;
        reserve_identity_map(&mut self.retired_shape_to_entity, dependents.shapes.len())?;
        reserve_identity_map(&mut self.retired_joint_to_entity, dependents.joints.len())
    }

    pub(super) fn reserve_retired_shape(&mut self) -> BoxddResult<()> {
        reserve_identity_map(&mut self.retired_shape_to_entity, 1)
    }

    pub(super) fn reserve_retired_joint(&mut self) -> BoxddResult<()> {
        reserve_identity_map(&mut self.retired_joint_to_entity, 1)
    }

    pub(super) fn event_lookup(&self) -> EventEntityLookup<'_> {
        EventEntityLookup { graph: self }
    }

    pub(super) fn release_retired_event_identities(&mut self) {
        self.retired_body_to_entity.clear();
        self.retired_shape_to_entity.clear();
        self.retired_joint_to_entity.clear();
    }
}

fn reserve_identity_map<K, V>(map: &mut HashMap<K, V>, additional: usize) -> BoxddResult<()>
where
    K: Eq + std::hash::Hash,
{
    map.try_reserve(additional)
        .map_err(|_| BoxddError::IdentityTrackingAllocationFailed)
}
