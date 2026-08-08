use bevy::ecs::system::NonSend;
use bevy::log::{info, warn};
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    let foundation =
        boxdd::Foundation::initialize_default().expect("Box2D foundation should initialize");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, report_first_ray_hit)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(4.0, 0.25),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::circle(0.35),
        Transform::from_xyz(1.0, 2.5, 0.0),
    ));
}

fn report_first_ray_hit(
    context: NonSend<BoxddPhysicsContext>,
    origin: Res<BoxddWorldOrigin>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }

    let Ok(ray_origin) = origin.checked_local_to_absolute(Vec2::new(0.0, 3.0)) else {
        warn!("ray origin is outside the active world-origin frame");
        return;
    };
    let Ok(Some(hit)) = context.cast_ray_closest_entity(
        ray_origin,
        Vec2::new(0.0, -6.0),
        boxdd::QueryFilter::default(),
    ) else {
        return;
    };

    info!(
        entity = ?hit.entity,
        point = ?hit.hit.point,
        normal = ?hit.hit.normal,
        "ray hit"
    );
    *reported = true;
}
