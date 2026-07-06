use bevy::ecs::message::MessageReader;
use bevy::log::info;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, log_contact_events)
        .run();
}

fn setup(mut commands: Commands) {
    let contact_material = PhysicsMaterial {
        enable_contact_events: true,
        enable_hit_events: true,
        friction: 0.7,
        restitution: 0.1,
        ..Default::default()
    };

    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(5.0, 0.25),
        contact_material,
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::rectangle(0.4, 0.4),
        contact_material,
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
}

fn log_contact_events(
    mut begin: MessageReader<BoxddContactBeginMessage>,
    mut end: MessageReader<BoxddContactEndMessage>,
    mut hit: MessageReader<BoxddContactHitMessage>,
) {
    for message in begin.read() {
        info!(?message.entity_a, ?message.entity_b, "contact began");
    }

    for message in end.read() {
        info!(?message.entity_a, ?message.entity_b, "contact ended");
    }

    for message in hit.read() {
        info!(
            ?message.entity_a,
            ?message.entity_b,
            approach_speed = message.approach_speed,
            "contact hit"
        );
    }
}
