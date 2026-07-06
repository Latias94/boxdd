use bevy::ecs::message::MessageReader;
use bevy::log::info;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

const PLAYER: u64 = 0x0002;
const TERRAIN: u64 = 0x0004;
const GHOST: u64 = 0x0008;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, log_contacts)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(5.0, 0.18),
        material(TERRAIN, PLAYER),
        Sprite::from_color(Color::srgb(0.2, 0.45, 0.35), Vec2::new(5.0, 0.18)),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(2.0, 0.12),
        material(GHOST, GHOST),
        Sprite::from_color(Color::srgba(0.65, 0.65, 0.85, 0.35), Vec2::new(2.0, 0.12)),
        Transform::from_xyz(0.0, 0.65, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::rectangle(0.35, 0.35),
        material(PLAYER, TERRAIN),
        Sprite::from_color(Color::srgb(0.95, 0.7, 0.25), Vec2::splat(0.7)),
        Transform::from_xyz(0.0, 2.2, 0.0),
    ));
}

fn material(category_bits: u64, mask_bits: u64) -> PhysicsMaterial {
    PhysicsMaterial {
        enable_contact_events: true,
        filter: boxdd::Filter {
            category_bits,
            mask_bits,
            group_index: 0,
        },
        ..Default::default()
    }
}

fn log_contacts(mut begin: MessageReader<BoxddContactBeginMessage>) {
    for message in begin.read() {
        info!(
            ?message.entity_a,
            ?message.entity_b,
            "contact passed collision filters"
        );
    }
}
