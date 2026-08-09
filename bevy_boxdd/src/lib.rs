//! Bevy integration for the `boxdd` Box2D bindings.
//!
//! The core `boxdd` crate stays engine-agnostic. This crate owns the Bevy-specific
//! plugin, ECS components, resources, systems, and examples.
//!
//! Bevy [`bevy_transform::components::Transform`] translations are local to
//! [`BoxddWorldOrigin`]. Absolute world-space APIs use [`boxdd::Position`] and
//! require an explicit checked conversion at the boundary.

#[cfg(doc)]
#[doc(hidden)]
#[doc = include_str!("../MIGRATION.md")]
pub mod migration_0_5_to_0_6_doctests {}

pub mod components;
pub mod errors;
pub mod math;
pub mod messages;
pub mod origin;
pub mod plugin;
pub mod prelude;
pub mod resources;
mod systems;

pub use boxdd;
pub use components::*;
pub use math::*;
pub use messages::*;
pub use origin::*;
pub use plugin::*;
pub use resources::*;
