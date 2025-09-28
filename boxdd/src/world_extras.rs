//! Optional world extensions that are not core to the safe API surface.
//!
//! Includes: explosion helpers (maps to `b2ExplosionDef` / `b2World_Explode`).

use crate::{types::Vec2, world::World};
use boxdd_sys::ffi;

/// Explosion configuration (maps to `b2ExplosionDef`).
#[derive(Copy, Clone, Debug)]
pub struct ExplosionDef(pub(crate) ffi::b2ExplosionDef);

impl Default for ExplosionDef {
    fn default() -> Self {
        let def = unsafe { ffi::b2DefaultExplosionDef() };
        Self(def)
    }
}

impl ExplosionDef {
    /// Create a default explosion definition.
    pub fn new() -> Self {
        Self::default()
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
    /// Falloff distance controlling impulse attenuation.
    pub fn falloff(mut self, f: f32) -> Self {
        self.0.falloff = f;
        self
    }
    /// Impulse per unit length applied to facing perimeters (N·s/m).
    pub fn impulse_per_length(mut self, v: f32) -> Self {
        self.0.impulsePerLength = v;
        self
    }
}

/// Extension trait adding world explosion support.
pub trait WorldExplosionExt {
    /// Trigger an explosion in the world using the provided definition.
    fn explode(&mut self, def: &ExplosionDef);
}

impl WorldExplosionExt for World {
    fn explode(&mut self, def: &ExplosionDef) {
        unsafe { ffi::b2World_Explode(self.raw(), &def.0) }
    }
}
