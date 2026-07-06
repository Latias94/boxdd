//! Bevy resources that own the native physics world and plugin settings.

use crate::components::{Collider, JointDescriptor, PhysicsMaterial};
use crate::math::to_boxdd_vec2;
use bevy_ecs::prelude::{Entity, Resource};
use bevy_math::Vec2 as BevyVec2;
use boxdd::{
    ApiResult, BodyId, DebugDrawCmd, DebugDrawOptions, JointId, QueryFilter, RayResult, ShapeId,
    World, WorldDef,
};
use std::collections::HashMap;

/// How the plugin reports recoverable errors from fixed-update systems.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum BoxddErrorPolicy {
    /// Emit [`crate::BoxddErrorMessage`] only.
    #[default]
    MessageOnly,
    /// Emit [`crate::BoxddErrorMessage`] and log the error.
    MessageAndLog,
    /// Panic immediately when a recoverable plugin error is observed.
    Panic,
}

/// Runtime settings used by [`crate::BoxddPhysicsPlugin`].
#[derive(Resource, Clone, Debug)]
pub struct BoxddPhysicsSettings {
    /// Gravity used when creating the native Box2D world.
    pub gravity: BevyVec2,
    /// Box2D sub-step count used for each fixed step.
    pub sub_step_count: i32,
    /// Optional Bevy fixed timestep override in seconds.
    pub fixed_timestep_seconds: Option<f64>,
    /// Error reporting policy for plugin systems.
    pub error_policy: BoxddErrorPolicy,
}

impl Default for BoxddPhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: BevyVec2::new(0.0, -10.0),
            sub_step_count: 4,
            fixed_timestep_seconds: Some(1.0 / 60.0),
            error_policy: BoxddErrorPolicy::MessageOnly,
        }
    }
}

/// Non-send resource that owns the native Box2D world and ECS id mappings.
///
/// `boxdd::World` is intentionally `!Send`/`!Sync`; Bevy apps should access
/// this resource from main-thread systems and move plain snapshots across
/// threads when needed.
pub struct BoxddPhysicsContext {
    world: Option<World>,
    pub(crate) entity_to_body: HashMap<Entity, BodyId>,
    pub(crate) body_to_entity: HashMap<BodyId, Entity>,
    pub(crate) entity_to_shape: HashMap<Entity, ShapeId>,
    pub(crate) shape_to_entity: HashMap<ShapeId, Entity>,
    pub(crate) shape_to_body_entity: HashMap<Entity, Entity>,
    pub(crate) shape_descriptors: HashMap<Entity, ShapeDescriptor>,
    pub(crate) entity_to_joint: HashMap<Entity, JointId>,
    pub(crate) joint_to_entity: HashMap<JointId, Entity>,
    pub(crate) joint_descriptors: HashMap<Entity, JointDescriptor>,
    ray_hits: Vec<RayResult>,
    pub(crate) last_step_failed: bool,
}

/// Ray-cast hit enriched with the Bevy entity mapped to the native shape.
#[derive(Copy, Clone, Debug)]
pub struct BoxddRayHit {
    /// Native Box2D ray result.
    pub hit: RayResult,
    /// Bevy entity mapped to `hit.shape_id`, if the shape is owned by this plugin.
    pub entity: Option<Entity>,
}

impl std::fmt::Debug for BoxddPhysicsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxddPhysicsContext")
            .field("world_enabled", &self.world.is_some())
            .field("body_count", &self.entity_to_body.len())
            .field("shape_count", &self.entity_to_shape.len())
            .field("joint_count", &self.entity_to_joint.len())
            .field("last_step_failed", &self.last_step_failed)
            .finish()
    }
}

impl BoxddPhysicsContext {
    /// Creates a context and native Box2D world from plugin settings.
    pub fn new(settings: &BoxddPhysicsSettings) -> Result<Self, boxdd::world::Error> {
        let world = World::new(
            WorldDef::builder()
                .gravity(to_boxdd_vec2(settings.gravity))
                .build(),
        )?;
        Ok(Self::from_world(world))
    }

    /// Creates a context without a native world.
    ///
    /// This is used after startup world creation fails so the app can keep
    /// running while reporting the failure through the configured error policy.
    pub fn disabled() -> Self {
        Self {
            world: None,
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            entity_to_shape: HashMap::new(),
            shape_to_entity: HashMap::new(),
            shape_to_body_entity: HashMap::new(),
            shape_descriptors: HashMap::new(),
            entity_to_joint: HashMap::new(),
            joint_to_entity: HashMap::new(),
            joint_descriptors: HashMap::new(),
            ray_hits: Vec::new(),
            last_step_failed: true,
        }
    }

    /// Creates a context from an existing native world.
    pub fn from_world(world: World) -> Self {
        Self {
            world: Some(world),
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
            entity_to_shape: HashMap::new(),
            shape_to_entity: HashMap::new(),
            shape_to_body_entity: HashMap::new(),
            shape_descriptors: HashMap::new(),
            entity_to_joint: HashMap::new(),
            joint_to_entity: HashMap::new(),
            joint_descriptors: HashMap::new(),
            ray_hits: Vec::new(),
            last_step_failed: false,
        }
    }

    /// Returns the native world, if startup succeeded.
    pub fn world(&self) -> Option<&World> {
        self.world.as_ref()
    }

    /// Returns the native world mutably, if startup succeeded.
    pub fn world_mut(&mut self) -> Option<&mut World> {
        self.world.as_mut()
    }

    /// Returns the Bevy entity mapped to a native body id.
    pub fn body_entity(&self, body_id: BodyId) -> Option<Entity> {
        self.body_to_entity.get(&body_id).copied()
    }

    /// Returns the Bevy entity mapped to a native shape id.
    pub fn shape_entity(&self, shape_id: ShapeId) -> Option<Entity> {
        self.shape_to_entity.get(&shape_id).copied()
    }

    /// Returns the Bevy entity mapped to a native joint id.
    pub fn joint_entity(&self, joint_id: JointId) -> Option<Entity> {
        self.joint_to_entity.get(&joint_id).copied()
    }

    /// Casts a ray and returns the closest hit with the mapped Bevy shape entity.
    pub fn try_cast_ray_closest_entity(
        &self,
        origin: BevyVec2,
        translation: BevyVec2,
        filter: QueryFilter,
    ) -> ApiResult<Option<BoxddRayHit>> {
        let Some(world) = self.world() else {
            return Ok(None);
        };
        let hit = world.try_cast_ray_closest(
            to_boxdd_vec2(origin),
            to_boxdd_vec2(translation),
            filter,
        )?;
        Ok(hit.hit.then(|| self.ray_hit_with_entity(hit)))
    }

    /// Casts a ray and writes all hits with mapped Bevy shape entities into `out`.
    pub fn try_cast_ray_all_entities_into(
        &mut self,
        origin: BevyVec2,
        translation: BevyVec2,
        filter: QueryFilter,
        out: &mut Vec<BoxddRayHit>,
    ) -> ApiResult<()> {
        let Some(world) = self.world.as_ref() else {
            self.ray_hits.clear();
            out.clear();
            return Ok(());
        };
        world.try_cast_ray_all_into(
            to_boxdd_vec2(origin),
            to_boxdd_vec2(translation),
            filter,
            &mut self.ray_hits,
        )?;
        out.clear();
        out.reserve(self.ray_hits.len());
        out.extend(
            self.ray_hits
                .iter()
                .copied()
                .map(|hit| self.ray_hit_with_entity(hit)),
        );
        Ok(())
    }

    /// Casts a ray and returns all hits with mapped Bevy shape entities.
    pub fn try_cast_ray_all_entities(
        &mut self,
        origin: BevyVec2,
        translation: BevyVec2,
        filter: QueryFilter,
    ) -> ApiResult<Vec<BoxddRayHit>> {
        let mut out = Vec::new();
        self.try_cast_ray_all_entities_into(origin, translation, filter, &mut out)?;
        Ok(out)
    }

    /// Collects Box2D debug-draw commands into a caller-owned buffer.
    pub fn try_debug_draw_collect_into(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        options: DebugDrawOptions,
    ) -> ApiResult<()> {
        let Some(world) = self.world_mut() else {
            out.clear();
            return Ok(());
        };
        world.try_debug_draw_collect_into(out, options)
    }

    /// Collects Box2D debug-draw commands into a new vector.
    pub fn try_debug_draw_collect(
        &mut self,
        options: DebugDrawOptions,
    ) -> ApiResult<Vec<DebugDrawCmd>> {
        let mut out = Vec::new();
        self.try_debug_draw_collect_into(&mut out, options)?;
        Ok(out)
    }

    fn ray_hit_with_entity(&self, hit: RayResult) -> BoxddRayHit {
        BoxddRayHit {
            hit,
            entity: self.shape_entity(hit.shape_id),
        }
    }

    pub(crate) fn insert_body(&mut self, entity: Entity, body_id: BodyId) {
        self.entity_to_body.insert(entity, body_id);
        self.body_to_entity.insert(body_id, entity);
    }

    pub(crate) fn remove_body(&mut self, entity: Entity, body_id: BodyId) {
        self.entity_to_body.remove(&entity);
        self.body_to_entity.remove(&body_id);

        let shapes = self
            .shape_to_body_entity
            .iter()
            .filter_map(|(shape_entity, body_entity)| {
                (*body_entity == entity).then_some(*shape_entity)
            })
            .collect::<Vec<_>>();
        for shape_entity in shapes {
            if let Some(shape_id) = self.entity_to_shape.get(&shape_entity).copied() {
                self.remove_shape(shape_entity, shape_id);
            }
        }
    }

    pub(crate) fn insert_shape(
        &mut self,
        entity: Entity,
        body_entity: Entity,
        descriptor: ShapeDescriptor,
        shape_id: ShapeId,
    ) {
        self.entity_to_shape.insert(entity, shape_id);
        self.shape_to_entity.insert(shape_id, entity);
        self.shape_to_body_entity.insert(entity, body_entity);
        self.shape_descriptors.insert(entity, descriptor);
    }

    pub(crate) fn remove_shape(&mut self, entity: Entity, shape_id: ShapeId) {
        self.entity_to_shape.remove(&entity);
        self.shape_to_entity.remove(&shape_id);
        self.shape_to_body_entity.remove(&entity);
        self.shape_descriptors.remove(&entity);
    }

    pub(crate) fn shape_body_entity(&self, shape_entity: Entity) -> Option<Entity> {
        self.shape_to_body_entity.get(&shape_entity).copied()
    }

    pub(crate) fn shape_descriptor(&self, shape_entity: Entity) -> Option<ShapeDescriptor> {
        self.shape_descriptors.get(&shape_entity).copied()
    }

    pub(crate) fn insert_joint(
        &mut self,
        entity: Entity,
        descriptor: JointDescriptor,
        joint_id: JointId,
    ) {
        self.entity_to_joint.insert(entity, joint_id);
        self.joint_to_entity.insert(joint_id, entity);
        self.joint_descriptors.insert(entity, descriptor);
    }

    pub(crate) fn remove_joint(&mut self, entity: Entity, joint_id: JointId) {
        self.entity_to_joint.remove(&entity);
        self.joint_to_entity.remove(&joint_id);
        self.joint_descriptors.remove(&entity);
    }

    pub(crate) fn joint_descriptor(&self, entity: Entity) -> Option<JointDescriptor> {
        self.joint_descriptors.get(&entity).copied()
    }

    pub(crate) fn joints_connected_to_body(&self, body_entity: Entity) -> Vec<(Entity, JointId)> {
        self.joint_descriptors
            .iter()
            .filter_map(|(joint_entity, descriptor)| {
                (descriptor.entity_a == body_entity || descriptor.entity_b == body_entity)
                    .then(|| {
                        self.entity_to_joint
                            .get(joint_entity)
                            .copied()
                            .map(|joint_id| (*joint_entity, joint_id))
                    })
                    .flatten()
            })
            .collect()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct ShapeDescriptor {
    pub collider: Collider,
    pub material: PhysicsMaterial,
    pub local_transform: ShapeLocalTransform,
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
