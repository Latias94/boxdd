# bevy_boxdd

Bevy 0.19 integration for `boxdd` 0.6 and its pinned Box2D 3.2.0 development snapshot at
`56edae79f2949d86142b03450d5d60f63bcf5a6f`.

This crate keeps the core physics binding engine-agnostic and provides Bevy-native ECS components,
fixed-step systems, transform synchronization, ECS-authored joints, entity-mapped ray/AABB queries,
debug draw command collection, and physics messages.

The 0.6 adapter makes the engine/world coordinate boundary explicit. Read the
[0.5 to 0.6 migration guide](../docs/migration-0.5-to-0.6.md) before upgrading an existing app.

## Quick Start

```rust
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
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

- Bevy `Transform` translation is local to `BoxddWorldOrigin`; `boxdd::Position` is always an
  absolute world position. Use the resource's checked conversion methods at this boundary.
- Enable `double-precision` to forward `boxdd/double-precision` while keeping Bevy-local vectors
  and transforms as `f32`.
- Call `BoxddWorldOrigin::request_rebase` to move the local frame atomically. A rebase that cannot
  be staged remains pending and pauses physics until it is replaced, cancelled, or becomes
  representable.
- `boxdd::World` is `!Send`/`!Sync`; the plugin stores it as a non-send Bevy resource.
- `TransformSyncMode::BevyToPhysics` and `PhysicsToBevy` both pass through the active origin. There
  is no implicit `Transform <-> boxdd::WorldTransform` conversion.
- Contact and sensor messages are only emitted for shapes whose `PhysicsMaterial` enables the
  matching Box2D event flags.
- `JointDescriptor` supports ECS-authored distance and revolute joints and inserts `BoxddJoint`
  after the native joint is created.
- `BoxddPhysicsContext` exposes the native `boxdd::World` plus body/shape/joint-to-entity mappings.
  Closest-ray helpers are portable; callback-backed all-hit rays, AABB overlap helpers, and reusable
  debug-draw command collection are native-only until the WASM provider proves Rust callback
  transport.
- Recoverable plugin failures are emitted as `BoxddErrorMessage` by default, including invalid
  collider, material, or joint inputs that fail before native creation.

## World Origin

Insert a non-zero origin before adding the plugin when the simulation starts far from zero:

```rust
# use bevy::prelude::*;
# use bevy_boxdd::prelude::*;
# fn main() -> Result<(), BoxddWorldOriginError> {
let mut app = App::new();
let origin = BoxddWorldOrigin::try_new(boxdd::Position::new(10_000_000.0, 0.0))?;
app.insert_resource(origin)
    .add_plugins(BoxddPhysicsPlugin::default());
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
