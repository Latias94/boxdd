//! Bevy resources that own the native physics world and plugin settings.

use crate::components::{Collider, PhysicsMaterial};
use crate::math::to_boxdd_vec2;
use bevy_ecs::prelude::{Entity, Resource};
use bevy_math::Vec2 as BevyVec2;
use boxdd::{BodyId, ShapeId, World, WorldDef};
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
    pub(crate) last_step_failed: bool,
}

impl std::fmt::Debug for BoxddPhysicsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxddPhysicsContext")
            .field("world_enabled", &self.world.is_some())
            .field("body_count", &self.entity_to_body.len())
            .field("shape_count", &self.entity_to_shape.len())
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
