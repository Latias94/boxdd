#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::broken_intra_doc_links)]
//! boxdd: Safe, ergonomic Rust bindings for Box2D (v3 C API)
//!
//! Highlights
//! - Thin safe layer on top of the official Box2D v3 C API.
//! - Modular API: world, bodies, shapes, joints, queries, events, debug draw.
//! - Ergonomics: builder patterns, world-space helpers, mint integration.
//! - Two usage styles:
//!   - RAII wrappers (Rust lifetimes, Drop auto-destroys ids).
//!   - ID-style (return raw ids; easy to store and pass around without borrow issues).
//!
//! Quickstart (RAII)
//! ```no_run
//! use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
//! let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
//! let mut world = World::new(def).unwrap();
//! {
//!     // Limit the borrow of `world` by scoping the body wrapper.
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
//! mint integration
//! - `b2Vec2` accepts `mint::Vector2<f32>`, `mint::Point2<f32>`, `[f32; 2]`, `(f32, f32)` anywhere `Into<b2Vec2>` is used.
//! - Returned vectors can be converted back using `From` to mint types.
//!
//! Modules
//! - `world`, `body`, `shapes`, `joints`, `query`, `events`, `debug_draw`, `prelude`.
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
//! // Ray (closest)
//! let r = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
//! if r.hit { let _ = (r.point, r.normal, r.fraction); }
//! ```
//!
//! Feature Flags
//! - `serialize`: scene snapshot helpers (save/apply world config; build/restore minimal full-scene snapshot).
//! - `pkg-config`: allow linking against a system `box2d` via pkg-config.
//! - `cgmath` / `nalgebra` / `glam`: conversions with their 2D math types.
//!
//! Events
//! - Three access styles:
//!   - By value: `world.contact_events()`/`sensor_events()`/`body_events()`/`joint_events()` return owned data for storage or cross‑frame use.
//!   - Zero‑copy views: `with_*_events_view(...)` iterate without exposing FFI types and without allocations (recommended per‑frame).
//!   - Raw slices: `with_*_events(...)` expose FFI slices (advanced); data valid only within the callback.
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
pub mod debug_draw;
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
pub mod world;
pub mod world_extras;
pub mod core {
    pub mod math;
}

pub use body::{Body, BodyBuilder, BodyDef, BodyType};
pub use core::math::{Rot, Transform};
pub use debug_draw::{DebugDraw, DebugDrawOptions};
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
pub use query::{Aabb, QueryFilter, RayResult};
pub use shapes::chain::{Chain, ChainDef, ChainDefBuilder};
pub use shapes::{Shape, ShapeDef, ShapeDefBuilder, SurfaceMaterial};
pub use types::Vec2;
pub use world::{World, WorldBuilder, WorldDef};
