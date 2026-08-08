use bevy::ecs::system::NonSendMut;
use bevy::log::warn;
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
        .add_systems(Update, draw_boxdd_gizmos)
        .run();
}

fn setup(mut commands: Commands, origin: Res<BoxddWorldOrigin>) {
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
        JointDescriptor::distance(
            left,
            right,
            origin
                .checked_local_to_absolute(Vec2::new(-1.6, 0.4))
                .expect("debug joint anchor must be representable"),
            origin
                .checked_local_to_absolute(Vec2::new(-0.9, 0.4))
                .expect("debug joint anchor must be representable"),
        )
        .with_constraint_tuning(4.0, 0.7),
    );
}

fn draw_boxdd_gizmos(
    mut context: NonSendMut<BoxddPhysicsContext>,
    origin: Res<BoxddWorldOrigin>,
    mut commands: Local<Vec<boxdd::DebugDrawCmd>>,
    mut gizmos: Gizmos,
) {
    if let Err(error) =
        context.debug_draw_collect_into(&mut commands, boxdd::DebugDrawOptions::default())
    {
        warn!(?error, "debug draw collection failed");
        return;
    }

    for command in commands.iter() {
        match command {
            boxdd::DebugDrawCmd::Polygon {
                transform,
                vertices,
                color,
            } => {
                draw_world_loop(
                    &mut gizmos,
                    &origin,
                    vertices
                        .iter()
                        .copied()
                        .map(|point| transform.transform_point(point)),
                    debug_color(*color),
                );
            }
            boxdd::DebugDrawCmd::SolidPolygon {
                transform,
                vertices,
                color,
                ..
            } => {
                draw_world_loop(
                    &mut gizmos,
                    &origin,
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
                if let Ok(center) = origin.checked_absolute_to_local(*center) {
                    gizmos.circle_2d(center, *radius, debug_color(*color));
                }
            }
            boxdd::DebugDrawCmd::SolidCircle {
                transform,
                center,
                radius,
                color,
            } => {
                let local_center = *center;
                let world_center = transform.transform_point(local_center);
                let Ok(bevy_center) = origin.checked_absolute_to_local(world_center) else {
                    continue;
                };
                let axis = transform
                    .transform_point(boxdd::Vec2::new(local_center.x + *radius, local_center.y));
                let Ok(axis) = origin.checked_absolute_to_local(axis) else {
                    continue;
                };
                let color = debug_color(*color);
                gizmos.circle_2d(bevy_center, *radius, color);
                gizmos.line_2d(bevy_center, axis, color);
            }
            boxdd::DebugDrawCmd::SolidCapsule {
                p1,
                p2,
                radius,
                color,
            } => {
                let (Ok(p1), Ok(p2)) = (
                    origin.checked_absolute_to_local(*p1),
                    origin.checked_absolute_to_local(*p2),
                ) else {
                    continue;
                };
                let color = debug_color(*color);
                gizmos.line_2d(p1, p2, color);
                gizmos.circle_2d(p1, *radius, color);
                gizmos.circle_2d(p2, *radius, color);
            }
            boxdd::DebugDrawCmd::Segment { p1, p2, color } => {
                let (Ok(p1), Ok(p2)) = (
                    origin.checked_absolute_to_local(*p1),
                    origin.checked_absolute_to_local(*p2),
                ) else {
                    continue;
                };
                gizmos.line_2d(p1, p2, debug_color(*color));
            }
            boxdd::DebugDrawCmd::Transform(transform) => {
                let Ok(center) = origin.checked_absolute_to_local(transform.position()) else {
                    continue;
                };
                let x_axis = transform.transform_point(boxdd::Vec2::new(0.25, 0.0));
                let y_axis = transform.transform_point(boxdd::Vec2::new(0.0, 0.25));
                let (Ok(x_axis), Ok(y_axis)) = (
                    origin.checked_absolute_to_local(x_axis),
                    origin.checked_absolute_to_local(y_axis),
                ) else {
                    continue;
                };
                gizmos.line_2d(center, x_axis, Color::srgb(1.0, 0.2, 0.2));
                gizmos.line_2d(center, y_axis, Color::srgb(0.2, 1.0, 0.2));
            }
            boxdd::DebugDrawCmd::Point { p, size, color } => {
                if let Ok(p) = origin.checked_absolute_to_local(*p) {
                    gizmos.circle_2d(p, *size * 0.01, debug_color(*color));
                }
            }
            boxdd::DebugDrawCmd::String { p, color, .. } => {
                if let Ok(p) = origin.checked_absolute_to_local(*p) {
                    gizmos.circle_2d(p, 0.03, debug_color(*color));
                }
            }
            boxdd::DebugDrawCmd::Bounds { bounds, color } => {
                let Ok(corners) = absolute_bounds_to_local_corners(&origin, *bounds) else {
                    continue;
                };
                draw_loop(&mut gizmos, corners, debug_color(*color));
            }
        }
    }
}

fn draw_world_loop(
    gizmos: &mut Gizmos,
    origin: &BoxddWorldOrigin,
    points: impl IntoIterator<Item = boxdd::Position>,
    color: Color,
) {
    let Ok(points) = points
        .into_iter()
        .map(|point| origin.checked_absolute_to_local(point))
        .collect::<Result<Vec<_>, _>>()
    else {
        return;
    };
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return;
    };

    let mut last = first;
    let mut count = 1usize;
    for point in points {
        gizmos.line_2d(last, point, color);
        last = point;
        count += 1;
    }
    if count > 2 {
        gizmos.line_2d(last, first, color);
    }
}

fn absolute_bounds_to_local_corners(
    origin: &BoxddWorldOrigin,
    bounds: boxdd::Aabb,
) -> Result<[Vec2; 4], BoxddWorldOriginError> {
    let world_corners = [
        bounds.lower(),
        boxdd::Vec2::new(bounds.upper().x, bounds.lower().y),
        bounds.upper(),
        boxdd::Vec2::new(bounds.lower().x, bounds.upper().y),
    ];

    Ok([
        origin.checked_absolute_to_local(world_corners[0].into())?,
        origin.checked_absolute_to_local(world_corners[1].into())?,
        origin.checked_absolute_to_local(world_corners[2].into())?,
        origin.checked_absolute_to_local(world_corners[3].into())?,
    ])
}

fn draw_loop(gizmos: &mut Gizmos, points: impl IntoIterator<Item = Vec2>, color: Color) {
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return;
    };

    let mut last = first;
    let mut count = 1usize;
    for point in points {
        gizmos.line_2d(last, point, color);
        last = point;
        count += 1;
    }

    if count > 2 {
        gizmos.line_2d(last, first, color);
    }
}

fn debug_color(color: boxdd::HexColor) -> Color {
    let rgb = color.rgb_u32();
    Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_bounds_are_mapped_into_a_nonzero_local_origin() {
        let origin =
            BoxddWorldOrigin::new(boxdd::Position::from([1_000_000.0_f32, -2_000_000.0])).unwrap();
        let bounds =
            boxdd::Aabb::new([1_000_002.0_f32, -1_999_997.0], [1_000_006.0, -1_999_992.0]).unwrap();

        assert_eq!(
            absolute_bounds_to_local_corners(&origin, bounds).unwrap(),
            [
                Vec2::new(2.0, 3.0),
                Vec2::new(6.0, 3.0),
                Vec2::new(6.0, 8.0),
                Vec2::new(2.0, 8.0),
            ]
        );
    }
}
