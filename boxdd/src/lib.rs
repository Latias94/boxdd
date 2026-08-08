//! Safe, owner-scoped Rust bindings for the pinned Box2D v3 C API.
//!
//! `boxdd` deliberately presents one ownership model and one fallible API:
//!
//! - [`World`] owns the native simulation and all Rust-side identity and callback state.
//! - [`BodyId`], [`ShapeId`], [`JointId`], and [`ChainId`] are copyable, world-bound identifiers
//!   intended for application storage.
//! - [`Body`], [`Shape`], [`Joint`], and [`Chain`] are capabilities tied to a mutable borrow of the
//!   world. Dropping one releases the borrow; destruction is always explicit.
//! - [`Query`] is a read-only capability tied to its owner borrow. Reusable query buffers avoid
//!   repeated allocation on hot paths.
//! - Public operations that can fail return [`Result`]. Stale IDs, invalid definitions, callback
//!   reentry, and terminal native state are reported rather than routed through a parallel
//!   panic-style API.
//!
//! Live IDs cannot be detached, rebound, reconstructed from raw Box2D IDs, or used as persistence
//! keys through Safe Rust. Use application-owned stable keys for saved state. Use [`boxdd_sys`]
//! directly only when the caller accepts the raw FFI contract.
//!
//! # Quick start
//!
//! ```no_run
//! use boxdd::{BodyType, Foundation, ShapeDef, Vec2, shapes};
//!
//! let foundation = Foundation::initialize_default().unwrap();
//! let mut world = foundation.create_world(
//!     foundation
//!         .world_builder()
//!         .gravity(Vec2::new(0.0, -9.8))
//!         .build()?,
//! )?;
//! let body_id = world.create_body(
//!     world
//!         .body_builder()
//!         .body_type(BodyType::Dynamic)
//!         .position([0.0, 2.0])
//!         .build()?,
//! )?;
//!
//! {
//!     let mut body = world.body(body_id)?;
//!     body.create_polygon(
//!         &ShapeDef::builder().density(1.0).build()?,
//!         &shapes::box_polygon(0.5, 0.5)?,
//!     )?;
//! }
//!
//! let completed = world.step(1.0 / 60.0, 4)?;
//! let contacts = completed.contact_events()?.to_owned()?;
//! # let _ = contacts;
//! # Ok::<(), boxdd::Error>(())
//! ```
//!
//! # Ownership and identity
//!
//! Creation returns a world-bound ID. Acquire a capability when an operation needs access to the
//! corresponding object:
//!
//! ```no_run
//! # use boxdd::Foundation;
//! # let foundation = Foundation::initialize_default().unwrap();
//! # let mut world = foundation.create_world(foundation.world_def())?;
//! let body_id = world.create_body(world.body_builder().build()?)?;
//! world.body(body_id)?.set_awake(true)?;
//! world.body(body_id)?.destroy()?;
//! # Ok::<(), boxdd::Error>(())
//! ```
//!
//! Capability acquisition validates the ID once and prevents overlapping mutable world access for
//! the lifetime of the capability. A body, shape, joint, or chain is destroyed only by its
//! explicit `destroy` method or by destruction of an owning native object/world.
//!
//! [`ContactId`] values are additionally tied to a contact epoch. Inspect them through
//! [`World::contact_is_valid`] and [`World::contact_data`].
//!
//! # Coordinates
//!
//! [`Position`] and [`WorldTransform`] represent absolute world coordinates using [`WorldScalar`],
//! which is `f32` by default and `f64` with `double-precision`. [`Vec2`] and [`Transform`] represent
//! local offsets, directions, extents, and relative transforms and always remain `f32`.
//!
//! Queries therefore take an explicit absolute origin plus local geometry:
//!
//! ```no_run
//! use boxdd::{Aabb, Foundation, Position, QueryFilter, ShapeQueryBuffer};
//!
//! let foundation = Foundation::initialize_default().unwrap();
//! let world = foundation.create_world(foundation.world_def())?;
//! let query = world.query()?;
//! let mut hits = ShapeQueryBuffer::new();
//! query.overlap_aabb_into(
//!     Position::ZERO,
//!     Aabb::from_center_half_extents([0.0, 0.0], [5.0, 5.0])?,
//!     QueryFilter::default(),
//!     &mut hits,
//! )?;
//! # Ok::<(), boxdd::Error>(())
//! ```
//!
//! # Threading and callbacks
//!
//! [`World`] and its borrow-scoped capabilities are `!Send` and `!Sync`. Keep a world on one
//! owner thread; a dedicated physics thread plus channels is the portable integration model for
//! multi-threaded or async applications.
//!
//! On native targets, custom-filter, pre-solve, friction-mix, and restitution-mix callbacks may run
//! on Box2D workers. Their closures are `Send + Sync + 'static` and receive only copyable values and
//! branded IDs, never a world context. Query, dynamic-tree, event-view, and debug-draw callbacks
//! are closure-scoped. Every Rust callback catches unwind-capable panics before returning through C.
//!
//! [`WorkerCount`] selects the qualified built-in Box2D scheduler. Safe Rust does not expose raw
//! task-system function pointers.
//!
//! # Snapshots, recording, and replay
//!
//! [`Snapshot`] is an opaque capability that can restore only its originating world.
//! [`RecordingSession`] exclusively borrows a world and produces an opaque process-local
//! [`Recording`]. [`ReplayPlayer`] accepts only such a recording and exposes epoch-bound views while
//! holding exclusive process-global foundation access.
//!
//! Safe Rust exposes no native snapshot or recording bytes, fresh-world snapshot loading, or replay
//! from external bytes. Durable persistence requires an application-owned versioned schema that
//! rebuilds a world.
//!
//! # Features
//!
//! - `double-precision` changes absolute world coordinates and the native ABI together.
//! - `serde` covers safe value and configuration types, not live worlds or object IDs.
//! - `mint`, `nalgebra`, and `glam` provide scalar-correct math interop.
//! - `bytemuck` covers layout-qualified value types.
//!
//! See the repository's migration guide and FFI lifetime audit for the complete ownership,
//! provider, callback, and platform contracts.

const _: () = assert!(
    boxdd_sys::IS_DOUBLE_PRECISION == cfg!(feature = "double-precision"),
    "boxdd and boxdd-sys precision features must be enabled through the same dependency edge"
);

pub mod body;
pub mod collision;
pub mod contact;
pub mod debug_draw;
pub mod dynamic_tree;
pub mod error;
pub mod events;
pub mod filter;
pub mod id;
pub mod joints;
pub mod prelude;
pub mod query;
pub mod recording;
pub mod replay;
pub mod shapes;
pub mod snapshot;
pub mod tuning;
pub mod types;
pub mod world;
pub mod world_extras;
pub mod core {
    pub(crate) mod callback_state;
    pub(crate) mod ffi_vec;
    pub mod foundation;
    pub(crate) mod identity_registry;
    pub(crate) mod length_scale;
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) mod material_mix_registry;
    pub mod math;
    pub(crate) mod native_defaults;
    pub(crate) mod user_data;
    pub(crate) mod world_core;
}

pub use body::{Body, BodyBuilder, BodyDef, BodyType, MAX_BODY_NAME_BYTES};
pub use collision::{
    CastOutput, DistanceInput, DistanceOutput, LocalManifold, LocalManifoldPoint,
    MAX_LOCAL_MANIFOLD_POINTS, MAX_SHAPE_PROXY_POINTS, SegmentDistanceResult, ShapeCastInput,
    ShapeCastPairInput, ShapeProxy, SimplexCache, Sweep, ToiInput, ToiOutput, ToiState,
    collide_capsule_and_circle, collide_capsules, collide_chain_segment_and_capsule,
    collide_chain_segment_and_circle, collide_chain_segment_and_polygon, collide_circles,
    collide_polygon_and_capsule, collide_polygon_and_circle, collide_polygons,
    collide_segment_and_capsule, collide_segment_and_circle, collide_segment_and_polygon,
    segment_distance, shape_cast, shape_distance, time_of_impact,
};
pub use core::foundation::{
    Foundation, FoundationActivity, FoundationActivityError, FoundationAdapterIdentityField,
    FoundationConfig, FoundationDiagnostics, FoundationInitError,
};
#[cfg(not(target_arch = "wasm32"))]
pub use core::foundation::{FoundationAssertHook, FoundationLogHook};
#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
pub use core::math::RotFromGlamError;
#[cfg(feature = "mint")]
#[cfg_attr(docsrs, doc(cfg(feature = "mint")))]
pub use core::math::RotFromMintError;
#[cfg(feature = "glam")]
#[cfg_attr(docsrs, doc(cfg(feature = "glam")))]
pub use core::math::TransformFromGlamError;
#[cfg(feature = "mint")]
#[cfg_attr(docsrs, doc(cfg(feature = "mint")))]
pub use core::math::TransformFromMintError;
pub use core::math::{
    HASH_INIT, Rot, Transform, Version, allocated_byte_count, atan2, compute_cos_sin, hash_bytes,
    is_valid_float, milliseconds_and_reset, milliseconds_since, rotation_between_unit_vectors,
    ticks, version, yield_now,
};
#[cfg(not(target_arch = "wasm32"))]
pub use debug_draw::DebugDraw;
pub use debug_draw::{DebugDrawCmd, DebugDrawOptions, HexColor};
pub use dynamic_tree::{
    DynamicTree, TreeBoxCastInput, TreeCastControl, TreeProxyId, TreeRayCastInput, TreeStats,
};
pub use error::{Error, Result};
pub use events::{
    BodyEvents, BodyMoveEvent, CompletedStep, ContactBeginTouchEvent, ContactEndTouchEvent,
    ContactEvents, ContactEventsView, ContactHitEvent, JointEvent, JointEvents,
    SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents, SensorEventsView, StepEventsSnapshot,
};
pub use filter::Filter;
pub use joints::{
    ConstraintTuning, DistanceJoint, DistanceJointBuilder, DistanceJointDef, FilterJoint,
    FilterJointBuilder, FilterJointDef, Joint, JointBase, JointType, MotorJoint, MotorJointBuilder,
    MotorJointDef, PrismaticJoint, PrismaticJointBuilder, PrismaticJointDef, RevoluteJoint,
    RevoluteJointBuilder, RevoluteJointDef, WeldJoint, WeldJointBuilder, WeldJointDef, WheelJoint,
    WheelJointBuilder, WheelJointDef,
};
pub use query::{
    Aabb, ClosestRayCastResult, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult, Query,
    QueryFilter, RayResult, clip_vector, solve_planes,
};
#[cfg(not(target_arch = "wasm32"))]
pub use query::{MoverQueryBuffer, RayQueryBuffer, ShapeQueryBuffer};
pub use recording::{MixerId, MixerIdentities, Recording, RecordingLimits, RecordingSession};
pub use replay::{
    ReplayBodyView, ReplayConfig, ReplayEpoch, ReplayInfo, ReplayKeyframePolicy,
    ReplayKeyframeState, ReplayPlayer, ReplayQueryHitView, ReplayQueryKind, ReplayQueryView,
    ReplayStatus, ReplayView,
};
pub use shapes::chain::{Chain, ChainDef, ChainDefBuilder, ChainDefMaterialLayout};
pub use shapes::{
    Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, Polygon, Segment, Shape, ShapeDef,
    ShapeDefBuilder, ShapeType, SurfaceMaterial,
};
pub use snapshot::{PreparedSnapshotRestore, Snapshot, SnapshotRestore};
pub use types::{
    BodyId, ChainId, ContactData, ContactId, JointId, MAX_MANIFOLD_POINTS, Manifold, ManifoldPoint,
    MassData, MotionLocks, Position, PositionToLocalError, ShapeId, Vec2, WorldCastOutput,
    WorldScalar, WorldTransform, WorldTransformFromInteropError,
};
pub use world::{
    B2_MAX_WORKERS, Counters, MaterialMixInput, Profile, WorkerCount, World, WorldBuilder,
    WorldCapacity, WorldDef,
};
pub use world_extras::ExplosionDef;
