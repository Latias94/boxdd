# boxdd Example Catalog

This catalog groups the examples by the workflow they are meant to teach.

If you are new to `boxdd`, start with the first section instead of scanning file names alphabetically.

Every example that reaches native Box2D initializes one `Foundation` before its first safe native
call. Worlds and scale-sensitive `WorldDef`, `BodyDef`, and `JointBase` defaults are derived from
that same root; worldless collision helpers and `DynamicTree` still require the root to be
initialized even though they do not create a `World`.

## Recommended First Examples

- `world_basics.rs`: one `World`, stored world-bound IDs, borrow-scoped body/shape capabilities,
  explicit destruction, and stepping
- `basic.rs`: slightly broader foundation sample after `world_basics`
- `foundation_scheduler.rs`: one-time length configuration, validated native worker count, and
  runtime scheduler diagnostics
- `shapes_variety.rs`: safe shape geometry creation across the common built-in shape types
- `joints.rs` and `joints_presets.rs`: common joint setup paths

## Math Interop

- `mint_interop.rs`: `mint::Vector2` / `mint::Point2` / matrix conversions across world setup, `Aabb`, `Rot`, and `Transform` (`--features mint`)

## Queries and Hot Paths

- `buffer_reuse.rs`: reusable query buffers and visitor-based hot paths through one `Query`
  capability
- `queries.rs`: borrow-scoped `Query` acquisition, overlap collections, reusable buffers, visitors,
  and polygon overlap helpers
- `query_casts.rs`: ray-cast and shape-cast overview using reusable cast-hit buffers without mover overlap
- `dynamic_tree.rs`: standalone Box2D broad-phase tree ownership, query, ray-cast, and AABB box-cast
  helpers
- `raycast.rs`: focused ray-cast sample
- `shapecast.rs`: focused shape-cast sample
- `character_mover.rs`: the full safe mover pipeline (`cast_mover`, `collide_mover`, `solve_planes`, `clip_vector`)
- `collision_basics.rs`: standalone low-level collision geometry (`segment_distance`, `shape_distance`, `shape_cast`, TOI, shape-A local manifolds, explicit world reconstruction, `Aabb::ray_cast`) without a live world
- `debug_draw.rs`: collected/safe debug draw flows

## Events and Contacts

- `events_summary.rs`: materializing an owned event snapshot from `CompletedStep`
- `events_view.rs`: lazy borrowed event-family views scoped to `CompletedStep`
- `sensors.rs`: sensor events and overlap behavior
- `contacts.rs`: contact behavior and inspection

## Runtime Control and Gameplay Patterns

- `bodies.rs`: body runtime control helpers
- `kinematic_platform.rs`: kinematic-body interaction pattern
- `revolute_motor.rs`, `prismatic_elevator.rs`, `prismatic_wheel.rs`: focused motor/joint control examples
- `bridge.rs`, `car.rs`, `chain_walkway.rs`, `stacking.rs`, `pyramid.rs`: scene-style gameplay setups
- `continuous_bullet.rs`: continuous collision / bullet-style motion
- `determinism.rs`: deterministic stepping expectations
- `robustness.rs`: misuse-resistant or edge-oriented API paths
- `issues.rs`: targeted regressions or issue-driven examples
- `doohickey.rs`, `donut.rs`, `convex_hull.rs`, `benchmark.rs`: specialized geometry or stress samples

## Integration and Ownership Models

- `snapshot_replay.rs`: transactional same-world snapshot restore, opaque process-local recording,
  and exclusive epoch-bound replay without native byte import/export
- `physics_thread.rs`: dedicated-thread ownership model for apps that are otherwise multi-threaded or async-driven
- `world_basics.rs`: the canonical stored-ID and borrow-scoped capability flow; there is no separate
  world-handle or owning-object-handle model
- `../../bevy_boxdd/examples/falling_box_2d.rs`: Bevy ECS adapter smoke example for body/shape creation and transform sync
- `../../bevy_boxdd/examples/ray_query_2d.rs` and `../../bevy_boxdd/examples/overlap_query_2d.rs`: Bevy entity-mapped query helpers for rays and AABB overlaps
- `../../bevy_boxdd/examples/joint_bridge_2d.rs`: Bevy ECS distance/revolute joint authoring
- `../../bevy_boxdd/examples/child_colliders_2d.rs`: Bevy parent body with multiple child collider entities
- `../../bevy_boxdd/examples/collision_filter_2d.rs`: Bevy `PhysicsMaterial::filter` category and mask setup
- `../../bevy_boxdd/examples/debug_draw_collect_2d.rs` and `../../bevy_boxdd/examples/debug_draw_gizmos_2d.rs`: Bevy-side debug draw command collection with and without renderer coupling

## Interactive Testbed

- `testbed/main.rs`: optional interactive testbed using the current `dear-imgui-rs` + `dear-imgui-winit` + `dear-imgui-glow` stack

The testbed scene router lives under `examples/testbed/` and intentionally groups many focused physics demos behind one UI instead of exposing each scene as a separate top-level Cargo example.

WASM runtime qualification is intentionally separate from core examples. See
`../../docs/platforms/wasm.md`; `wasm32-wasip1` is compile-only and has no runtime example.
