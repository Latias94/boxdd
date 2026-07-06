use bevy::ecs::system::NonSend;
use bevy::log::info;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
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

fn report_first_ray_hit(context: NonSend<BoxddPhysicsContext>, mut reported: Local<bool>) {
    if *reported {
        return;
    }

    let Ok(Some(hit)) = context.try_cast_ray_closest_entity(
        Vec2::new(0.0, 3.0),
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
