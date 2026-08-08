//! Borrow-scoped broad-phase queries, casts, and character-mover helpers.
//!
//! Acquire [`Query`] from [`crate::World::query`] or [`crate::RecordingSession::query`]. Both
//! owners issue the same capability type; the sealed access proof carries the permitted activity.
//! On native targets, repeated overlap, cast, and mover queries can reuse `ShapeQueryBuffer`,
//! `RayQueryBuffer`, and `MoverQueryBuffer` so native and mapped storage remain warm together.
//!
//! Temporary proxies accept at most [`crate::MAX_SHAPE_PROXY_POINTS`] points; exceeding that limit
//! returns [`crate::Error::InvalidArgument`] before native query activity. Visitor methods bind the
//! complete native result batch first, then stop Rust-side iteration when the visitor returns
//! `false`.

#[cfg(test)]
mod availability_tests;
mod buffers;
mod capability;
mod raw;
mod types;

#[cfg(not(target_arch = "wasm32"))]
pub use buffers::{MoverQueryBuffer, RayQueryBuffer, ShapeQueryBuffer};
pub use capability::Query;
pub use types::{
    Aabb, ClosestRayCastResult, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult,
    QueryFilter, RayResult, clip_vector, solve_planes,
};
