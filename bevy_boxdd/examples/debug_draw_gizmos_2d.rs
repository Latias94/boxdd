use bevy::ecs::system::NonSendMut;
use bevy::log::warn;
use bevy::prelude::*;
use bevy_boxdd::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BoxddPhysicsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, draw_boxdd_gizmos)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(4.0, 0.18),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    for (x, radius) in [(-0.7, 0.28), (0.0, 0.36), (0.8, 0.22)] {
        commands.spawn((
            RigidBody::Dynamic,
            Collider::circle(radius),
            Transform::from_xyz(x, 1.4 + radius, 0.0),
        ));
    }

    let left = commands
        .spawn((RigidBody::Static, Transform::from_xyz(-1.6, 0.4, 0.0)))
        .id();
    let right = commands
        .spawn((
            RigidBody::Dynamic,
            Collider::rectangle(0.35, 0.08),
            Transform::from_xyz(-0.9, 0.4, 0.0),
        ))
        .id();
    commands.spawn(
        JointDescriptor::distance(left, right, Vec2::new(-1.6, 0.4), Vec2::new(-0.9, 0.4))
            .with_constraint_tuning(4.0, 0.7),
    );
}

fn draw_boxdd_gizmos(
    mut context: NonSendMut<BoxddPhysicsContext>,
    mut commands: Local<Vec<boxdd::DebugDrawCmd>>,
    mut gizmos: Gizmos,
) {
    if let Err(error) =
        context.try_debug_draw_collect_into(&mut commands, boxdd::DebugDrawOptions::default())
    {
        warn!(?error, "debug draw collection failed");
        return;
    }

    for command in commands.iter() {
        match command {
            boxdd::DebugDrawCmd::Polygon { vertices, color } => {
                draw_loop(&mut gizmos, vertices.iter().copied(), debug_color(*color));
            }
            boxdd::DebugDrawCmd::SolidPolygon {
                transform,
                vertices,
                color,
                ..
            } => {
                draw_loop(
                    &mut gizmos,
                    vertices
                        .iter()
                        .copied()
                        .map(|point| transform.transform_point(point)),
                    debug_color(*color),
                );
            }
            boxdd::DebugDrawCmd::Circle {
                center,
                radius,
                color,
            } => {
                gizmos.circle_2d(center.to_bevy_vec2(), *radius, debug_color(*color));
            }
            boxdd::DebugDrawCmd::SolidCircle {
                transform,
                radius,
                color,
            } => {
                let center = transform.position().to_bevy_vec2();
                let axis = transform
                    .transform_point(boxdd::Vec2::new(*radius, 0.0))
                    .to_bevy_vec2();
                let color = debug_color(*color);
                gizmos.circle_2d(center, *radius, color);
                gizmos.line_2d(center, axis, color);
            }
            boxdd::DebugDrawCmd::SolidCapsule {
                p1,
                p2,
                radius,
                color,
            } => {
                let color = debug_color(*color);
                gizmos.line_2d(p1.to_bevy_vec2(), p2.to_bevy_vec2(), color);
                gizmos.circle_2d(p1.to_bevy_vec2(), *radius, color);
                gizmos.circle_2d(p2.to_bevy_vec2(), *radius, color);
            }
            boxdd::DebugDrawCmd::Segment { p1, p2, color } => {
                gizmos.line_2d(p1.to_bevy_vec2(), p2.to_bevy_vec2(), debug_color(*color));
            }
            boxdd::DebugDrawCmd::Transform(transform) => {
                let center = transform.position().to_bevy_vec2();
                let x_axis = transform
                    .transform_point(boxdd::Vec2::new(0.25, 0.0))
                    .to_bevy_vec2();
                let y_axis = transform
                    .transform_point(boxdd::Vec2::new(0.0, 0.25))
                    .to_bevy_vec2();
                gizmos.line_2d(center, x_axis, Color::srgb(1.0, 0.2, 0.2));
                gizmos.line_2d(center, y_axis, Color::srgb(0.2, 1.0, 0.2));
            }
            boxdd::DebugDrawCmd::Point { p, size, color } => {
                gizmos.circle_2d(p.to_bevy_vec2(), *size * 0.01, debug_color(*color));
            }
            boxdd::DebugDrawCmd::String { p, color, .. } => {
                gizmos.circle_2d(p.to_bevy_vec2(), 0.03, debug_color(*color));
            }
        }
    }
}

fn draw_loop(gizmos: &mut Gizmos, points: impl IntoIterator<Item = boxdd::Vec2>, color: Color) {
    let points = points
        .into_iter()
        .map(boxdd::Vec2::to_bevy_vec2)
        .collect::<Vec<_>>();
    for pair in points.windows(2) {
        gizmos.line_2d(pair[0], pair[1], color);
    }
    if let (Some(first), Some(last)) = (points.first(), points.last())
        && points.len() > 2
    {
        gizmos.line_2d(*last, *first, color);
    }
}

fn debug_color(color: boxdd::HexColor) -> Color {
    let rgb = color.rgb_u32();
    Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
