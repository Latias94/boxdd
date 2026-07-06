# bevy_boxdd

Bevy integration for `boxdd`, the Rust bindings to the official Box2D v3 C API.

This crate keeps the core physics binding engine-agnostic and provides Bevy-native ECS components,
fixed-step systems, transform synchronization, ECS-authored joints, entity-mapped ray/AABB queries,
debug draw command collection, and physics messages.

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

- Bevy `Transform` maps to Box2D XY translation and Z-axis rotation.
- `boxdd::World` is `!Send`/`!Sync`; the plugin stores it as a non-send Bevy resource.
- Contact and sensor messages are only emitted for shapes whose `PhysicsMaterial` enables the
  matching Box2D event flags.
- `JointDescriptor` supports ECS-authored distance and revolute joints and inserts `BoxddJoint`
  after the native joint is created.
- `BoxddPhysicsContext` exposes the native `boxdd::World` plus body/shape/joint-to-entity mappings,
  entity-level ray helpers, AABB overlap helpers, and reusable debug-draw command collection.
- Recoverable plugin failures are emitted as `BoxddErrorMessage` by default, including invalid
  collider, material, or joint inputs that fail before native creation.

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
