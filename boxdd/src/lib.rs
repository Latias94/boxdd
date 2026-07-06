#![allow(rustdoc::broken_intra_doc_links)]
//! boxdd: Safe, ergonomic Rust bindings for Box2D (v3 C API)
//!
//! Highlights
//! - Thin safe layer on top of the official Box2D v3 C API.
//! - Modular API: world, bodies, shapes, joints, queries, collision geometry, events, debug draw.
//! - Ergonomics: builder patterns, world-space helpers, and optional math interop (`mint`/`cgmath`/`nalgebra`/`glam`).
//! - Hot-path friendly APIs: keep the convenience `Vec`-returning methods, reuse caller-owned buffers with `*_into`, or use `visit_*` overlap queries to avoid result-container allocation entirely.
//! - Character mover helpers: cast movers, collect collision planes, solve planes, and clip velocity without raw FFI.
//! - Standalone collision geometry helpers: shape proxies, segment/GJK distance, manifolds, shape cast, TOI, recoverable `try_*` validation paths, AABB validation/ray cast, and deterministic global math helpers.
//! - Core math types (`Vec2`, `Rot`, `Transform`) use explicit `from_raw(...)` / `into_raw()` naming for Box2D interop instead of implicit raw conversions.
//! - Global Box2D foundation helpers expose allocated-byte inspection, timing ticks/millisecond helpers, thread yielding, and deterministic hashing without dropping to `boxdd_sys::ffi`.
//! - Shape geometry uses crate-owned values (`Circle`, `Segment`, `ChainSegment`, `Capsule`, `Polygon`) across helpers, shape editing, and creation, including square/rounded/offset/hull-based polygon builders plus standalone and construction-time `try_*` geometry helpers without raw FFI.
//! - Chain runtime material helpers use visible live-segment indexing on open chains instead of Box2D's ghost-placeholder storage layout.
//! - Safe shape/joint mutators front-load obvious Box2D assert preconditions such as non-negative material scalars and ordered joint limits.
//! - Pointer-bearing config wrappers keep their raw re-entry explicit: `BodyDef::from_raw(...)`
//!   and `WorldDef::from_raw(...)` are `unsafe`.
//! - Live shapes expose safe runtime helpers for AABB, point tests, direct ray casts, computed mass data, and runtime event toggles.
//! - Bodies expose safe runtime helpers for rotation, sleep/awake/enabled/bullet/name controls, attached shape/joint enumeration, and body-level contact/hit event toggles.
//! - Joints expose safe runtime helpers for joint kind, connected body ids, `collide_connected`, constraint tuning, local frames, wake controls, and type-specific runtime state across distance/prismatic/revolute/weld/wheel/motor families.
//! - `ContactId` values from contact events or snapshots expose direct safe inherent helpers for validity checks and crate-owned/raw contact-data reads.
//! - World runtime helpers expose counters, per-stage `Profile` timings, explosion control, and `try_*` access for callback-sensitive tuning toggles.
//! - Core value types such as `ShapeType`, `MassData`, `SurfaceMaterial`, and contact manifolds are crate-owned instead of leaking raw Box2D structs.
//! - Typed material mixing callbacks for friction and restitution using `user_material_id`.
//! - Three usage styles:
//!   - Owned handles: `OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain` (Drop destroys; easy to store).
//!   - Scoped handles: `Body<'_>`/`Shape<'_>`/`Joint<'_>`/`Chain<'_>` (dropping only releases the world borrow).
//!   - ID-style: raw ids (`BodyId`/`ShapeId`/`JointId`/`ChainId`) for maximum flexibility.
//! - Safe handle methods validate ids and panic on invalid ids (prevents UB if an id becomes stale).
//!   For recoverable failures (invalid ids / wrong typed-joint family / calling during Box2D callbacks), use `try_*` APIs returning `ApiResult<T>`.
//! - Threading: `World` and owned handles are `!Send`/`!Sync`. Run physics on one thread; in async runtimes prefer
//!   `spawn_local`/`LocalSet`, or create the world inside a dedicated physics thread and communicate via channels.
//!
//! Quickstart (owned handles)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
//! let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
//! let mut world = World::new(def).unwrap();
//! let mut body = world.create_body_owned(BodyBuilder::new().position([0.0, 2.0]).build());
//! let sdef = ShapeDef::builder().density(1.0).build();
//! let poly = shapes::box_polygon(0.5, 0.5);
//! let _shape = body.create_polygon_shape(&sdef, &poly);
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
//! - `mint` also covers `Rot <-> mint::RowMatrix2` / `mint::ColumnMatrix2`, plus row- and
//!   column-major 2D affine matrices for `Transform`.
//!
//! Modules
//! - `world`, `body`, `contact`, `shapes`, `joints`, `query`, `collision`, `events`, `debug_draw`, `prelude`.
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
//! let mut visited = 0;
//! let complete = world.visit_overlap_aabb(
//!     Aabb::from_center_half_extents([0.0, 1.0], [1.0, 1.5]),
//!     QueryFilter::default(),
//!     |_| {
//!         visited += 1;
//!         true
//!     },
//! );
//! assert!(complete);
//! assert_eq!(hits.len(), visited);
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
//!     segment_distance, shape_distance, DistanceInput, ShapeProxy, SimplexCache, ToiInput,
//!     ToiState, Sweep, Transform,
//! };
//! let proxy_a = ShapeProxy::new([[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]], 0.0).unwrap();
//! let proxy_b = ShapeProxy::new([[2.0_f32, -1.0], [2.0, 1.0]], 0.0).unwrap();
//! let mut cache = SimplexCache::default();
//! let seg = segment_distance([-1.0_f32, 0.0], [1.0, 0.0], [0.0, -1.0], [0.0, 1.0]);
//! assert!(seg.distance_squared >= 0.0);
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
//! - `mint`: lightweight math interop types (`mint::Vector2`, `mint::Point2`, `mint::RowMatrix2` /
//!   `mint::ColumnMatrix2` for `Rot`, and row/column-major 2D affine matrices for `Transform`).
//! - `cgmath` / `nalgebra` / `glam`: conversions with their 2D math types.
//! - `bytemuck`: `Pod`/`Zeroable` for core math types (`Vec2`, `Rot`, `Transform`, `Aabb`) for zero-copy interop.
//!
//! Threading and async
//! - `WorldDef::builder().worker_count(n)` preserves Box2D's worker-count setting, but actual
//!   multithreaded stepping still requires explicit raw task callbacks through
//!   `unsafe WorldBuilder::task_system_raw(...)` / `WorldDef::set_task_system_raw(...)`. It does
//!   not make `World`, `WorldHandle`, or owned handles `Send`/`Sync`.
//! - Keep the world on one thread/task. In async runtimes prefer `spawn_local` / `LocalSet`; in
//!   multi-threaded engines prefer a dedicated physics thread plus channels.
//! - `set_custom_filter*`, `set_pre_solve*`, `set_friction_callback`, and `set_restitution_callback`
//!   may run on Box2D worker threads and therefore require `Send + Sync` closures.
//! - See `examples/physics_thread.rs` for the dedicated-thread pattern.
//!
//! Error handling
//! - The default safe surface panics on misuse such as stale ids or calling Box2D while the world
//!   is locked in a callback. This keeps the common path terse and avoids Rust-level UB.
//! - At runtime boundaries, prefer `try_*` APIs and handle `ApiError` explicitly.
//! - `try_*` setters also turn obvious Box2D assert preconditions into recoverable `ApiError`
//!   values instead of relying on assert-enabled native builds.
//! - `WorldDef`, `BodyDef`, `ShapeDef`, `SurfaceMaterial`, `JointBase`, and concrete
//!   `*JointDef` values expose `validate()` for preflight checks before crossing the FFI boundary.
//! - Crate-owned geometry values (`Circle`, `Segment`, `Capsule`, `ChainSegment`, `Polygon`) also
//!   expose `is_valid()` / `validate()` for preflight geometry checks, and the world-free helper
//!   methods (`mass_data`, `aabb`, `contains_point`, `ray_cast`, `transformed`) follow the same
//!   panic-by-default plus recoverable `try_*` split as the rest of the crate.
//!
//! Events
//! - Four access styles:
//!   - By value: `World` and `WorldHandle` expose `contact_events()`/`sensor_events()`/`body_events()`/`joint_events()` for owned data that can be stored or used cross-frame.
//!   - Reusable buffers: `World` and `WorldHandle` also expose `*_events_into(...)` to reuse caller-owned event storage across frames.
//!   - Zero‑copy views: `with_*_events_view(...)` iterate without allocations (borrows internal buffers).
//!   - Raw slices: `unsafe { with_*_events_raw(...) }` expose FFI slices (borrows internal buffers).
//! - Callback-sensitive event entrypoints also expose matching `try_*` variants so callback-lock
//!   failures can return `ApiError::InCallback` instead of forcing panic-only control flow.
//! - Borrowed view/raw event APIs intentionally stay on `World`, not `WorldHandle`, because they
//!   are tied to the completed step's world-local event buffers and deferred-destroy flushing behavior.
//!
//! Example (reusable buffers + zero‑copy views)
//! ```no_run
//! use boxdd::{ContactEvents, World, WorldDef};
//! let mut world = World::new(WorldDef::default()).unwrap();
//! let mut contact_events = ContactEvents::default();
//! world.contact_events_into(&mut contact_events);
//! world.with_contact_events_view(|begin, end, hit| {
//!     let _ = (begin.count(), end.count(), hit.count());
//! });
//! world.with_sensor_events_view(|beg, end| { let _ = (beg.count(), end.count()); });
//! world.with_body_events_view(|moves| { for m in moves { let _ = (m.body_id(), m.fell_asleep()); } });
//! world.with_joint_events_view(|j| { let _ = j.count(); });
//! ```

pub mod body;
pub mod collision;
pub mod contact;
pub mod debug_draw;
pub mod dynamic_tree;
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
    ShapeCastInput, ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput, ToiOutput,
    ToiState, collide_capsule_and_circle, collide_capsules, collide_chain_segment_and_capsule,
    collide_chain_segment_and_circle, collide_chain_segment_and_polygon, collide_circles,
    collide_polygon_and_capsule, collide_polygon_and_circle, collide_polygons,
    collide_segment_and_capsule, collide_segment_and_circle, collide_segment_and_polygon,
    segment_distance, shape_cast, shape_distance, time_of_impact, try_collide_capsule_and_circle,
    try_collide_capsules, try_collide_chain_segment_and_capsule,
    try_collide_chain_segment_and_circle, try_collide_chain_segment_and_polygon,
    try_collide_circles, try_collide_polygon_and_capsule, try_collide_polygon_and_circle,
    try_collide_polygons, try_collide_segment_and_capsule, try_collide_segment_and_circle,
    try_collide_segment_and_polygon, try_segment_distance, try_shape_cast, try_shape_distance,
    try_time_of_impact,
};
#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
pub use core::math::RotFromGlamError;
#[cfg(feature = "mint")]
#[cfg_attr(docsrs, doc(cfg(feature = "mint")))]
pub use core::math::RotFromMintError;
#[cfg(feature = "cgmath")]
#[cfg_attr(docsrs, doc(cfg(feature = "cgmath")))]
pub use core::math::TransformFromCgmathError;
#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
pub use core::math::TransformFromGlamError;
#[cfg(feature = "mint")]
#[cfg_attr(docsrs, doc(cfg(feature = "mint")))]
pub use core::math::TransformFromMintError;
pub use core::math::{
    HASH_INIT, Rot, Transform, Version, allocated_byte_count, atan2, compute_cos_sin, hash_bytes,
    is_valid_float, length_units_per_meter, milliseconds_and_reset, milliseconds_since,
    rotation_between_unit_vectors, set_length_units_per_meter, ticks, version, yield_now,
};
pub use debug_draw::{DebugDraw, DebugDrawCmd, DebugDrawOptions, HexColor};
pub use dynamic_tree::{DynamicTree, TreeProxyId, TreeRayCastInput, TreeShapeCastInput, TreeStats};
pub use error::{ApiError, ApiResult};
pub use events::{
    BodyMoveEvent, ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents, ContactHitEvent,
    JointEvent, SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents,
};
pub use filter::Filter;
pub use joints::{
    ConstraintTuning, DistanceJointBuilder, DistanceJointDef, FilterJointBuilder, FilterJointDef,
    Joint, JointBase, JointBaseBuilder, JointType, MotorJointBuilder, MotorJointDef,
    PrismaticJointBuilder, PrismaticJointDef, RevoluteJointBuilder, RevoluteJointDef,
    WeldJointBuilder, WeldJointDef, WheelJointBuilder, WheelJointDef,
};
pub use query::{
    Aabb, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult, QueryFilter, RayResult,
    clip_vector, solve_planes, try_clip_vector, try_solve_planes,
};
pub use shapes::chain::{Chain, ChainDef, ChainDefBuilder, ChainDefMaterialLayout, OwnedChain};
pub use shapes::{
    Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, OwnedShape, Polygon, Segment, Shape,
    ShapeDef, ShapeDefBuilder, ShapeType, SurfaceMaterial,
};
pub use types::{
    BodyId, ChainId, ContactData, ContactId, JointId, Manifold, ManifoldPoint, MassData,
    MotionLocks, ShapeId, Vec2,
};
pub use world::{
    CallbackWorld, MaterialMixInput, OutstandingOwnedHandles, OwnedHandleCounts, Profile, World,
    WorldBuilder, WorldDef, WorldHandle,
};
pub use world_extras::ExplosionDef;
