use super::{BodyDependents, BoxddPhysicsContext};
use crate::components::{
    BodySettings, Collider, DistanceJointDescriptor, JointDescriptor, JointKind, PhysicsMaterial,
    RevoluteJointDescriptor, RigidBody,
};
use crate::math::to_boxdd_vec2;
use bevy_ecs::prelude::Entity;
use bevy_math::Vec2 as BevyVec2;
use boxdd::{
    BodyDef, BodyId, Capsule as BoxddCapsule, Circle as BoxddCircle, DistanceJointDef,
    Error as BoxddError, JointId, Polygon as BoxddPolygon, Position, Result as BoxddResult,
    RevoluteJointDef, Segment as BoxddSegment, ShapeDef, ShapeId, World,
};

impl BoxddPhysicsContext {
    pub(crate) fn create_body(
        &mut self,
        entity: Entity,
        def: BodyDef,
        descriptor: BodyDescriptor,
    ) -> BoxddResult<BodyId> {
        if self.graph.entity_to_body.contains_key(&entity) {
            return Err(BoxddError::InvalidBodyId);
        }
        self.graph.reserve_body()?;
        let body_id = self.live_world_mut()?.create_body(def)?;
        self.insert_body(entity, body_id, descriptor);
        Ok(body_id)
    }

    pub(crate) fn destroy_body(
        &mut self,
        entity: Entity,
        body_id: BodyId,
        dependents: BodyDependents,
    ) -> BoxddResult<BodyDependents> {
        if self.authoritative_body(entity) != Some(body_id) {
            return Err(BoxddError::InvalidBodyId);
        }
        self.graph.reserve_retired_body_dependents(&dependents)?;
        self.live_world_mut()?.body(body_id)?.destroy()?;
        self.remove_body(entity, body_id, &dependents);
        Ok(dependents)
    }

    pub(crate) fn apply_body_settings(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        rigid_body: RigidBody,
        settings: BodySettings,
    ) -> BoxddResult<()> {
        settings.validate()?;
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        {
            let world = self.live_world_mut()?;
            let mut body = world.body(body_id)?;
            body.set_body_type(rigid_body.into())?;
            body.set_gravity_scale(settings.gravity_scale)?;
            body.set_linear_damping(settings.linear_damping)?;
            body.set_angular_damping(settings.angular_damping)?;
            body.enable_sleep(settings.sleep_enabled)?;
            body.set_bullet(settings.bullet)?;
            body.set_motion_locks(settings.motion_locks)?;
        }
        self.set_body_descriptor(
            entity,
            BodyDescriptor {
                rigid_body,
                settings,
            },
        );
        Ok(())
    }

    pub(crate) fn create_shape(
        &mut self,
        entity: Entity,
        body_entity: Entity,
        descriptor: ShapeDescriptor,
    ) -> BoxddResult<ShapeId> {
        if self.graph.entity_to_shape.contains_key(&entity) {
            return Err(BoxddError::InvalidShapeId);
        }
        descriptor.collider.validate()?;
        descriptor.material.validate()?;
        let body_id = self
            .authoritative_body(body_entity)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.graph.reserve_shape()?;
        let shape_def = descriptor.material.shape_def()?;
        let shape_id = create_native_shape(
            self.live_world_mut()?,
            body_id,
            descriptor.collider,
            descriptor.local_transform,
            &shape_def,
        )?;
        self.insert_shape(entity, body_entity, descriptor, shape_id);
        Ok(shape_id)
    }

    pub(crate) fn destroy_shape(&mut self, entity: Entity, shape_id: ShapeId) -> BoxddResult<()> {
        if self.authoritative_shape(entity) != Some(shape_id) {
            return Err(BoxddError::InvalidShapeId);
        }
        self.graph.reserve_retired_shape()?;
        self.live_world_mut()?.shape(shape_id)?.destroy(true)?;
        self.remove_shape(entity, shape_id);
        Ok(())
    }

    pub(crate) fn replace_shape(
        &mut self,
        entity: Entity,
        old_id: ShapeId,
        body_entity: Entity,
        descriptor: ShapeDescriptor,
    ) -> BoxddResult<ShapeId> {
        if self.authoritative_shape(entity) != Some(old_id) {
            return Err(BoxddError::InvalidShapeId);
        }
        descriptor.collider.validate()?;
        descriptor.material.validate()?;
        let body_id = self
            .authoritative_body(body_entity)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.graph.reserve_retired_shape()?;

        let shape_def = descriptor.material.shape_def()?;
        let new_id = create_native_shape(
            self.live_world_mut()?,
            body_id,
            descriptor.collider,
            descriptor.local_transform,
            &shape_def,
        )?;
        if let Err(error) = self.live_world_mut()?.shape(old_id)?.destroy(true) {
            let rollback_failed = self
                .live_world_mut()
                .and_then(|world| world.shape(new_id))
                .and_then(|shape| shape.destroy(true))
                .is_err();
            if rollback_failed {
                self.disable_lifecycle_transaction();
            }
            return Err(error);
        }
        self.replace_shape_mapping(entity, old_id, new_id, body_entity, descriptor);
        Ok(new_id)
    }

    pub(crate) fn create_joint(
        &mut self,
        entity: Entity,
        descriptor: JointDescriptor,
    ) -> BoxddResult<JointId> {
        if self.graph.entity_to_joint.contains_key(&entity) {
            return Err(BoxddError::InvalidJointId);
        }
        descriptor.validate()?;
        let body_a = self
            .authoritative_body(descriptor.entity_a)
            .ok_or(BoxddError::InvalidBodyId)?;
        let body_b = self
            .authoritative_body(descriptor.entity_b)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.graph.reserve_joint()?;
        let joint_id = create_native_joint(self.live_world_mut()?, descriptor, body_a, body_b)?;
        self.insert_joint(entity, descriptor, joint_id);
        Ok(joint_id)
    }

    pub(crate) fn destroy_joint(&mut self, entity: Entity, joint_id: JointId) -> BoxddResult<()> {
        if self.authoritative_joint(entity) != Some(joint_id) {
            return Err(BoxddError::InvalidJointId);
        }
        self.graph.reserve_retired_joint()?;
        self.live_world_mut()?.joint(joint_id)?.destroy(true)?;
        self.remove_joint(entity, joint_id);
        Ok(())
    }

    pub(crate) fn replace_joint(
        &mut self,
        entity: Entity,
        old_id: JointId,
        descriptor: JointDescriptor,
    ) -> BoxddResult<JointId> {
        if self.authoritative_joint(entity) != Some(old_id) {
            return Err(BoxddError::InvalidJointId);
        }
        descriptor.validate()?;
        let body_a = self
            .authoritative_body(descriptor.entity_a)
            .ok_or(BoxddError::InvalidBodyId)?;
        let body_b = self
            .authoritative_body(descriptor.entity_b)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.graph.reserve_retired_joint()?;

        let new_id = create_native_joint(self.live_world_mut()?, descriptor, body_a, body_b)?;
        if let Err(error) = self.live_world_mut()?.joint(old_id)?.destroy(true) {
            let rollback_failed = self
                .live_world_mut()
                .and_then(|world| world.joint(new_id))
                .and_then(|joint| joint.destroy(true))
                .is_err();
            if rollback_failed {
                self.disable_lifecycle_transaction();
            }
            return Err(error);
        }
        self.replace_joint_mapping(entity, old_id, new_id, descriptor);
        Ok(new_id)
    }

    fn disable_lifecycle_transaction(&mut self) {
        self.world = None;
        self.graph = Default::default();
        self.disabled_reason = Some(crate::BoxddContextDisabledReason::LifecycleTransactionFailed);
        self.last_step_failed = true;
    }

    pub(crate) fn set_body_linear_velocity(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        velocity: BevyVec2,
    ) -> BoxddResult<()> {
        if !velocity.is_finite() {
            return Err(BoxddError::invalid_argument(
                "BoxddPhysicsContext::set_body_linear_velocity",
                "velocity",
                "a finite vector",
            ));
        }
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.live_world_mut()?
            .body(body_id)?
            .set_linear_velocity(to_boxdd_vec2(velocity))
    }

    pub(crate) fn set_body_angular_velocity(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        velocity: f32,
    ) -> BoxddResult<()> {
        if !velocity.is_finite() {
            return Err(BoxddError::invalid_argument(
                "BoxddPhysicsContext::set_body_angular_velocity",
                "velocity",
                "a finite value",
            ));
        }
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.live_world_mut()?
            .body(body_id)?
            .set_angular_velocity(velocity)
    }

    pub(crate) fn apply_body_linear_impulse(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        impulse: BevyVec2,
        wake: bool,
    ) -> BoxddResult<()> {
        if !impulse.is_finite() {
            return Err(BoxddError::invalid_argument(
                "BoxddPhysicsContext::apply_body_linear_impulse",
                "impulse",
                "a finite vector",
            ));
        }
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.live_world_mut()?
            .body(body_id)?
            .apply_linear_impulse_to_center(to_boxdd_vec2(impulse), wake)
    }

    pub(crate) fn apply_body_angular_impulse(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        impulse: f32,
        wake: bool,
    ) -> BoxddResult<()> {
        if !impulse.is_finite() {
            return Err(BoxddError::invalid_argument(
                "BoxddPhysicsContext::apply_body_angular_impulse",
                "impulse",
                "a finite value",
            ));
        }
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.live_world_mut()?
            .body(body_id)?
            .apply_angular_impulse(impulse, wake)
    }

    pub(crate) fn set_body_transform(
        &mut self,
        entity: Entity,
        projection: &crate::BoxddBody,
        position: Position,
        angle: f32,
    ) -> BoxddResult<()> {
        let body_id = self
            .body_projection(entity, projection)
            .ok_or(BoxddError::InvalidBodyId)?;
        self.live_world_mut()?
            .body(body_id)?
            .set_position_and_rotation(position, angle)
    }
}

fn create_native_shape(
    world: &mut World,
    body_id: BodyId,
    collider: Collider,
    local_transform: ShapeLocalTransform,
    shape_def: &ShapeDef,
) -> BoxddResult<ShapeId> {
    let mut body = world.body(body_id)?;
    let has_identity_local_transform = local_transform == ShapeLocalTransform::IDENTITY;
    let local_transform = to_boxdd_local_transform(local_transform)?;
    match collider {
        Collider::Circle { radius, center } => {
            let circle = BoxddCircle::new(transform_local_point(local_transform, center), radius)?;
            body.create_circle(shape_def, &circle)
        }
        Collider::Capsule {
            point1,
            point2,
            radius,
        } => {
            let capsule = BoxddCapsule::new(
                transform_local_point(local_transform, point1),
                transform_local_point(local_transform, point2),
                radius,
            )?;
            body.create_capsule(shape_def, &capsule)
        }
        Collider::Segment { point1, point2 } => {
            let segment = BoxddSegment::new(
                transform_local_point(local_transform, point1),
                transform_local_point(local_transform, point2),
            )?;
            body.create_segment(shape_def, &segment)
        }
        Collider::Rectangle { half_extents } => {
            let polygon = if has_identity_local_transform {
                BoxddPolygon::box_polygon(half_extents.x, half_extents.y)?
            } else {
                BoxddPolygon::offset_box_polygon(half_extents.x, half_extents.y, local_transform)?
            };
            body.create_polygon(shape_def, &polygon)
        }
        Collider::RoundedRectangle {
            half_extents,
            radius,
        } => {
            let polygon = if has_identity_local_transform {
                BoxddPolygon::rounded_box_polygon(half_extents.x, half_extents.y, radius)?
            } else {
                BoxddPolygon::offset_rounded_box_polygon(
                    half_extents.x,
                    half_extents.y,
                    radius,
                    local_transform,
                )?
            };
            body.create_polygon(shape_def, &polygon)
        }
        Collider::ConvexPolygon {
            vertices,
            count,
            radius,
        } => {
            let points = vertices[..count as usize]
                .iter()
                .map(|point| transform_local_point(local_transform, *point));
            let polygon = BoxddPolygon::from_points(points, radius)?;
            body.create_polygon(shape_def, &polygon)
        }
    }
}

fn create_native_joint(
    world: &mut World,
    descriptor: JointDescriptor,
    body_a: BodyId,
    body_b: BodyId,
) -> BoxddResult<JointId> {
    match descriptor.kind {
        JointKind::Distance(distance) => {
            create_distance_joint(world, descriptor, body_a, body_b, distance)
        }
        JointKind::Revolute(revolute) => {
            create_revolute_joint(world, descriptor, body_a, body_b, revolute)
        }
    }
}

fn create_distance_joint(
    world: &mut World,
    descriptor: JointDescriptor,
    body_a: BodyId,
    body_b: BodyId,
    distance: DistanceJointDescriptor,
) -> BoxddResult<JointId> {
    let base = joint_base_from_world_points(
        world,
        descriptor,
        body_a,
        body_b,
        distance.anchor_a,
        distance.anchor_b,
    )?;
    let def = match distance.length {
        Some(length) => DistanceJointDef::new(base).length(length),
        None => DistanceJointDef::new(base)
            .length_from_world_points(distance.anchor_a, distance.anchor_b)?,
    };
    world.create_distance_joint(&def)
}

fn create_revolute_joint(
    world: &mut World,
    descriptor: JointDescriptor,
    body_a: BodyId,
    body_b: BodyId,
    revolute: RevoluteJointDescriptor,
) -> BoxddResult<JointId> {
    let base = joint_base_from_world_points(
        world,
        descriptor,
        body_a,
        body_b,
        revolute.anchor,
        revolute.anchor,
    )?;
    world.create_revolute_joint(&RevoluteJointDef::new(base))
}

fn joint_base_from_world_points(
    world: &mut World,
    descriptor: JointDescriptor,
    body_a: BodyId,
    body_b: BodyId,
    anchor_a: Position,
    anchor_b: Position,
) -> BoxddResult<boxdd::JointBase> {
    let local_a = checked_world_to_body_local(world, body_a, anchor_a)?;
    let local_b = checked_world_to_body_local(world, body_b, anchor_b)?;
    let base = world.joint_base(body_a, body_b)?.with_local_frames(
        boxdd::Transform::from_pos_angle(local_a, 0.0)?,
        boxdd::Transform::from_pos_angle(local_b, 0.0)?,
    );
    base.with_collide_connected(descriptor.collide_connected)
        .with_force_threshold(descriptor.force_threshold)?
        .with_torque_threshold(descriptor.torque_threshold)?
        .with_constraint_tuning(boxdd::ConstraintTuning::new(
            descriptor.constraint_hertz,
            descriptor.constraint_damping_ratio,
        )?)
        .with_draw_scale(descriptor.draw_scale)
}

fn checked_world_to_body_local(
    world: &mut World,
    body: BodyId,
    anchor: Position,
) -> BoxddResult<boxdd::Vec2> {
    world.body(body)?.local_point(anchor)
}

fn to_boxdd_local_transform(value: ShapeLocalTransform) -> BoxddResult<boxdd::Transform> {
    boxdd::Transform::from_pos_angle(to_boxdd_vec2(value.translation), value.angle)
}

fn transform_local_point(transform: boxdd::Transform, point: BevyVec2) -> boxdd::Vec2 {
    transform.transform_point(to_boxdd_vec2(point))
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct ShapeDescriptor {
    pub collider: Collider,
    pub material: PhysicsMaterial,
    pub local_transform: ShapeLocalTransform,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct BodyDescriptor {
    pub rigid_body: RigidBody,
    pub settings: BodySettings,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct ShapeLocalTransform {
    pub translation: BevyVec2,
    pub angle: f32,
}

impl ShapeLocalTransform {
    pub const IDENTITY: Self = Self {
        translation: BevyVec2::ZERO,
        angle: 0.0,
    };

    pub fn from_transform(transform: Option<&bevy_transform::components::Transform>) -> Self {
        transform.map_or(Self::IDENTITY, |transform| Self {
            translation: BevyVec2::new(transform.translation.x, transform.translation.y),
            angle: crate::math::to_boxdd_angle(transform.rotation),
        })
    }
}
