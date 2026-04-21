//! Tuning Notes and Upstream Constants
//!
//! This module documents upstream Box2D tuning concepts (from constants.h)
//! and how they map to the safe `boxdd` API. Many of these are engine
//! internals and not exposed as stable constants here, but the related
//! behavior can be controlled through safe setters.
//!
//! Key concepts (upstream names in parentheses):
//!
//! - Linear slop (`B2_LINEAR_SLOP`): small collision/constraint tolerance.
//!   Not directly exposed; part of solver internals. Affects contact stability.
//! - Speculative distance (`B2_SPECULATIVE_DISTANCE`): reduces jitter with
//!   limited speculative collision. Runtime access is exposed through
//!   `World::enable_speculative`.
//! - AABB margin (`B2_AABB_MARGIN`): fattening of dynamic-tree proxies to avoid
//!   frequent broadphase updates. Not directly exposed.
//! - Max rotation per step (`B2_MAX_ROTATION`): large cap to prevent numerical
//!   issues; increasing too much can break continuous collision.
//! - Time to sleep (`B2_TIME_TO_SLEEP`): inactivity time before sleeping.
//!   Use `World::enable_sleeping`/`World::is_sleeping_enabled` to toggle sleeping.
//! - Graph color count (`B2_GRAPH_COLOR_COUNT`): internal constraint-coloring
//!   size. Not exposed.
//! - Max workers (`B2_MAX_WORKERS`): internal upper bound; configure desired
//!   worker count via `WorldDef::builder().worker_count(n)`. Actual multithreaded
//!   stepping still requires explicit raw task callbacks on `WorldDef` / `WorldBuilder`.
//!
//! Safe API controls related to tuning:
//!
//! - Restitution and hit thresholds
//!   - `WorldBuilder::restitution_threshold`, `World::set_restitution_threshold`,
//!     `World::restitution_threshold`
//!   - `WorldBuilder::hit_event_threshold`, `World::set_hit_event_threshold`,
//!     `World::hit_event_threshold`
//! - Contact solver tuning
//!   - `WorldBuilder::contact_hertz`, `WorldBuilder::contact_damping_ratio`,
//!     `WorldBuilder::contact_speed`
//!   - `World::set_contact_tuning(hertz, damping_ratio, push_speed)`
//! - Warm starting
//!   - `World::enable_warm_starting`, `World::is_warm_starting_enabled`
//! - Sleeping and continuous collision detection
//!   - `WorldBuilder::enable_sleep`, `World::enable_sleeping`, `World::is_sleeping_enabled`
//!   - `WorldBuilder::enable_continuous`, `World::enable_continuous`, `World::is_continuous_enabled`
//! - Speculative collision
//!   - `World::enable_speculative`
//! - Maximum linear speed
//!   - `WorldBuilder::maximum_linear_speed`, `World::set_maximum_linear_speed`,
//!     `World::maximum_linear_speed`
//! - Worker threads
//!   - `WorldBuilder::worker_count`
//!   - This only affects Box2D's worker usage when a task system is also installed; `World`
//!     itself stays pinned to one thread/task.
//!
//! Notes
//! - Upstream constants in `src/constants.h` are implementation details and may
//!   change across Box2D versions. The safe API focuses on stable, high-level
//!   controls. If you need additional tuning hooks, open an issue and we can
//!   consider exposing them in a versioned, documented way.
