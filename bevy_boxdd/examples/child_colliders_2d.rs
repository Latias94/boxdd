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
    commands.spawn(Camera2d);

    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(5.0, 0.2),
        Sprite::from_color(Color::srgb(0.2, 0.35, 0.3), Vec2::new(5.0, 0.2)),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    commands
        .spawn((
            RigidBody::Dynamic,
            BodySettings {
                angular_damping: 0.25,
                ..Default::default()
            },
            AngularImpulse::new(1.8),
            Sprite::from_color(Color::srgb(0.2, 0.5, 0.85), Vec2::new(1.15, 0.18)),
            Transform::from_xyz(0.0, 1.4, 0.0),
        ))
        .with_children(|body| {
            body.spawn((
                Collider::rectangle(0.58, 0.09),
                PhysicsMaterial {
                    density: 1.0,
                    friction: 0.7,
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            body.spawn((
                Collider::circle(0.2),
                PhysicsMaterial {
                    density: 0.7,
                    ..Default::default()
                },
                Sprite::from_color(Color::srgb(0.95, 0.55, 0.25), Vec2::splat(0.4)),
                Transform::from_xyz(-0.58, 0.0, 0.0),
            ));
            body.spawn((
                Collider::circle(0.2),
                PhysicsMaterial {
                    density: 0.7,
                    ..Default::default()
                },
                Sprite::from_color(Color::srgb(0.95, 0.55, 0.25), Vec2::splat(0.4)),
                Transform::from_xyz(0.58, 0.0, 0.0),
            ));
        });
}
