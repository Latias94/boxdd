//! Additional world runtime helpers and value types that sit beside the core world API.

use crate::{error::ApiResult, types::Vec2, world::World};
use boxdd_sys::ffi;

/// Explosion configuration (maps to `b2ExplosionDef`).
#[derive(Copy, Clone, Debug)]
pub struct ExplosionDef(pub(crate) ffi::b2ExplosionDef);

impl Default for ExplosionDef {
    fn default() -> Self {
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
    pub fn mask_bits(mut self, bits: u64) -> Self {
        self.0.maskBits = bits;
        self
    }

    /// World-space center position.
    pub fn position<V: Into<Vec2>>(mut self, p: V) -> Self {
        self.0.position = p.into().into();
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
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_Explode(self.raw(), &def.0) }
    }

    pub fn try_explode(&mut self, def: &ExplosionDef) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        unsafe { ffi::b2World_Explode(self.raw(), &def.0) }
        Ok(())
    }
}
