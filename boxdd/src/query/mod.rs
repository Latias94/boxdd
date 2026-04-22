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

mod checked;
mod raw;
mod types;
mod world_api;

pub use types::{
    Aabb, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult, QueryFilter, RayResult,
    clip_vector, solve_planes, try_clip_vector, try_solve_planes,
};
