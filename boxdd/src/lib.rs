#![allow(rustdoc::broken_intra_doc_links)]
//! boxdd: Safe, ergonomic Rust bindings for Box2D (v3 C API)
//!
//! Highlights
//! - Thin safe layer on top of the official Box2D v3 C API.
//! - Modular API: world, bodies, shapes, joints, queries, collision geometry, events, debug draw.
//! - Ergonomics: builder patterns, world-space helpers, optional `mint` integration.
//! - Hot-path friendly APIs: keep the convenience `Vec`-returning methods, or reuse caller-owned buffers with `*_into`.
//! - Character mover helpers: cast movers, collect collision planes, solve planes, and clip velocity without raw FFI.
//! - Standalone collision geometry helpers: shape proxies, GJK distance, shape cast, TOI, and AABB validation/ray cast.
//! - Shape geometry uses crate-owned values (`Circle`, `Segment`, `Capsule`, `Polygon`) across helpers, shape editing, and creation.
//! - Core value types such as `ShapeType`, `MassData`, and contact manifolds are crate-owned instead of leaking raw Box2D structs.
//! - Typed material mixing callbacks for friction and restitution using `user_material_id`.
//! - Three usage styles:
//!   - Owned handles: `OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain` (Drop destroys; easy to store).
//!   - Scoped handles: `Body<'_>`/`Shape<'_>`/`Joint<'_>`/`Chain<'_>` (dropping only releases the world borrow).
//!   - ID-style: raw ids (`BodyId`/`ShapeId`/`JointId`/`ChainId`) for maximum flexibility.
//! - Safe handle methods validate ids and panic on invalid ids (prevents UB if an id becomes stale).
//!   For recoverable failures (invalid ids / calling during Box2D callbacks), use `try_*` APIs returning `ApiResult<T>`.
//! - Threading: `World` and owned handles are `!Send`/`!Sync`. Run physics on one thread; in async runtimes prefer
//!   `spawn_local`/`LocalSet`, or create the world inside a dedicated physics thread and communicate via channels.
//!
//! Quickstart (owned handles)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
//! let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
//! let mut world = World::new(def).unwrap();
//! let body = world.create_body_owned(BodyBuilder::new().position([0.0, 2.0]).build());
//! let sdef = ShapeDef::builder().density(1.0).build();
//! let poly = shapes::box_polygon(0.5, 0.5);
//! let _shape = world.create_polygon_shape_for_owned(body.id(), &sdef, &poly);
//! world.step(1.0/60.0, 4);
//! ```
//!
//! Quickstart (scoped handles)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
//! let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
//! let mut world = World::new(def).unwrap();
//! {
//!     // Limit the borrow of `world` by scoping the body handle.
//!     let mut body = world.create_body(BodyBuilder::new().position([0.0, 2.0]).build());
//!     let sdef = ShapeDef::builder().density(1.0).build();
//!     let poly = shapes::box_polygon(0.5, 0.5);
//!     let _shape = body.create_polygon_shape(&sdef, &poly);
//! }
//! world.step(1.0/60.0, 4);
//! ```
//!
//! Quickstart (ID-style)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
//! let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
//! let mut world = World::new(def).unwrap();
//! let body_id = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
//! let sdef = ShapeDef::builder().density(1.0).build();
//! let poly = shapes::box_polygon(0.5, 0.5);
//! let _shape_id = world.create_polygon_shape_for(body_id, &sdef, &poly);
//! world.step(1.0/60.0, 4);
//! ```
//!
//! Math interop (optional features)
//! - `Vec2` always accepts `[f32; 2]` and `(f32, f32)` anywhere `Into<Vec2>` is used.
//! - With `mint`, `cgmath`, `nalgebra`, or `glam` enabled, `Vec2` also accepts those crates'
//!   2D vector/point types via `From`/`Into`.
//! - Returned vectors can be converted back using `From` to the corresponding math types.
//!
//! Modules
//! - `world`, `body`, `shapes`, `joints`, `query`, `collision`, `events`, `debug_draw`, `prelude`.
//!   Import `boxdd::prelude::*` for the most common types.
//!
//! Queries (AABB + Ray Cast)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, Aabb, QueryFilter};
//! let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
//! let b = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
//! let sdef = ShapeDef::builder().density(1.0).build();
//! world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
//! // AABB overlap
//! let hits = world.overlap_aabb(Aabb::from_center_half_extents([0.0, 1.0], [1.0, 1.5]), QueryFilter::default());
//! assert!(!hits.is_empty());
//! let mut reused = Vec::new();
//! world.overlap_aabb_into(
//!     Aabb::from_center_half_extents([0.0, 1.0], [1.0, 1.5]),
//!     QueryFilter::default(),
//!     &mut reused,
//! );
//! assert_eq!(hits.len(), reused.len());
//! // Ray (closest)
//! let r = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
//! if r.hit { let _ = (r.point, r.normal, r.fraction); }
//! ```
//!
//! Character Mover Helpers
//! ```no_run
//! use boxdd::{clip_vector, solve_planes, CollisionPlane, QueryFilter, Vec2, World, WorldDef};
//! let world = World::new(WorldDef::default()).unwrap();
//! let planes = world.collide_mover([0.0_f32, 0.75], [0.0, 1.75], 0.25, QueryFilter::default());
//! let mut rigid: Vec<CollisionPlane> = planes
//!     .into_iter()
//!     .filter_map(|p| p.into_rigid_collision_plane())
//!     .collect();
//! let solved = solve_planes([0.0_f32, -0.1], &mut rigid);
//! let _clipped_velocity = clip_vector(Vec2::new(0.0, -1.0), &rigid);
//! let _ = solved.translation;
//! ```
//!
//! Collision Geometry
//! ```no_run
//! use boxdd::{
//!     shape_distance, DistanceInput, ShapeProxy, SimplexCache, ToiInput, ToiState, Sweep,
//!     Transform,
//! };
//! let proxy_a = ShapeProxy::new([[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]], 0.0).unwrap();
//! let proxy_b = ShapeProxy::new([[2.0_f32, -1.0], [2.0, 1.0]], 0.0).unwrap();
//! let mut cache = SimplexCache::default();
//! let distance = shape_distance(
//!     DistanceInput::new(proxy_a, proxy_b, Transform::IDENTITY, Transform::IDENTITY),
//!     &mut cache,
//! );
//! assert!(distance.distance >= 0.0);
//! let toi = boxdd::time_of_impact(ToiInput::new(
//!     proxy_a,
//!     proxy_b,
//!     Sweep::new([0.0_f32, 0.0], [0.0, 0.0], [0.0, 0.0], boxdd::Rot::IDENTITY, boxdd::Rot::IDENTITY),
//!     Sweep::new([0.0_f32, 0.0], [0.0, 0.0], [-2.0, 0.0], boxdd::Rot::IDENTITY, boxdd::Rot::IDENTITY),
//! ));
//! let _ = matches!(toi.state, ToiState::Hit | ToiState::Separated | ToiState::Overlapped | ToiState::Failed | ToiState::Unknown);
//! ```
//!
//! Material Mixing Callbacks
//! ```no_run
//! use boxdd::{MaterialMixInput, World, WorldDef};
//! let mut world = World::new(WorldDef::default()).unwrap();
//! world.set_friction_callback(|a: MaterialMixInput, b: MaterialMixInput| {
//!     if a.user_material_id == 1 || b.user_material_id == 1 {
//!         0.0
//!     } else {
//!         (a.coefficient * b.coefficient).sqrt()
//!     }
//! });
//! ```
//!
//! Feature Flags
//! - `serialize`: scene snapshot helpers (save/apply world config; build/restore minimal full-scene snapshot).
//! - `pkg-config`: allow linking against a system `box2d` via pkg-config.
//! - `mint`: lightweight math interop types (`mint::Vector2`, `mint::Point2`, and 2D affine matrices for `Transform`).
//! - `cgmath` / `nalgebra` / `glam`: conversions with their 2D math types.
//! - `bytemuck`: `Pod`/`Zeroable` for core math types (`Vec2`, `Rot`, `Transform`, `Aabb`) for zero-copy interop.
//!
//! Events
//! - Three access styles:
//!   - By value: `world.contact_events()`/`sensor_events()`/`body_events()`/`joint_events()` return owned data for storage or cross‑frame use.
//!   - Zero‑copy views: `with_*_events_view(...)` iterate without allocations (borrows internal buffers).
//!   - Raw slices: `unsafe { with_*_events(...) }` expose FFI slices (borrows internal buffers).
//!
//! Example (zero‑copy views)
//! ```no_run
//! use boxdd::prelude::*;
//! let mut world = World::new(WorldDef::default()).unwrap();
//! world.with_contact_events_view(|begin, end, hit| {
//!     let _ = (begin.count(), end.count(), hit.count());
//! });
//! world.with_sensor_events_view(|beg, end| { let _ = (beg.count(), end.count()); });
//! world.with_body_events_view(|moves| { for m in moves { let _ = (m.body_id(), m.fell_asleep()); } });
//! world.with_joint_events_view(|j| { let _ = j.count(); });
//! ```

pub mod body;
pub mod collision;
pub mod debug_draw;
pub mod error;
pub mod events;
pub mod filter;
pub mod joints;
pub mod prelude;
pub mod query;
#[cfg(feature = "serialize")]
#[cfg_attr(docsrs, doc(cfg(feature = "serialize")))]
pub mod serialize;
pub mod shapes;
pub mod tuning;
pub mod types;
#[cfg(feature = "unchecked")]
#[cfg_attr(docsrs, doc(cfg(feature = "unchecked")))]
pub mod unchecked;
pub mod world;
pub mod world_extras;
pub mod core {
    pub(crate) mod box2d_lock;
    pub(crate) mod callback_state;
    pub(crate) mod debug_checks;
    pub(crate) mod ffi_vec;
    pub(crate) mod material_mix_registry;
    pub mod math;
    #[cfg(feature = "serialize")]
    pub(crate) mod serialize_registry;
    pub(crate) mod user_data;
    pub(crate) mod world_core;
}

pub use body::OwnedBody;
pub use body::{Body, BodyBuilder, BodyDef, BodyType};
pub use collision::{
    CastOutput, DistanceInput, DistanceOutput, MAX_SHAPE_PROXY_POINTS, SegmentDistanceResult,
    ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput, ToiOutput, ToiState,
    segment_distance, shape_cast, shape_distance, time_of_impact,
};
#[cfg(feature = "cgmath")]
#[cfg_attr(docsrs, doc(cfg(feature = "cgmath")))]
pub use core::math::TransformFromCgmathError;
#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
pub use core::math::TransformFromGlamError;
#[cfg(feature = "mint")]
#[cfg_attr(docsrs, doc(cfg(feature = "mint")))]
pub use core::math::TransformFromMintError;
pub use core::math::{Rot, Transform};
pub use debug_draw::{DebugDraw, DebugDrawCmd, DebugDrawOptions};
pub use error::{ApiError, ApiResult};
pub use events::{
    BodyMoveEvent, ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent,
    JointEvent, SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents,
};
pub use filter::Filter;
pub use joints::{
    DistanceJointBuilder, DistanceJointDef, FilterJointBuilder, FilterJointDef, Joint, JointBase,
    JointBaseBuilder, MotorJointBuilder, MotorJointDef, PrismaticJointBuilder, PrismaticJointDef,
    RevoluteJointBuilder, RevoluteJointDef, WeldJointBuilder, WeldJointDef, WheelJointBuilder,
    WheelJointDef,
};
pub use query::{
    Aabb, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult, QueryFilter, RayResult,
    clip_vector, solve_planes,
};
pub use shapes::chain::{Chain, ChainDef, ChainDefBuilder, OwnedChain};
pub use shapes::{
    Capsule, Circle, MAX_POLYGON_VERTICES, OwnedShape, Polygon, Segment, Shape, ShapeDef,
    ShapeDefBuilder, ShapeType, SurfaceMaterial,
};
pub use types::{
    BodyId, ChainId, ContactData, ContactId, JointId, Manifold, ManifoldPoint, MassData, ShapeId,
    Vec2,
};
pub use world::{
    CallbackWorld, MaterialMixInput, OutstandingOwnedHandles, OwnedHandleCounts, World,
    WorldBuilder, WorldDef, WorldHandle,
};
