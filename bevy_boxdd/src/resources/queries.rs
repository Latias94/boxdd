use super::BoxddPhysicsContext;
use crate::math::to_boxdd_vec2;
use crate::messages::BoxddPluginError;
use bevy_ecs::prelude::Entity;
use bevy_math::Vec2 as BevyVec2;
#[cfg(not(target_arch = "wasm32"))]
use boxdd::{Aabb, DebugDrawCmd, DebugDrawOptions, Error as BoxddError};
use boxdd::{Position, QueryFilter, RayResult, ShapeId};

/// Ray-cast hit enriched with the Bevy entity mapped to the native shape.
#[derive(Copy, Clone, Debug)]
pub struct BoxddRayHit {
    /// Native Box2D ray result.
    pub hit: RayResult,
    /// Bevy entity mapped to `hit.shape_id`, if the shape is owned by this plugin.
    pub entity: Option<Entity>,
}

/// Closest ray-cast result enriched with Bevy entity mapping and traversal statistics.
#[derive(Copy, Clone, Debug)]
pub struct BoxddClosestRayCastResult {
    /// Closest hit with its mapped entity, or `None` when no shape was hit.
    pub hit: Option<BoxddRayHit>,
    /// Number of broad-phase tree nodes visited by Box2D.
    pub node_visits: i32,
    /// Number of broad-phase leaves visited by Box2D.
    pub leaf_visits: i32,
}

/// AABB overlap hit enriched with the Bevy entity mapped to the native shape.
#[derive(Copy, Clone, Debug)]
pub struct BoxddShapeHit {
    /// Native Box2D shape id returned by the overlap query.
    pub shape_id: ShapeId,
    /// Bevy entity mapped to `shape_id`, if the shape is owned by this plugin.
    pub entity: Option<Entity>,
}

impl BoxddPhysicsContext {
    /// Casts from an absolute world `origin` by a local `translation` and returns
    /// the closest hit with the mapped Bevy shape entity.
    pub fn cast_ray_closest_entity(
        &self,
        origin: Position,
        translation: BevyVec2,
        filter: QueryFilter,
    ) -> Result<Option<BoxddRayHit>, BoxddPluginError> {
        self.cast_ray_closest_entity_with_stats(origin, translation, filter)
            .map(|result| result.hit)
    }

    /// Casts a ray and returns its mapped closest hit together with traversal statistics.
    ///
    /// Statistics remain available when the ray misses every shape.
    pub fn cast_ray_closest_entity_with_stats(
        &self,
        origin: Position,
        translation: BevyVec2,
        filter: QueryFilter,
    ) -> Result<BoxddClosestRayCastResult, BoxddPluginError> {
        let world = self.plugin_world()?;
        let result = world.query()?.cast_ray_closest_with_stats(
            origin,
            to_boxdd_vec2(translation),
            filter,
        )?;
        Ok(BoxddClosestRayCastResult {
            hit: result.hit.map(|hit| self.ray_hit_with_entity(hit)),
            node_visits: result.node_visits,
            leaf_visits: result.leaf_visits,
        })
    }

    /// Casts from an absolute world `origin` by a local `translation` and writes
    /// all hits with mapped Bevy shape entities into `out`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all_entities_into(
        &mut self,
        origin: Position,
        translation: BevyVec2,
        filter: QueryFilter,
        out: &mut Vec<BoxddRayHit>,
    ) -> Result<(), BoxddPluginError> {
        let reason = self.disabled_reason();
        {
            let (world, ray_hits) = (&self.world, &mut self.ray_hits);
            let world = world.as_ref().ok_or(BoxddPluginError::ContextDisabled {
                reason: reason.unwrap_or(crate::BoxddContextDisabledReason::Explicit),
            })?;
            world.query()?.cast_ray_all_into(
                origin,
                to_boxdd_vec2(translation),
                filter,
                ray_hits,
            )?;
        }
        out.try_reserve(self.ray_hits.len().saturating_sub(out.len()))
            .map_err(|_| BoxddError::FfiOutputAllocationFailed)?;
        out.clear();
        out.extend(
            self.ray_hits
                .iter()
                .copied()
                .map(|hit| self.ray_hit_with_entity(hit)),
        );
        Ok(())
    }

    /// Casts from an absolute world `origin` by a local `translation` and returns
    /// all hits with mapped Bevy shape entities.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all_entities(
        &mut self,
        origin: Position,
        translation: BevyVec2,
        filter: QueryFilter,
    ) -> Result<Vec<BoxddRayHit>, BoxddPluginError> {
        let mut out = Vec::new();
        self.cast_ray_all_entities_into(origin, translation, filter, &mut out)?;
        Ok(out)
    }

    /// Queries AABB bounds local to the absolute world `origin` and writes all
    /// hits with mapped Bevy shape entities into `out`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_aabb_entities_into(
        &mut self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<BoxddShapeHit>,
    ) -> Result<(), BoxddPluginError> {
        let reason = self.disabled_reason();
        {
            let (world, shape_hits) = (&self.world, &mut self.shape_hits);
            let world = world.as_ref().ok_or(BoxddPluginError::ContextDisabled {
                reason: reason.unwrap_or(crate::BoxddContextDisabledReason::Explicit),
            })?;
            world
                .query()?
                .overlap_aabb_into(origin, aabb, filter, shape_hits)?;
        }
        out.try_reserve(self.shape_hits.len().saturating_sub(out.len()))
            .map_err(|_| BoxddError::FfiOutputAllocationFailed)?;
        out.clear();
        out.extend(
            self.shape_hits
                .iter()
                .copied()
                .map(|shape_id| self.shape_hit_with_entity(shape_id)),
        );
        Ok(())
    }

    /// Queries AABB bounds local to the absolute world `origin` and returns all
    /// hits with mapped Bevy shape entities.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlap_aabb_entities(
        &mut self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> Result<Vec<BoxddShapeHit>, BoxddPluginError> {
        let mut out = Vec::new();
        self.overlap_aabb_entities_into(origin, aabb, filter, &mut out)?;
        Ok(out)
    }

    /// Collects Box2D debug-draw commands into a caller-owned buffer.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn debug_draw_collect_into(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        options: DebugDrawOptions,
    ) -> Result<(), BoxddPluginError> {
        self.plugin_world_mut()?
            .debug_draw_collect_into(out, options)?;
        Ok(())
    }

    /// Collects Box2D debug-draw commands into a new vector.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn debug_draw_collect(
        &mut self,
        options: DebugDrawOptions,
    ) -> Result<Vec<DebugDrawCmd>, BoxddPluginError> {
        let mut out = Vec::new();
        self.debug_draw_collect_into(&mut out, options)?;
        Ok(out)
    }

    fn ray_hit_with_entity(&self, hit: RayResult) -> BoxddRayHit {
        BoxddRayHit {
            hit,
            entity: self.shape_entity(hit.shape_id),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shape_hit_with_entity(&self, shape_id: ShapeId) -> BoxddShapeHit {
        BoxddShapeHit {
            shape_id,
            entity: self.shape_entity(shape_id),
        }
    }
}
