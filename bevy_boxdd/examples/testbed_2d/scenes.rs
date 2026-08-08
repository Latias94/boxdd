use bevy::prelude::*;
use bevy_boxdd::prelude::*;

pub use crate::scene_catalog::{SCENE_REGISTRY, TestbedScene, TestbedSceneMetadata};

#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
pub struct TestbedEntity;

#[derive(Component, Copy, Clone, Debug)]
pub struct KinematicOscillator {
    center: Vec2,
    amplitude: f32,
    speed: f32,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Spinner {
    speed: f32,
}

pub fn spawn_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    origin: &BoxddWorldOrigin,
    metadata: &TestbedSceneMetadata,
) {
    match metadata.scene {
        TestbedScene::SingleBox => spawn_single_box(commands, meshes, materials, origin),
        TestbedScene::TiltedStack => spawn_tilted_stack(commands, meshes, materials, origin),
        TestbedScene::CircleStack => spawn_circle_stack(commands, meshes, materials, origin),
        TestbedScene::Pyramid => spawn_pyramid(commands, meshes, materials, origin),
        TestbedScene::BodyType => spawn_body_type(commands, meshes, materials, origin),
        TestbedScene::KinematicPlatform => {
            spawn_kinematic_platform(commands, meshes, materials, origin);
        }
        TestbedScene::ContinuousBullet => {
            spawn_continuous_bullet(commands, meshes, materials, origin);
        }
        TestbedScene::Restitution => spawn_restitution(commands, meshes, materials, origin),
        TestbedScene::Friction => spawn_friction(commands, meshes, materials, origin),
        TestbedScene::ShapeFilter => spawn_shape_filter(commands, meshes, materials, origin),
        TestbedScene::SensorFunnel => spawn_sensor_funnel(commands, meshes, materials, origin),
        TestbedScene::ContactEvents => spawn_contact_events(commands, meshes, materials, origin),
        TestbedScene::DistanceBridge => spawn_distance_bridge(commands, meshes, materials, origin),
        TestbedScene::RevolutePendulum => {
            spawn_revolute_pendulum(commands, meshes, materials, origin);
        }
    }
}

pub fn animate_kinematic_platforms(
    time: Res<Time>,
    mut platforms: Query<(&KinematicOscillator, &mut Transform)>,
) {
    for (oscillator, mut transform) in &mut platforms {
        transform.translation.x = oscillator.center.x
            + oscillator.amplitude * (time.elapsed_secs() * oscillator.speed).sin();
        transform.translation.y = oscillator.center.y;
    }
}

pub fn animate_spinners(time: Res<Time>, mut spinners: Query<(&Spinner, &mut Transform)>) {
    for (spinner, mut transform) in &mut spinners {
        transform.rotation = Quat::from_rotation_z(time.elapsed_secs() * spinner.speed);
    }
}

pub fn draw_scene_overlays(
    state: Res<crate::control::TestbedState>,
    origin: Res<BoxddWorldOrigin>,
    joints: Query<&JointDescriptor>,
    transforms: Query<&Transform>,
    mut gizmos: Gizmos,
) {
    if !state.draw_overlays {
        return;
    }

    for descriptor in &joints {
        match descriptor.kind {
            JointKind::Distance(distance) => {
                let (Ok(anchor_a), Ok(anchor_b)) = (
                    origin.checked_absolute_to_local(distance.anchor_a),
                    origin.checked_absolute_to_local(distance.anchor_b),
                ) else {
                    continue;
                };
                gizmos.line_2d(anchor_a, anchor_b, Color::srgb(0.75, 0.82, 0.9));
            }
            JointKind::Revolute(revolute) => {
                let Ok(anchor) = origin.checked_absolute_to_local(revolute.anchor) else {
                    continue;
                };
                gizmos.circle_2d(anchor, 0.16, Color::srgb(0.95, 0.68, 0.25));
                if let Ok(transform) = transforms.get(descriptor.entity_b) {
                    gizmos.line_2d(
                        anchor,
                        transform.translation.truncate(),
                        Color::srgb(0.95, 0.68, 0.25),
                    );
                }
            }
        }
    }
}

fn spawn_single_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.0);
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::splat(0.5),
        Transform::from_xyz(-2.4, -1.7, 2.0),
        dynamic_material(0.35, 0.05),
        Color::srgb(0.93, 0.62, 0.25),
    )
    .insert(LinearVelocity(Vec2::new(4.5, 0.0)));
}

fn spawn_tilted_stack(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 18.0, -3.5);
    for column in 0..5 {
        for row in 0..7 {
            let x = -3.4 + column as f32 * 0.82 + row as f32 * 0.045;
            let y = -2.95 + row as f32 * 0.56;
            spawn_rounded_box(
                commands,
                meshes,
                materials,
                RoundedBoxShape {
                    half_extents: Vec2::splat(0.25),
                    radius: 0.04,
                },
                Transform::from_xyz(x, y, 2.0).with_rotation(Quat::from_rotation_z(0.03)),
                dynamic_material(0.45, 0.0),
                Color::srgb(0.24, 0.55, 0.88),
            );
        }
    }
}

fn spawn_circle_stack(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 18.0, -3.5);
    for column in 0..4 {
        for row in 0..7 {
            spawn_circle(
                commands,
                meshes,
                materials,
                0.26,
                Transform::from_xyz(-2.8 + column as f32 * 0.74, -2.95 + row as f32 * 0.58, 2.0),
                dynamic_material(0.65, 0.05),
                Color::srgb(0.9, 0.42, 0.32),
            );
        }
    }
}

fn spawn_pyramid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 18.0, -3.5);
    let rows = 10;
    for row in 0..rows {
        let width = rows - row;
        for column in 0..width {
            let x = (column as f32 - width as f32 * 0.5) * 0.54 + row as f32 * 0.27;
            let y = -3.0 + row as f32 * 0.52;
            spawn_box(
                commands,
                meshes,
                materials,
                Vec2::splat(0.24),
                Transform::from_xyz(x, y, 2.0),
                dynamic_material(0.55, 0.0),
                Color::srgb(0.34, 0.58, 0.92),
            );
        }
    }
}

fn spawn_body_type(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.4);
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(1.0, 0.18),
        Transform::from_xyz(-3.4, -1.4, 0.0).with_rotation(Quat::from_rotation_z(-0.25)),
        static_material(),
        Color::srgb(0.22, 0.42, 0.39),
    );
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(0.85, 0.12),
        Transform::from_xyz(0.0, -0.4, 0.0),
        static_material(),
        Color::srgb(0.5, 0.5, 0.58),
    )
    .insert((
        RigidBody::Kinematic,
        KinematicOscillator {
            center: Vec2::new(0.0, -0.4),
            amplitude: 2.1,
            speed: 1.1,
        },
        TransformSyncMode::BevyToPhysics,
    ));
    for i in 0..7 {
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::splat(0.28),
            Transform::from_xyz(-1.8 + i as f32 * 0.55, 1.6 + i as f32 * 0.2, 2.0),
            dynamic_material(0.55, 0.0),
            Color::srgb(0.42, 0.58, 0.95),
        );
    }
}

fn spawn_kinematic_platform(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.5);
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(1.35, 0.14),
        Transform::from_xyz(0.0, -1.25, 1.0),
        PhysicsMaterial {
            friction: 0.85,
            ..static_material()
        },
        Color::srgb(0.28, 0.68, 0.62),
    )
    .insert((
        RigidBody::Kinematic,
        KinematicOscillator {
            center: Vec2::new(0.0, -1.25),
            amplitude: 3.0,
            speed: 0.9,
        },
        TransformSyncMode::BevyToPhysics,
    ));
    for row in 0..5 {
        for column in 0..3 {
            spawn_box(
                commands,
                meshes,
                materials,
                Vec2::splat(0.26),
                Transform::from_xyz(-0.55 + column as f32 * 0.55, -0.65 + row as f32 * 0.56, 2.0),
                dynamic_material(0.55, 0.0),
                Color::srgb(0.94, 0.64, 0.26),
            );
        }
    }
}

fn spawn_continuous_bullet(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.5);
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(0.1, 2.35),
        Transform::from_xyz(3.2, -0.45, 0.0),
        PhysicsMaterial {
            enable_contact_events: true,
            enable_hit_events: true,
            ..static_material()
        },
        Color::srgb(0.7, 0.72, 0.78),
    );
    spawn_circle(
        commands,
        meshes,
        materials,
        0.22,
        Transform::from_xyz(-4.4, -0.35, 2.0),
        PhysicsMaterial {
            enable_contact_events: true,
            enable_hit_events: true,
            ..dynamic_material(0.25, 0.05)
        },
        Color::srgb(0.95, 0.36, 0.28),
    )
    .insert((BodySettings::bullet(), LinearVelocity(Vec2::new(32.0, 0.0))));
}

fn spawn_restitution(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    for (index, restitution) in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0].into_iter().enumerate() {
        let x = -4.2 + index as f32 * 1.65;
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::new(0.62, 0.12),
            Transform::from_xyz(x, -2.85, 0.0),
            PhysicsMaterial {
                restitution,
                ..static_material()
            },
            Color::srgb(0.25, 0.36, 0.48),
        );
        spawn_circle(
            commands,
            meshes,
            materials,
            0.3,
            Transform::from_xyz(x, 2.0, 2.0),
            dynamic_material(0.35, restitution),
            Color::srgb(0.3 + restitution * 0.55, 0.48, 0.92 - restitution * 0.35),
        );
    }
}

fn spawn_friction(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    for (index, friction) in [0.0, 0.2, 0.45, 0.75, 1.0].into_iter().enumerate() {
        let y = 0.8 - index as f32 * 1.0;
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::new(2.9, 0.1),
            Transform::from_xyz(0.0, y, 0.0).with_rotation(Quat::from_rotation_z(-0.22)),
            PhysicsMaterial {
                friction,
                ..static_material()
            },
            Color::srgb(0.34 + friction * 0.3, 0.43, 0.36),
        );
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::splat(0.23),
            Transform::from_xyz(-2.5, y + 0.45, 2.0),
            dynamic_material(friction, 0.0),
            Color::srgb(0.78, 0.66 - friction * 0.25, 0.32),
        )
        .insert(LinearVelocity(Vec2::new(2.0, 0.0)));
    }
}

fn spawn_shape_filter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.4);
    let red_filter = filter(0x0002, 0x0004 | 0x0008);
    let blue_filter = filter(0x0004, 0x0002 | 0x0008);
    let green_filter = filter(0x0008, 0x0002 | 0x0004 | 0x0008);
    for i in 0..6 {
        spawn_circle(
            commands,
            meshes,
            materials,
            0.28,
            Transform::from_xyz(-3.0 + i as f32 * 1.0, 1.9, 2.0),
            filtered_material(red_filter),
            Color::srgb(0.88, 0.32, 0.32),
        );
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::splat(0.28),
            Transform::from_xyz(-3.0 + i as f32 * 1.0, 0.7, 2.0),
            filtered_material(blue_filter),
            Color::srgb(0.32, 0.52, 0.9),
        );
        spawn_circle(
            commands,
            meshes,
            materials,
            0.22,
            Transform::from_xyz(-3.0 + i as f32 * 1.0, -0.5, 2.0),
            filtered_material(green_filter),
            Color::srgb(0.32, 0.75, 0.45),
        );
    }
}

fn spawn_sensor_funnel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.5);
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(1.8, 0.12),
        Transform::from_xyz(-1.8, -1.1, 0.0).with_rotation(Quat::from_rotation_z(-0.45)),
        static_material(),
        Color::srgb(0.3, 0.39, 0.42),
    );
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(1.8, 0.12),
        Transform::from_xyz(1.8, -1.1, 0.0).with_rotation(Quat::from_rotation_z(0.45)),
        static_material(),
        Color::srgb(0.3, 0.39, 0.42),
    );
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(1.0, 1.0),
        Transform::from_xyz(0.0, -2.1, 1.0),
        PhysicsMaterial {
            density: 0.0,
            is_sensor: true,
            enable_sensor_events: true,
            ..Default::default()
        },
        Color::srgba(0.25, 0.64, 0.9, 0.28),
    );
    for i in 0..16 {
        spawn_circle(
            commands,
            meshes,
            materials,
            0.18,
            Transform::from_xyz(
                -1.8 + (i % 6) as f32 * 0.72,
                1.8 + (i / 6) as f32 * 0.52,
                2.0,
            ),
            dynamic_material(0.4, 0.1),
            Color::srgb(0.95, 0.66, 0.25),
        );
    }
}

fn spawn_contact_events(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    _origin: &BoxddWorldOrigin,
) {
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(6.5, 0.18),
        Transform::from_xyz(0.0, -3.2, 0.0),
        PhysicsMaterial {
            enable_contact_events: true,
            enable_hit_events: true,
            ..static_material()
        },
        Color::srgb(0.26, 0.36, 0.32),
    );
    for i in 0..10 {
        let x = -3.0 + i as f32 * 0.66;
        let y = 1.4 + (i % 3) as f32 * 0.55;
        spawn_box(
            commands,
            meshes,
            materials,
            Vec2::splat(0.28),
            Transform::from_xyz(x, y, 2.0),
            PhysicsMaterial {
                enable_contact_events: true,
                enable_hit_events: true,
                ..dynamic_material(0.45, 0.2)
            },
            Color::srgb(0.82, 0.36, 0.34),
        );
    }
}

fn spawn_distance_bridge(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.7);
    let plank_count = 12;
    let spacing = 0.58;
    let y = -1.2;
    let left_x = -((plank_count as f32 + 1.0) * spacing) * 0.5;
    let right_x = -left_x;
    let left_anchor = spawn_anchor(commands, meshes, materials, left_x, y);
    let right_anchor = spawn_anchor(commands, meshes, materials, right_x, y);

    let mut previous = left_anchor;
    let mut previous_anchor = world_position(origin, left_x, y);
    for index in 0..plank_count {
        let x = left_x + (index as f32 + 1.0) * spacing;
        let plank = spawn_box(
            commands,
            meshes,
            materials,
            Vec2::new(0.25, 0.07),
            Transform::from_xyz(x, y, 2.0),
            dynamic_material(0.75, 0.0),
            Color::srgb(0.53, 0.42, 0.31),
        )
        .id();
        let anchor = world_position(origin, x, y);
        commands.spawn((
            TestbedEntity,
            JointDescriptor::distance(previous, plank, previous_anchor, anchor)
                .with_constraint_tuning(4.0, 0.75),
        ));
        previous = plank;
        previous_anchor = anchor;
    }
    commands.spawn((
        TestbedEntity,
        JointDescriptor::distance(
            previous,
            right_anchor,
            previous_anchor,
            world_position(origin, right_x, y),
        )
        .with_constraint_tuning(4.0, 0.75),
    ));

    spawn_circle(
        commands,
        meshes,
        materials,
        0.45,
        Transform::from_xyz(0.0, 1.6, 2.0),
        dynamic_material(0.4, 0.0),
        Color::srgb(0.85, 0.35, 0.25),
    );
}

fn spawn_revolute_pendulum(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    origin: &BoxddWorldOrigin,
) {
    spawn_floor(commands, meshes, materials, 16.0, -3.5);
    let hinge = spawn_anchor(commands, meshes, materials, -2.4, 1.7);
    let pendulum = spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(0.13, 1.25),
        Transform::from_xyz(-2.4, 0.45, 2.0),
        dynamic_material(0.45, 0.0),
        Color::srgb(0.88, 0.58, 0.26),
    )
    .insert(AngularImpulse::new(2.4))
    .id();
    commands.spawn((
        TestbedEntity,
        JointDescriptor::revolute(hinge, pendulum, world_position(origin, -2.4, 1.7)),
    ));
    for row in 0..5 {
        for column in 0..4 {
            spawn_box(
                commands,
                meshes,
                materials,
                Vec2::splat(0.24),
                Transform::from_xyz(1.0 + column as f32 * 0.5, -2.95 + row as f32 * 0.5, 2.0),
                dynamic_material(0.55, 0.0),
                Color::srgb(0.36, 0.62, 0.9),
            );
        }
    }
}

fn spawn_floor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    half_width: f32,
    y: f32,
) {
    spawn_box(
        commands,
        meshes,
        materials,
        Vec2::new(half_width, 0.16),
        Transform::from_xyz(0.0, y, 0.0),
        static_material(),
        Color::srgb(0.2, 0.31, 0.32),
    );
}

fn world_position(origin: &BoxddWorldOrigin, x: f32, y: f32) -> boxdd::Position {
    origin
        .checked_local_to_absolute(Vec2::new(x, y))
        .expect("testbed joint anchor must be representable")
}

fn spawn_anchor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    x: f32,
    y: f32,
) -> Entity {
    spawn_circle(
        commands,
        meshes,
        materials,
        0.12,
        Transform::from_xyz(x, y, 1.0),
        static_material(),
        Color::srgb(0.78, 0.8, 0.86),
    )
    .id()
}

fn spawn_box<'a>(
    commands: &'a mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    half_extents: Vec2,
    transform: Transform,
    material: PhysicsMaterial,
    color: Color,
) -> EntityCommands<'a> {
    commands.spawn((
        TestbedEntity,
        rigid_body_for(material),
        Collider::rectangle(half_extents.x, half_extents.y),
        material,
        Mesh2d(meshes.add(Rectangle::new(half_extents.x * 2.0, half_extents.y * 2.0))),
        MeshMaterial2d(materials.add(color)),
        transform,
    ))
}

struct RoundedBoxShape {
    half_extents: Vec2,
    radius: f32,
}

fn spawn_rounded_box<'a>(
    commands: &'a mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    shape: RoundedBoxShape,
    transform: Transform,
    material: PhysicsMaterial,
    color: Color,
) -> EntityCommands<'a> {
    commands.spawn((
        TestbedEntity,
        rigid_body_for(material),
        Collider::rounded_rectangle(shape.half_extents.x, shape.half_extents.y, shape.radius),
        material,
        Mesh2d(meshes.add(Rectangle::new(
            shape.half_extents.x * 2.0,
            shape.half_extents.y * 2.0,
        ))),
        MeshMaterial2d(materials.add(color)),
        transform,
    ))
}

fn spawn_circle<'a>(
    commands: &'a mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    radius: f32,
    transform: Transform,
    material: PhysicsMaterial,
    color: Color,
) -> EntityCommands<'a> {
    commands.spawn((
        TestbedEntity,
        rigid_body_for(material),
        Collider::circle(radius),
        material,
        Mesh2d(meshes.add(Circle::new(radius))),
        MeshMaterial2d(materials.add(color)),
        transform,
    ))
}

fn static_material() -> PhysicsMaterial {
    PhysicsMaterial {
        density: 0.0,
        friction: 0.65,
        restitution: 0.0,
        ..Default::default()
    }
}

fn dynamic_material(friction: f32, restitution: f32) -> PhysicsMaterial {
    PhysicsMaterial {
        density: 1.0,
        friction,
        restitution,
        ..Default::default()
    }
}

fn filtered_material(filter: boxdd::Filter) -> PhysicsMaterial {
    PhysicsMaterial {
        filter,
        ..dynamic_material(0.45, 0.0)
    }
}

fn filter(category_bits: u64, mask_bits: u64) -> boxdd::Filter {
    boxdd::Filter {
        category_bits,
        mask_bits,
        group_index: 0,
    }
}

fn rigid_body_for(material: PhysicsMaterial) -> RigidBody {
    if material.density == 0.0 {
        RigidBody::Static
    } else {
        RigidBody::Dynamic
    }
}
