# bevy_boxdd

Bevy 0.19 integration for `boxdd` 0.6 and its pinned Box2D 3.2.0 development snapshot at
`56edae79f2949d86142b03450d5d60f63bcf5a6f`.

This crate keeps the core physics binding engine-agnostic and provides Bevy-native ECS components,
fixed-step systems, transform synchronization, ECS-authored joints, entity-mapped ray/AABB queries,
debug draw command collection, and physics messages.

The 0.6 adapter makes the engine/world coordinate boundary explicit. Read the
[0.5 to 0.6 migration guide](MIGRATION.md) before upgrading an existing app.

## Quick Start

```rust
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    let foundation = boxdd::Foundation::initialize_default()
        .expect("Box2D foundation should initialize");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(8.0, 0.25),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::circle(0.4),
        PhysicsMaterial::default(),
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
}
```

## Notes

- Initialize `boxdd::Foundation` once before installing the plugin and pass that exact root to
  `BoxddPhysicsPlugin::new`; the Bevy adapter has no implicit/default Foundation constructor.
- Bevy `Transform` translation is local to `BoxddWorldOrigin`; `boxdd::Position` is always an
  absolute world position. Use the resource's checked conversion methods at this boundary.
- Enable `double-precision` to forward `boxdd/double-precision` while keeping Bevy-local vectors
  and transforms as `f32`.
- Call `BoxddWorldOrigin::request_rebase` to move the local frame atomically. A rebase that cannot
  be staged remains pending and pauses physics until it is replaced, cancelled, or becomes
  representable. Removing or replacing the resource after plugin initialization also pauses the
  pipeline; restore the committed resource state instead of bypassing the rebase transaction.
- `boxdd::World` is `!Send`/`!Sync`; the plugin stores it as a non-send Bevy resource.
- `TransformSyncMode::BevyToPhysics` and `PhysicsToBevy` both pass through the active origin. There
  is no implicit `Transform <-> boxdd::WorldTransform` conversion.
- Event publication is opt-in through `BoxddEventInterests`; the default does not materialize any
  native event family. Contact and sensor messages additionally require the matching
  `PhysicsMaterial` Box2D event flags. Joint threshold events use `BoxddJointEventMessage`.
- `RigidBody` must be a world-root entity. `ChildOf` is supported for collider-only entities, but
  is rejected on rigid-body entities because Bevy-local parent transforms are not a Box2D body
  ownership model.
- Runtime tuning is split into `BoxddStepSettings`, `BoxddEventInterests`, and `BoxddErrorPolicy`.
  `BoxddPhysicsSettings` supplies initial gravity and fixed-clock configuration; change gravity at
  runtime through `BoxddPhysicsContext::set_gravity`. Use `BoxddPhysicsSet` to order application
  systems around pipeline stages.
- `JointDescriptor` supports ECS-authored distance and revolute joints and inserts `BoxddJoint`
  after the native joint is created.
- `BoxddPhysicsContext` owns one `boxdd::World` and is the checked ECS gateway to its world-bound
  IDs and borrow-scoped capabilities. Native mutation stays behind context-owned controls so each
  mutation boundary keeps the Box2D world, identity graph, and ECS projections coherent. Plugin
  snapshots restore all three as one transaction and are bound to the originating Bevy `WorldId`;
  cross-world restore requires explicit application remapping rather than coincidentally equal
  `Entity` values. Queue a one-shot restore with `BoxddPhysicsContext::queue_snapshot_restore`
  and read `BoxddSnapshotRestoreMessage` for its result; it commits in `BoxddPhysicsSet::Restore`,
  before cleanup and stepping can expose a restored object to stale authored ECS state. If a
  transient same-world origin or binding keeps the request pending, `cancel_snapshot_restore`
  returns the owned snapshot and frees the slot without emitting a completion message. The fixed
  update pipeline also rejects a context moved into another Bevy world before rebasing,
  reconciling, or stepping state.
  Closest-ray helpers are portable; callback-backed all-hit rays, AABB overlap helpers, and reusable
  debug-draw command collection are native-only until the WASM provider proves Rust callback
  transport.
- The core wrapper has one `Result` API. Systems surface recoverable failures as
  `BoxddErrorMessage` by default, including invalid collider, material, or joint inputs rejected
  before native creation.

## World Origin

Insert a non-zero origin before adding the plugin when the simulation starts far from zero:

```rust
# use bevy::prelude::*;
# use bevy_boxdd::prelude::*;
# fn main() -> Result<(), BoxddWorldOriginError> {
let mut app = App::new();
let foundation = boxdd::Foundation::initialize_default()
    .expect("Box2D foundation should initialize");
let origin = BoxddWorldOrigin::new(boxdd::Position::new(10_000_000.0, 0.0))?;
app.insert_resource(origin)
    .add_plugins(BoxddPhysicsPlugin::new(
        foundation,
        BoxddPhysicsSettings::default(),
    ));
# Ok(())
# }
```

Queries and world-space joint anchors use absolute `boxdd::Position` values. Convert authored
Bevy-local points explicitly with `checked_local_to_absolute`; convert query hits and debug-draw
positions back with `checked_absolute_to_local`.

An origin rebase is transactional: the plugin first stages every affected local transform, then
commits the new origin and revision together. If any absolute position cannot be represented in the
new local `f32` frame, the active origin and all transforms remain unchanged, the request stays
pending, and physics remains paused until the request is replaced, cancelled, or becomes valid.

In `double-precision` mode, Bevy vectors and transforms still use `f32`; only absolute Box2D
positions use `f64`. Keep large coordinates in `boxdd::Position` and narrow only after subtracting
the active origin.

## Examples

Run examples with `cargo run -p bevy_boxdd --example <name>`.

The GitHub Pages site builds `testbed_2d` to WebAssembly and exposes each scene as a dedicated Bevy + egui browser route.

| Example | Shows |
| --- | --- |
| `falling_box_2d` | Basic body, collider, material, fixed-step stepping, and transform sync. |
| `contact_events_2d` | Contact begin/end/hit messages mapped back to Bevy entities. |
| `sensor_events_2d` | Sensor begin/end messages for trigger-style overlaps. |
| `ray_query_2d` | Entity-mapped ray queries through `BoxddPhysicsContext`. |
| `overlap_query_2d` | Entity-mapped AABB overlap queries for area triggers, pickups, and editor selection. |
| `kinematic_platform_2d` | Driving a kinematic body from Bevy transforms with `BevyToPhysics` sync. |
| `joint_bridge_2d` | Distance and revolute joint descriptors authored as ECS components. |
| `child_colliders_2d` | A compound body authored as one parent `RigidBody` with multiple child `Collider` entities. |
| `collision_filter_2d` | Collision category and mask setup through `PhysicsMaterial::filter`. |
| `debug_draw_collect_2d` | Collecting render-agnostic `boxdd::DebugDrawCmd` values from the Bevy context. |
| `debug_draw_gizmos_2d` | Rendering collected `boxdd::DebugDrawCmd` values with Bevy Gizmos. |
| `testbed_2d` | Browser-oriented Bevy + egui testbed with official Box2D sample-style scenes for stacking, bodies, continuous collision, materials, events, and joints. |
