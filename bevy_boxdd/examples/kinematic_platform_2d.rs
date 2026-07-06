use bevy::prelude::*;
use bevy_boxdd::prelude::*;

#[derive(Component)]
struct Platform;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, move_platform)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Platform,
        RigidBody::Kinematic,
        Collider::rectangle(1.5, 0.2),
        TransformSyncMode::BevyToPhysics,
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::rectangle(0.25, 0.25),
        PhysicsMaterial {
            friction: 0.8,
            enable_contact_events: true,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}

fn move_platform(mut phase: Local<f32>, mut platforms: Query<&mut Transform, With<Platform>>) {
    *phase += 1.0 / 60.0;
    let x = (*phase * 1.5).sin() * 2.0;

    for mut transform in &mut platforms {
        transform.translation.x = x;
    }
}
