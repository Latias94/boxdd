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
        Collider::rectangle(0.35, 0.35),
        PhysicsMaterial {
            density: 1.0,
            friction: 0.7,
            restitution: 0.1,
            enable_contact_events: true,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
}
