//! Additional world runtime helpers and value types that sit beside the core world API.

use crate::{error::ApiResult, types::Position, world::World};
use boxdd_sys::ffi;

#[inline]
pub(crate) fn explosion_query_axis_is_representable(position: f64, extent: f32) -> bool {
    let extent = f64::from(extent);
    let lower = position - extent;
    let upper = position + extent;
    let limit = f64::from(f32::MAX);

    lower.is_finite()
        && upper.is_finite()
        && (-limit..=limit).contains(&lower)
        && (-limit..=limit).contains(&upper)
}

#[inline]
fn check_explosion_def_valid(def: &ExplosionDef) -> ApiResult<()> {
    let position = def.center();
    let radius = def.blast_radius();
    let falloff = def.falloff_distance();
    let impulse = def.impulse_per_unit_length();
    let extent = radius + falloff;
    #[cfg(not(feature = "double-precision"))]
    let position_x = f64::from(position.x);
    #[cfg(feature = "double-precision")]
    let position_x = position.x;
    #[cfg(not(feature = "double-precision"))]
    let position_y = f64::from(position.y);
    #[cfg(feature = "double-precision")]
    let position_y = position.y;

    if position.is_valid()
        && radius.is_finite()
        && radius >= 0.0
        && falloff.is_finite()
        && falloff >= 0.0
        && impulse.is_finite()
        && extent.is_finite()
        && explosion_query_axis_is_representable(position_x, extent)
        && explosion_query_axis_is_representable(position_y, extent)
    {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn try_world_explode_with_access(
    world: &World,
    def: &ExplosionDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    check_explosion_def_valid(def)?;
    crate::core::callback_state::check_not_in_callback()?;
    world.core().check_access(access)?;
    unsafe { ffi::b2World_Explode(world.raw(), &def.0) };
    Ok(())
}

/// Explosion configuration (maps to `b2ExplosionDef`).
#[derive(Copy, Clone, Debug)]
pub struct ExplosionDef(pub(crate) ffi::b2ExplosionDef);

impl Default for ExplosionDef {
    fn default() -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        Self(unsafe { ffi::b2DefaultExplosionDef() })
    }
}

impl ExplosionDef {
    /// Create a default explosion definition.
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2ExplosionDef) -> Self {
        Self(raw)
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2ExplosionDef {
        self.0
    }

    /// Mask bits used to filter affected shapes.
    pub fn affected_mask_bits(&self) -> u64 {
        self.0.maskBits
    }

    /// World-space center position.
    pub fn center(&self) -> Position {
        Position::from_raw(self.0.position)
    }

    /// Explosion radius in meters.
    pub fn blast_radius(&self) -> f32 {
        self.0.radius
    }

    /// Falloff distance beyond the radius where the impulse decays to zero.
    pub fn falloff_distance(&self) -> f32 {
        self.0.falloff
    }

    /// Impulse per unit length applied to perimeter facing the explosion.
    pub fn impulse_per_unit_length(&self) -> f32 {
        self.0.impulsePerLength
    }

    /// Mask bits used to filter affected shapes.
    pub fn mask_bits(mut self, bits: u64) -> Self {
        self.0.maskBits = bits;
        self
    }

    /// World-space center position.
    pub fn position<P: Into<Position>>(mut self, p: P) -> Self {
        self.0.position = p.into().into_raw();
        self
    }

    /// Explosion radius in meters.
    pub fn radius(mut self, r: f32) -> Self {
        self.0.radius = r;
        self
    }

    /// Falloff distance beyond the radius where the impulse decays to zero.
    pub fn falloff(mut self, f: f32) -> Self {
        self.0.falloff = f;
        self
    }

    /// Impulse per unit length applied to perimeter facing the explosion.
    pub fn impulse_per_length(mut self, v: f32) -> Self {
        self.0.impulsePerLength = v;
        self
    }
}

impl World {
    /// Trigger an explosion in the world using the provided definition.
    pub fn explode(&mut self, def: &ExplosionDef) {
        self.try_explode(def)
            .expect("explosion definition and world access must be valid")
    }

    /// Trigger an explosion after validating the definition and world access.
    pub fn try_explode(&mut self, def: &ExplosionDef) -> ApiResult<()> {
        try_world_explode_with_access(self, def, crate::core::world_core::WorldAccess::Idle)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn invalid_definition_is_rejected_before_callback_access_checks() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let invalid = ExplosionDef::new().radius(f32::NAN);
        let _guard = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world.try_explode(&invalid),
            Err(crate::ApiError::InvalidArgument)
        );
    }
}
