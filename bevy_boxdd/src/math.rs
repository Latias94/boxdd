//! Bevy adapters for local vectors and rotations.
//!
//! Absolute [`boxdd::Position`] and [`boxdd::WorldTransform`] conversions require
//! an explicit [`crate::BoxddWorldOrigin`] and intentionally do not live here.

use bevy_math::{EulerRot, Quat as BevyQuat, Vec2 as BevyVec2};

/// Converts a Bevy `Vec2` to a Box2D vector.
#[inline]
pub fn to_boxdd_vec2(value: BevyVec2) -> boxdd::Vec2 {
    boxdd::Vec2::new(value.x, value.y)
}

/// Extracts the Z-axis rotation angle from a Bevy quaternion.
#[inline]
pub fn to_boxdd_angle(value: BevyQuat) -> f32 {
    value.to_euler(EulerRot::XYZ).2
}

/// Converts a Box2D vector to a Bevy `Vec2`.
#[inline]
pub fn to_bevy_vec2(value: boxdd::Vec2) -> BevyVec2 {
    BevyVec2::new(value.x, value.y)
}

/// Converts a Box2D rotation to a Bevy Z-axis quaternion.
#[inline]
pub fn to_bevy_rotation(value: boxdd::Rot) -> BevyQuat {
    BevyQuat::from_rotation_z(value.angle())
}

/// Extension methods for converting Bevy 2D vectors to Box2D values.
pub trait BevyVec2BoxddExt {
    /// Converts this Bevy vector to a Box2D vector.
    fn to_boxdd_vec2(self) -> boxdd::Vec2;
}

impl BevyVec2BoxddExt for BevyVec2 {
    #[inline]
    fn to_boxdd_vec2(self) -> boxdd::Vec2 {
        to_boxdd_vec2(self)
    }
}

/// Extension methods for extracting Box2D rotations from Bevy quaternions.
pub trait BevyQuatBoxddExt {
    /// Returns the Z-axis angle in radians used by Box2D.
    fn to_boxdd_angle(self) -> f32;
}

impl BevyQuatBoxddExt for BevyQuat {
    #[inline]
    fn to_boxdd_angle(self) -> f32 {
        to_boxdd_angle(self)
    }
}

/// Extension method for converting Box2D vectors to Bevy vectors.
pub trait BoxddVec2BevyExt {
    /// Converts this value to Bevy's `Vec2`.
    fn to_bevy_vec2(self) -> BevyVec2;
}

impl BoxddVec2BevyExt for boxdd::Vec2 {
    #[inline]
    fn to_bevy_vec2(self) -> BevyVec2 {
        to_bevy_vec2(self)
    }
}

/// Extension method for converting Box2D rotations to Bevy quaternions.
pub trait BoxddQuatBevyExt {
    /// Converts this rotation to a Bevy Z-axis quaternion.
    fn to_bevy_quat(self) -> BevyQuat;
}

impl BoxddQuatBevyExt for boxdd::Rot {
    #[inline]
    fn to_bevy_quat(self) -> BevyQuat {
        to_bevy_rotation(self)
    }
}
