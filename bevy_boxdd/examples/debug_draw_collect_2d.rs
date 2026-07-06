use bevy::ecs::system::NonSendMut;
use bevy::log::{info, warn};
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, report_debug_draw_commands)
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
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));
}

fn report_debug_draw_commands(
    mut context: NonSendMut<BoxddPhysicsContext>,
    mut commands: Local<Vec<boxdd::DebugDrawCmd>>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }

    if let Err(error) =
        context.try_debug_draw_collect_into(&mut commands, boxdd::DebugDrawOptions::default())
    {
        warn!(?error, "debug draw collection failed");
        return;
    }

    if commands.is_empty() {
        return;
    }

    let mut polygons = 0usize;
    let mut circles = 0usize;
    let mut segments = 0usize;
    for command in commands.iter() {
        match command {
            boxdd::DebugDrawCmd::Polygon { .. } | boxdd::DebugDrawCmd::SolidPolygon { .. } => {
                polygons += 1;
            }
            boxdd::DebugDrawCmd::Circle { .. } | boxdd::DebugDrawCmd::SolidCircle { .. } => {
                circles += 1;
            }
            boxdd::DebugDrawCmd::Segment { .. } => {
                segments += 1;
            }
            boxdd::DebugDrawCmd::SolidCapsule { .. }
            | boxdd::DebugDrawCmd::Transform(_)
            | boxdd::DebugDrawCmd::Point { .. }
            | boxdd::DebugDrawCmd::String { .. } => {}
        }
    }

    info!(
        total = commands.len(),
        polygons, circles, segments, "debug draw commands collected"
    );
    *reported = true;
}
