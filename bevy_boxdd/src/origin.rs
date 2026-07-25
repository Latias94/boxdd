//! Explicit conversion between Bevy-local coordinates and Box2D world positions.

use crate::math::{to_bevy_rotation, to_boxdd_angle, to_boxdd_vec2};
use bevy_ecs::prelude::Resource;
use bevy_math::Vec2 as BevyVec2;
use bevy_transform::components::Transform as BevyTransform;
use boxdd::{Position, WorldTransform};

/// Failure while converting coordinates or rebasing the Bevy-local frame.
#[derive(Copy, Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoxddWorldOriginError {
    /// The configured world origin contains NaN or infinity.
    #[error("world origin must be finite")]
    InvalidOrigin,
    /// A Bevy-local position contains NaN or infinity.
    #[error("local position must be finite")]
    InvalidLocalPosition,
    /// An absolute Box2D position contains NaN or infinity.
    #[error("absolute world position must be finite")]
    InvalidAbsolutePosition,
    /// The absolute result cannot be represented by the active Box2D precision mode.
    #[error("absolute world position exceeds the active precision range")]
    AbsolutePositionOutOfRange,
    /// The position relative to the active origin cannot be represented by Bevy's `f32` vector.
    #[error("world position is outside the local f32 range")]
    LocalPositionOutOfRange,
    /// A 2D rotation contains NaN or infinity.
    #[error("rotation must be finite")]
    InvalidRotation,
    /// The monotonic world-origin revision counter is exhausted.
    #[error("world-origin revision counter is exhausted")]
    RevisionExhausted,
}

/// Active absolute origin for Bevy-local physics coordinates.
///
/// Fields are private so callers cannot change the coordinate frame without the
/// transactional rebase performed by [`crate::BoxddPhysicsPlugin`].
#[derive(Resource, Copy, Clone, Debug)]
pub struct BoxddWorldOrigin {
    active: Position,
    revision: u64,
    pending: Option<Position>,
}

impl BoxddWorldOrigin {
    /// Creates a coordinate frame at an absolute Box2D world position.
    pub fn try_new(active: Position) -> Result<Self, BoxddWorldOriginError> {
        if !active.is_valid() {
            return Err(BoxddWorldOriginError::InvalidOrigin);
        }

        Ok(Self {
            active,
            revision: 0,
            pending: None,
        })
    }

    /// Returns the active absolute origin.
    pub const fn active(&self) -> Position {
        self.active
    }

    /// Returns the revision committed by the latest successful rebase.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the requested origin while a rebase is pending.
    pub const fn pending(&self) -> Option<Position> {
        self.pending
    }

    /// Requests an atomic rebase during the next fixed update.
    ///
    /// Replacing an existing request is allowed. Requesting the active origin
    /// cancels any pending request because no coordinate change is necessary.
    pub fn request_rebase(&mut self, target: Position) -> Result<(), BoxddWorldOriginError> {
        if !target.is_valid() {
            return Err(BoxddWorldOriginError::InvalidOrigin);
        }

        self.pending = (target != self.active).then_some(target);
        Ok(())
    }

    /// Cancels a pending rebase without changing the active coordinate frame.
    pub fn cancel_pending_rebase(&mut self) {
        self.pending = None;
    }

    /// Converts a Bevy-local position into an absolute Box2D world position.
    pub fn checked_local_to_absolute(
        &self,
        local: BevyVec2,
    ) -> Result<Position, BoxddWorldOriginError> {
        if !local.is_finite() {
            return Err(BoxddWorldOriginError::InvalidLocalPosition);
        }

        let absolute = self.active.offset(to_boxdd_vec2(local));
        if !absolute.is_valid() {
            return Err(BoxddWorldOriginError::AbsolutePositionOutOfRange);
        }
        Ok(absolute)
    }

    /// Converts an absolute Box2D world position into the active Bevy-local frame.
    pub fn checked_absolute_to_local(
        &self,
        absolute: Position,
    ) -> Result<BevyVec2, BoxddWorldOriginError> {
        if !absolute.is_valid() {
            return Err(BoxddWorldOriginError::InvalidAbsolutePosition);
        }

        absolute
            .checked_relative_to(self.active)
            .map(|local| BevyVec2::new(local.x, local.y))
            .map_err(|_| BoxddWorldOriginError::LocalPositionOutOfRange)
    }

    /// Converts a Bevy-local transform into an absolute Box2D world transform.
    pub fn checked_local_transform_to_world(
        &self,
        local: BevyTransform,
    ) -> Result<WorldTransform, BoxddWorldOriginError> {
        if !local.rotation.is_finite() {
            return Err(BoxddWorldOriginError::InvalidRotation);
        }
        let angle = to_boxdd_angle(local.rotation);
        if !angle.is_finite() {
            return Err(BoxddWorldOriginError::InvalidRotation);
        }

        let position = self.checked_local_to_absolute(local.translation.truncate())?;
        Ok(WorldTransform::from_pos_angle(position, angle))
    }

    /// Applies an absolute Box2D transform in the active Bevy-local frame.
    ///
    /// Translation Z and scale are preserved.
    pub fn checked_apply_world_transform(
        &self,
        target: &mut BevyTransform,
        world: WorldTransform,
    ) -> Result<(), BoxddWorldOriginError> {
        let rotation = world.rotation();
        if !rotation.is_valid() {
            return Err(BoxddWorldOriginError::InvalidRotation);
        }
        let local = self.checked_absolute_to_local(world.position())?;

        target.translation.x = local.x;
        target.translation.y = local.y;
        target.rotation = to_bevy_rotation(rotation);
        Ok(())
    }

    pub(crate) fn next_revision(&self) -> Result<u64, BoxddWorldOriginError> {
        self.revision
            .checked_add(1)
            .ok_or(BoxddWorldOriginError::RevisionExhausted)
    }

    pub(crate) fn commit_rebase(&mut self, target: Position, revision: u64) {
        self.active = target;
        self.revision = revision;
        self.pending = None;
    }
}

impl Default for BoxddWorldOrigin {
    fn default() -> Self {
        Self {
            active: Position::ZERO,
            revision: 0,
            pending: None,
        }
    }
}
