//! Broad-phase queries, casts, and character-mover helpers.
//!
//! - AABB and shape overlap: collect matching shape ids, reuse caller-owned buffers, or visit hits without a result container.
//! - Ray casts: closest or all hits along a path.
//! - Shape overlap / casting: build a temporary proxy from points + radius (accepts `Into<Vec2>` points).
//! - Offset proxies: apply translation + rotation to the proxy for queries in local frames.
//! - Character mover helpers: cast a capsule mover, collect collision planes, solve planes, and clip velocity.
//!
//! Note: Box2D proxies support at most `B2_MAX_POLYGON_VERTICES` points (8). Extra points are ignored.
//!
//! Filters: use `QueryFilter` to restrict categories/masks.

#[cfg(test)]
mod availability_tests;
mod checked;
mod raw;
mod types;
mod world_api;

#[derive(Clone)]
pub(crate) struct QueryTarget {
    core: std::rc::Rc<crate::core::world_core::WorldCore>,
    access: crate::core::world_core::WorldAccess,
}

impl QueryTarget {
    pub(crate) fn new(core: std::rc::Rc<crate::core::world_core::WorldCore>) -> Self {
        Self::with_access(core, crate::core::world_core::WorldAccess::Idle)
    }

    pub(crate) fn recording(core: std::rc::Rc<crate::core::world_core::WorldCore>) -> Self {
        Self::with_access(core, crate::core::world_core::WorldAccess::Recording)
    }

    fn with_access(
        core: std::rc::Rc<crate::core::world_core::WorldCore>,
        access: crate::core::world_core::WorldAccess,
    ) -> Self {
        debug_assert_eq!(core.id.index1.checked_sub(1), Some(core.brand().world0()));
        debug_assert_eq!(core.id.generation, core.brand().world_generation());
        Self { core, access }
    }

    #[inline]
    pub(crate) fn raw(&self) -> boxdd_sys::ffi::b2WorldId {
        self.core.id
    }

    #[inline]
    pub(crate) fn brand(&self) -> crate::id::IdBrand {
        self.core.brand()
    }

    #[inline]
    pub(crate) fn check_available(&self) -> crate::error::ApiResult<()> {
        self.core.check_access(self.access)
    }

    pub(crate) fn begin_native_call(
        &self,
    ) -> crate::error::ApiResult<crate::core::world_core::NativeCallGuard> {
        self.core.begin_native_call_with_access(self.access)
    }

    pub(crate) fn core_rc(&self) -> std::rc::Rc<crate::core::world_core::WorldCore> {
        std::rc::Rc::clone(&self.core)
    }
}

pub(crate) use checked::*;

pub use types::{
    Aabb, ClosestRayCastResult, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult,
    QueryFilter, RayResult, clip_vector, solve_planes, try_clip_vector, try_solve_planes,
};
