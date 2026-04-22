# boxdd Example Catalog

This catalog groups the examples by the workflow they are meant to teach.

If you are new to `boxdd`, start with the first section instead of scanning file names alphabetically.

## Recommended First Examples

- `world_basics.rs`: minimal world, body, shape creation, and stepping
- `basic.rs`: slightly broader foundation sample after `world_basics`
- `shapes_variety.rs`: safe shape geometry creation across the common built-in shape types
- `joints.rs` and `joints_presets.rs`: common joint setup paths

## Queries and Hot Paths

- `queries.rs`: broad-phase overlap and query basics
- `query_casts.rs`: combined overlap, ray, and shape-cast workflows
- `raycast.rs`: focused ray-cast sample
- `shapecast.rs`: focused shape-cast sample
- `character_mover.rs`: the full safe mover pipeline (`cast_mover`, `collide_mover`, `solve_planes`, `clip_vector`)
- `collision_basics.rs`: standalone `boxdd::collision` APIs outside a live world
- `debug_draw.rs`: collected/safe debug draw flows

## Events and Contacts

- `events_summary.rs`: owned event snapshots
- `events_view.rs`: borrowed zero-copy event views
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

## Serialization and Threading

- `scene_serialize.rs`: scene snapshot round-trip (`--features serialize`)
- `physics_thread.rs`: dedicated-thread ownership model for apps that are otherwise multi-threaded or async-driven
- `wasm_wasi_smoke.rs`: minimal WASM/WASI-oriented smoke example

## Interactive Testbed

- `testbed_imgui_glow.rs`: optional interactive testbed using the current `dear-imgui-rs` + `dear-imgui-winit` + `dear-imgui-glow` stack

The testbed scene router lives under `examples/testbed/` and intentionally groups many focused physics demos behind one UI instead of exposing each scene as a separate top-level Cargo example.
