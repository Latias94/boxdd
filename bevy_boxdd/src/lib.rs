//! Bevy integration for the `boxdd` Box2D bindings.
//!
//! The core `boxdd` crate stays engine-agnostic. This crate owns the Bevy-specific
//! plugin, ECS components, resources, systems, and examples.
//!
//! Bevy [`bevy_transform::components::Transform`] translations are local to
//! [`BoxddWorldOrigin`]. Absolute world-space APIs use [`boxdd::Position`] and
//! require an explicit checked conversion at the boundary.

pub mod components;
pub mod errors;
pub mod math;
pub mod messages;
pub mod origin;
pub mod plugin;
pub mod prelude;
pub mod resources;
pub mod systems;

pub use boxdd;
pub use components::*;
pub use math::*;
pub use messages::*;
pub use origin::*;
pub use plugin::BoxddPhysicsPlugin;
pub use resources::*;
