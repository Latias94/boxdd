# bevy_boxdd

Bevy integration for `boxdd`, the Rust bindings to the official Box2D v3 C API.

This crate keeps the core physics binding engine-agnostic and provides Bevy-native ECS components,
fixed-step systems, transform synchronization, and physics messages.

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
