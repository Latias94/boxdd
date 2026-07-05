//! Bevy messages emitted by the physics plugin.

use bevy_ecs::prelude::{Entity, Message};
use boxdd::{ApiError, BodyId, ContactId, ShapeId, Transform as BoxddTransform, Vec2 as BoxddVec2};

/// Recoverable plugin error type routed through [`BoxddErrorMessage`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoxddPluginError {
    /// Error reported by the safe `boxdd` API.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Native world creation failed after a valid definition was supplied.
    #[error("failed to create Box2D world")]
    CreateWorldFailed,
}

impl From<boxdd::world::Error> for BoxddPluginError {
    fn from(value: boxdd::world::Error) -> Self {
        match value {
            boxdd::world::Error::InvalidDefinition(error) => Self::Api(error),
            boxdd::world::Error::CreateFailed => Self::CreateWorldFailed,
            _ => Self::CreateWorldFailed,
        }
    }
}

/// Plugin operation associated with a recoverable error message.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BoxddOperation {
    /// Creating the native Box2D world.
    CreateWorld,
    /// Creating a native body from a [`crate::RigidBody`] entity.
    CreateBody,
    /// Creating a native shape from a [`crate::Collider`] entity.
    CreateShape,
    /// Destroying a native body after ECS removal or descriptor invalidation.
    DestroyBody,
    /// Destroying a native shape after ECS removal or descriptor invalidation.
    DestroyShape,
    /// Applying velocity or one-shot impulse components.
    ApplyBodyControl,
    /// Applying changed body settings.
    ApplyBodySettings,
    /// Configuring Bevy's fixed timestep resource.
    ConfigureFixedTimestep,
    /// Synchronizing transforms between Bevy and Box2D.
    SyncTransform,
    /// Stepping the native Box2D world.
    StepWorld,
    /// Reading body, contact, or sensor events after a step.
    ReadEvents,
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
    pub transform: BoxddTransform,
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
    /// Native contact id.
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
    /// Contact point reported by Box2D.
    pub point: BoxddVec2,
    /// Contact normal reported by Box2D.
    pub normal: BoxddVec2,
    /// Relative approach speed for the hit.
    pub approach_speed: f32,
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
