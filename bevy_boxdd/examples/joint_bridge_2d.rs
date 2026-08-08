use bevy::prelude::*;
use bevy_boxdd::prelude::*;

const PLANK_COUNT: usize = 8;
const PLANK_SPACING: f32 = 0.75;
const BRIDGE_Y: f32 = 0.0;

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
        .run();
}

fn setup(mut commands: Commands, origin: Res<BoxddWorldOrigin>) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(6.0, 0.25),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    let left_anchor_x = -((PLANK_COUNT as f32 + 1.0) * PLANK_SPACING) * 0.5;
    let right_anchor_x = -left_anchor_x;
    let left_anchor = spawn_anchor(&mut commands, left_anchor_x);
    let right_anchor = spawn_anchor(&mut commands, right_anchor_x);

    let mut previous_entity = left_anchor;
    let mut previous_anchor = world_position(&origin, left_anchor_x, BRIDGE_Y);

    for index in 0..PLANK_COUNT {
        let x = left_anchor_x + (index as f32 + 1.0) * PLANK_SPACING;
        let plank = commands
            .spawn((
                RigidBody::Dynamic,
                Collider::rectangle(0.32, 0.08),
                PhysicsMaterial {
                    density: 1.2,
                    friction: 0.8,
                    ..Default::default()
                },
                Transform::from_xyz(x, BRIDGE_Y, 0.0),
            ))
            .id();

        let anchor = world_position(&origin, x, BRIDGE_Y);
        commands.spawn(
            JointDescriptor::distance(previous_entity, plank, previous_anchor, anchor)
                .with_constraint_tuning(4.0, 0.7),
        );

        previous_entity = plank;
        previous_anchor = anchor;
    }

    commands.spawn(
        JointDescriptor::distance(
            previous_entity,
            right_anchor,
            previous_anchor,
            world_position(&origin, right_anchor_x, BRIDGE_Y),
        )
        .with_constraint_tuning(4.0, 0.7),
    );

    let hinge = commands
        .spawn((RigidBody::Static, Transform::from_xyz(0.0, 1.35, 0.0)))
        .id();
    let pendulum = commands
        .spawn((
            RigidBody::Dynamic,
            Collider::rectangle(0.12, 0.65),
            LinearImpulse::new(Vec2::new(0.6, 0.0)),
            Transform::from_xyz(0.0, 0.7, 0.0),
        ))
        .id();
    commands.spawn(JointDescriptor::revolute(
        hinge,
        pendulum,
        world_position(&origin, 0.0, 1.35),
    ));
}

fn world_position(origin: &BoxddWorldOrigin, x: f32, y: f32) -> boxdd::Position {
    origin
        .checked_local_to_absolute(Vec2::new(x, y))
        .expect("joint anchor must be representable")
}

fn spawn_anchor(commands: &mut Commands, x: f32) -> Entity {
    commands
        .spawn((
            RigidBody::Static,
            Collider::circle(0.12),
            Transform::from_xyz(x, BRIDGE_Y, 0.0),
        ))
        .id()
}
