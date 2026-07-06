use std::collections::HashSet;

use bevy::ecs::system::NonSendMut;
use bevy::log::warn;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

const PROBE_HALF_EXTENTS: Vec2 = Vec2::new(0.8, 0.55);

#[derive(Component)]
struct Pickup;

#[derive(Component)]
struct Probe;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::new(BoxddPhysicsSettings {
            gravity: Vec2::ZERO,
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_probe, highlight_pickups).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Probe,
        Sprite::from_color(Color::srgba(0.9, 0.9, 1.0, 0.18), PROBE_HALF_EXTENTS * 2.0),
        Transform::from_xyz(-2.5, 0.0, 10.0),
    ));

    for (x, y) in [(-2.0, 0.2), (-0.6, -0.35), (0.8, 0.35), (2.1, -0.1)] {
        commands.spawn((
            Pickup,
            RigidBody::Static,
            Collider::circle(0.22),
            PhysicsMaterial {
                is_sensor: true,
                ..Default::default()
            },
            Sprite::from_color(Color::srgb(0.25, 0.55, 0.95), Vec2::splat(0.44)),
            Transform::from_xyz(x, y, 0.0),
        ));
    }
}

fn move_probe(time: Res<Time>, mut probe: Single<&mut Transform, With<Probe>>) {
    let x = (time.elapsed_secs() * 0.7).sin() * 2.6;
    probe.translation.x = x;
}

fn highlight_pickups(
    mut context: NonSendMut<BoxddPhysicsContext>,
    probe: Single<&Transform, With<Probe>>,
    mut hits: Local<Vec<BoxddShapeHit>>,
    mut pickups: Query<(Entity, &mut Sprite), With<Pickup>>,
) {
    let center = probe.translation.truncate();
    let aabb = boxdd::Aabb::from_center_half_extents(
        center.to_boxdd_vec2(),
        PROBE_HALF_EXTENTS.to_boxdd_vec2(),
    );

    if let Err(error) =
        context.try_overlap_aabb_entities_into(aabb, boxdd::QueryFilter::default(), &mut hits)
    {
        warn!(?error, "overlap query failed");
        return;
    }

    let active = hits
        .iter()
        .filter_map(|hit| hit.entity)
        .collect::<HashSet<_>>();
    for (entity, mut sprite) in &mut pickups {
        sprite.color = if active.contains(&entity) {
            Color::srgb(1.0, 0.8, 0.2)
        } else {
            Color::srgb(0.25, 0.55, 0.95)
        };
    }
}
