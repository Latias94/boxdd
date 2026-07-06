use bevy::ecs::message::MessageReader;
use bevy::log::info;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
            gravity: Vec2::ZERO,
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, log_sensor_events)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(0.5, 1.5),
        PhysicsMaterial {
            is_sensor: true,
            enable_sensor_events: true,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        BodySettings::bullet(),
        Collider::circle(0.2),
        PhysicsMaterial {
            enable_sensor_events: true,
            ..Default::default()
        },
        LinearVelocity(Vec2::new(4.0, 0.0)),
        Transform::from_xyz(-3.0, 0.0, 0.0),
    ));
}

fn log_sensor_events(
    mut begin: MessageReader<BoxddSensorBeginMessage>,
    mut end: MessageReader<BoxddSensorEndMessage>,
) {
    for message in begin.read() {
        info!(
            ?message.sensor_entity,
            ?message.visitor_entity,
            "sensor overlap began"
        );
    }

    for message in end.read() {
        info!(
            ?message.sensor_entity,
            ?message.visitor_entity,
            "sensor overlap ended"
        );
    }
}
