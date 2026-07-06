use bevy::prelude::*;
use bevy_boxdd::prelude::*;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TestbedScene {
    SingleBox,
    TiltedStack,
    CircleStack,
    Pyramid,
    BodyType,
    KinematicPlatform,
    ContinuousBullet,
    Restitution,
    Friction,
    ShapeFilter,
    SensorFunnel,
    ContactEvents,
    DistanceBridge,
    RevolutePendulum,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParityMode {
    TeachingAdaptation,
}

impl ParityMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TeachingAdaptation => "TeachingAdaptation",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UpstreamSampleRef {
    pub category: &'static str,
    pub name: &'static str,
    pub mode: ParityMode,
}

#[derive(Copy, Clone)]
pub struct TestbedSceneMetadata {
    pub scene: TestbedScene,
    pub id: &'static str,
    pub category: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub upstream: &'static [UpstreamSampleRef],
    spawn: fn(&mut Commands, &mut Assets<Mesh>, &mut Assets<ColorMaterial>),
}

impl TestbedSceneMetadata {
    pub const fn source_label(self) -> &'static str {
        "official Box2D sample"
    }
}

impl TestbedScene {
    pub fn metadata(self) -> &'static TestbedSceneMetadata {
        SCENE_REGISTRY
            .iter()
            .find(|metadata| metadata.scene == self)
            .expect("testbed scene metadata missing")
    }

    pub fn from_id(id: &str) -> Option<Self> {
        SCENE_REGISTRY
            .iter()
            .find(|metadata| metadata.id == id)
            .map(|metadata| metadata.scene)
    }

    pub fn index(self) -> usize {
        ALL_SCENES
            .iter()
            .position(|scene| *scene == self)
            .expect("testbed scene missing from ALL_SCENES")
    }
}

pub const ALL_SCENES: [TestbedScene; 14] = [
    TestbedScene::SingleBox,
    TestbedScene::TiltedStack,
    TestbedScene::CircleStack,
    TestbedScene::Pyramid,
    TestbedScene::BodyType,
    TestbedScene::KinematicPlatform,
    TestbedScene::ContinuousBullet,
    TestbedScene::Restitution,
    TestbedScene::Friction,
    TestbedScene::ShapeFilter,
    TestbedScene::SensorFunnel,
    TestbedScene::ContactEvents,
    TestbedScene::DistanceBridge,
    TestbedScene::RevolutePendulum,
];

pub const SCENE_REGISTRY: [TestbedSceneMetadata; 14] = [
    TestbedSceneMetadata {
        scene: TestbedScene::SingleBox,
        id: "single-box",
        category: "Stacking",
        name: "Single Box",
        description: "A dynamic box starts with horizontal velocity and settles on a long static segment.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Single Box",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_single_box,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::TiltedStack,
        id: "tilted-stack",
        category: "Stacking",
        name: "Tilted Stack",
        description: "Offset columns of rounded boxes show solver stability under uneven stacking pressure.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Tilted Stack",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_tilted_stack,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::CircleStack,
        id: "circle-stack",
        category: "Stacking",
        name: "Circle Stack",
        description: "Dynamic circles stack and roll through the same contact solver path as the official sample.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Circle Stack",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_circle_stack,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Pyramid,
        id: "pyramid",
        category: "Benchmark",
        name: "Large Pyramid",
        description: "A browser-sized version of the classic Box2D pyramid solver stress sample.",
        upstream: &[UpstreamSampleRef {
            category: "Benchmark",
            name: "Large Pyramid",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_pyramid,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::BodyType,
        id: "body-type",
        category: "Bodies",
        name: "Body Type",
        description: "Static, kinematic, and dynamic bodies share one scene so body behavior is visible.",
        upstream: &[UpstreamSampleRef {
            category: "Bodies",
            name: "Body Type",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_body_type,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::KinematicPlatform,
        id: "kinematic-platform",
        category: "Bodies",
        name: "Kinematic",
        description: "An app-controlled platform drives dynamic boxes through Bevy-to-Box2D transform sync.",
        upstream: &[UpstreamSampleRef {
            category: "Bodies",
            name: "Kinematic",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_kinematic_platform,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ContinuousBullet,
        id: "continuous-bullet",
        category: "Continuous",
        name: "Skinny Box",
        description: "A fast bullet body targets a thin wall with continuous collision enabled.",
        upstream: &[UpstreamSampleRef {
            category: "Continuous",
            name: "Skinny Box",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_continuous_bullet,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Restitution,
        id: "restitution",
        category: "Shapes",
        name: "Restitution",
        description: "Identical circles fall onto pads with increasing restitution values.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Restitution",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_restitution,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Friction,
        id: "friction",
        category: "Shapes",
        name: "Friction",
        description: "Boxes slide across ramps and floors with different friction coefficients.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Friction",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_friction,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ShapeFilter,
        id: "shape-filter",
        category: "Shapes",
        name: "Filter",
        description: "Category and mask bits split bodies into groups that collide or pass through each other.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Filter",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_shape_filter,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::SensorFunnel,
        id: "sensor-funnel",
        category: "Events",
        name: "Sensor Funnel",
        description: "Falling visitors pass through a transparent sensor and update the egui counters.",
        upstream: &[UpstreamSampleRef {
            category: "Events",
            name: "Sensor Funnel",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_sensor_funnel,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ContactEvents,
        id: "contact-event",
        category: "Events",
        name: "Contact",
        description: "Contact begin, end, and hit events are enabled on dynamic bodies and reflected in the panel.",
        upstream: &[UpstreamSampleRef {
            category: "Events",
            name: "Contact",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_contact_events,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::DistanceBridge,
        id: "bridge",
        category: "Joints",
        name: "Bridge",
        description: "Distance joints connect planks into a bridge and a dropped weight disturbs the chain.",
        upstream: &[
            UpstreamSampleRef {
                category: "Joints",
                name: "Distance Joint",
                mode: ParityMode::TeachingAdaptation,
            },
            UpstreamSampleRef {
                category: "Joints",
                name: "Bridge",
                mode: ParityMode::TeachingAdaptation,
            },
        ],
        spawn: spawn_distance_bridge,
    },
    TestbedSceneMetadata {
        scene: TestbedScene::RevolutePendulum,
        id: "revolute",
        category: "Joints",
        name: "Revolute",
        description: "A revolute joint creates a pendulum that strikes a small stack.",
        upstream: &[UpstreamSampleRef {
            category: "Joints",
            name: "Revolute",
            mode: ParityMode::TeachingAdaptation,
        }],
        spawn: spawn_revolute_pendulum,
    },
];

pub fn spawn_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    scene: TestbedScene,
) {
    (scene.metadata().spawn)(commands, meshes, materials);
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
                gizmos.line_2d(
                    distance.anchor_a,
                    distance.anchor_b,
                    Color::srgb(0.75, 0.82, 0.9),
                );
            }
            JointKind::Revolute(revolute) => {
                gizmos.circle_2d(revolute.anchor, 0.16, Color::srgb(0.95, 0.68, 0.25));
                if let Ok(transform) = transforms.get(descriptor.entity_b) {
                    gizmos.line_2d(
                        revolute.anchor,
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
    let mut previous_anchor = Vec2::new(left_x, y);
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
        let anchor = Vec2::new(x, y);
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
            Vec2::new(right_x, y),
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
        JointDescriptor::revolute(hinge, pendulum, Vec2::new(-2.4, 1.7)),
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
