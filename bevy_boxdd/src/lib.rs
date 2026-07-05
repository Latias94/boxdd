//! Bevy integration for the `boxdd` Box2D bindings.
//!
//! The core `boxdd` crate stays engine-agnostic. This crate owns the Bevy-specific
//! plugin, ECS components, resources, systems, and examples.

pub mod components;
pub mod errors;
pub mod math;
pub mod messages;
pub mod plugin;
pub mod prelude;
pub mod resources;
pub mod systems;

pub use boxdd;
pub use components::*;
pub use math::*;
pub use messages::*;
pub use plugin::BoxddPhysicsPlugin;
pub use resources::*;
