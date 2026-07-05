//! Bevy math adapters for `boxdd` value types.
//!
//! Box2D is two-dimensional. These helpers map Bevy `Transform` values to XY
//! translation plus Z-axis rotation and preserve Bevy Z/scale when applying
//! physics transforms back to render entities.

use bevy_math::{EulerRot, Quat as BevyQuat, Vec2 as BevyVec2, Vec3 as BevyVec3};
use bevy_transform::components::Transform as BevyTransform;

/// Converts a Bevy `Vec2` to a Box2D vector.
#[inline]
pub fn to_boxdd_vec2(value: BevyVec2) -> boxdd::Vec2 {
    boxdd::Vec2::new(value.x, value.y)
}

/// Converts a Bevy `Vec3` translation to a Box2D vector by taking XY.
#[inline]
pub fn to_boxdd_translation(value: BevyVec3) -> boxdd::Vec2 {
    boxdd::Vec2::new(value.x, value.y)
}

/// Extracts the Z-axis rotation angle from a Bevy quaternion.
#[inline]
pub fn to_boxdd_angle(value: BevyQuat) -> f32 {
    value.to_euler(EulerRot::XYZ).2
}

/// Converts a Bevy transform to a Box2D transform, ignoring Z and scale.
#[inline]
pub fn to_boxdd_transform(value: BevyTransform) -> boxdd::Transform {
    boxdd::Transform::from_pos_angle(
        to_boxdd_translation(value.translation),
        to_boxdd_angle(value.rotation),
    )
}

/// Converts a Box2D vector to a Bevy `Vec2`.
#[inline]
pub fn to_bevy_vec2(value: boxdd::Vec2) -> BevyVec2 {
    BevyVec2::new(value.x, value.y)
}

/// Converts a Box2D vector to a Bevy `Vec3` with the supplied Z coordinate.
#[inline]
pub fn to_bevy_translation(value: boxdd::Vec2, z: f32) -> BevyVec3 {
    BevyVec3::new(value.x, value.y, z)
}

/// Converts a Box2D rotation to a Bevy Z-axis quaternion.
#[inline]
pub fn to_bevy_rotation(value: boxdd::Rot) -> BevyQuat {
    BevyQuat::from_rotation_z(value.angle())
}

/// Converts a Box2D transform to a Bevy transform with Z = 0 and unit scale.
#[inline]
pub fn to_bevy_transform(value: boxdd::Transform) -> BevyTransform {
    BevyTransform::from_translation(to_bevy_translation(value.position(), 0.0))
        .with_rotation(to_bevy_rotation(value.rotation()))
}

/// Applies a Box2D transform to an existing Bevy transform, preserving Z and scale.
#[inline]
pub fn apply_boxdd_transform(target: &mut BevyTransform, value: boxdd::Transform) {
    let z = target.translation.z;
    target.translation = to_bevy_translation(value.position(), z);
    target.rotation = to_bevy_rotation(value.rotation());
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

/// Extension methods for converting Bevy transforms to Box2D transforms.
pub trait BevyTransformBoxddExt {
    /// Converts translation XY and rotation Z to a Box2D transform.
    fn to_boxdd_transform(self) -> boxdd::Transform;
}

impl BevyTransformBoxddExt for BevyTransform {
    #[inline]
    fn to_boxdd_transform(self) -> boxdd::Transform {
        to_boxdd_transform(self)
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

/// Extension methods for converting Box2D transforms to Bevy transforms.
pub trait BoxddTransformBevyExt {
    /// Converts this value to a Bevy transform with unit scale.
    fn to_bevy_transform(self) -> BevyTransform;

    /// Applies translation and rotation to an existing Bevy transform.
    fn apply_to_bevy_transform(self, target: &mut BevyTransform);
}

impl BoxddTransformBevyExt for boxdd::Transform {
    #[inline]
    fn to_bevy_transform(self) -> BevyTransform {
        to_bevy_transform(self)
    }

    #[inline]
    fn apply_to_bevy_transform(self, target: &mut BevyTransform) {
        apply_boxdd_transform(target, self);
    }
}
