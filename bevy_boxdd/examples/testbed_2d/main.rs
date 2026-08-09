mod control;
mod scene_catalog;
mod scenes;
mod support;
mod ui;

use std::fmt::Write as _;

use bevy::camera::ScalingMode;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy::time::Fixed;
use bevy_boxdd::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use control::{EventStats, TestbedState};
use scenes::{SCENE_REGISTRY, TestbedEntity, TestbedScene, TestbedSceneMetadata, spawn_scene};

#[derive(Component, Debug)]
pub(crate) struct TestbedCamera;

#[derive(Copy, Clone, Debug)]
struct TestbedLaunch {
    scene: TestbedScene,
    scene_switching_enabled: bool,
}

impl Default for TestbedLaunch {
    fn default() -> Self {
        Self {
            scene: TestbedScene::SingleBox,
            scene_switching_enabled: true,
        }
    }
}

impl TestbedState {
    fn scene_metadata(&self) -> &'static TestbedSceneMetadata {
        SCENE_REGISTRY
            .get(self.scene_index)
            .expect("testbed scene index must refer to SCENE_REGISTRY")
    }
}

fn main() {
    let foundation =
        boxdd::Foundation::initialize_default().expect("Box2D foundation should initialize");
    let launch = TestbedLaunch::from_environment();
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.045, 0.055)))
        .insert_resource(TestbedState::launch(
            launch.scene.index(),
            launch.scene_switching_enabled,
        ))
        .insert_resource(EventStats::default())
        .add_plugins(support::teaching_default_plugins("boxdd Bevy Testbed"))
        .add_plugins(EguiPlugin::default())
        .add_plugins(BoxddPhysicsPlugin::new(
            foundation,
            BoxddPhysicsSettings {
                event_interests: BoxddEventInterests::ALL,
                ..Default::default()
            },
        ))
        .add_systems(First, prepare_time_control)
        .add_systems(Startup, (setup_view, spawn_initial_scene).chain())
        .add_systems(EguiPrimaryContextPass, ui::draw_testbed_ui)
        .add_systems(
            Update,
            (
                handle_input,
                apply_testbed_settings,
                scenes::animate_kinematic_platforms,
                scenes::animate_spinners,
                scenes::draw_scene_overlays,
                update_event_counters,
            ),
        )
        .add_systems(PostUpdate, finish_single_step)
        .run();
}

impl TestbedLaunch {
    fn from_environment() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self::from_browser()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::from_args(std::env::args().skip(1))
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn from_browser() -> Self {
        let Some(location) = web_sys::window().map(|window| window.location()) else {
            return Self::default();
        };

        let scene_id = location
            .search()
            .ok()
            .and_then(|search| query_param(&search, "scene"))
            .or_else(|| {
                location
                    .pathname()
                    .ok()
                    .and_then(|path| scene_id_from_path(&path).map(ToOwned::to_owned))
            });

        scene_id
            .as_deref()
            .and_then(TestbedScene::from_id)
            .map(|scene| Self {
                scene,
                scene_switching_enabled: false,
            })
            .unwrap_or_default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut args = args.into_iter();
        let mut scene_id = None;
        let mut scene_switching_enabled = true;

        while let Some(arg) = args.next() {
            if arg == "--single-scene" {
                scene_switching_enabled = false;
            } else if arg == "--testbed" {
                scene_switching_enabled = true;
            } else if arg == "--scene" {
                scene_id = args.next();
                scene_switching_enabled = false;
            } else if let Some(value) = arg.strip_prefix("--scene=") {
                scene_id = Some(value.to_string());
                scene_switching_enabled = false;
            }
        }

        scene_id
            .as_deref()
            .and_then(TestbedScene::from_id)
            .map(|scene| Self {
                scene,
                scene_switching_enabled,
            })
            .unwrap_or_default()
    }
}

#[cfg(target_arch = "wasm32")]
fn query_param(search: &str, key: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|part| part.split_once('='))
        .find_map(|(name, value)| (name == key && !value.is_empty()).then(|| value.to_string()))
}

#[cfg(target_arch = "wasm32")]
fn scene_id_from_path(path: &str) -> Option<&str> {
    let mut parts = path.split('/');
    while let Some(part) = parts.next() {
        if part == "examples" {
            return parts.next().filter(|scene| !scene.is_empty());
        }
    }
    None
}

fn setup_view(mut commands: Commands) {
    commands.spawn((
        TestbedCamera,
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 9.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, -0.85, 0.0),
    ));
}

fn spawn_initial_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    state: Res<TestbedState>,
    origin: Res<BoxddWorldOrigin>,
) {
    let metadata = state.scene_metadata();
    log_scene_selection(metadata);
    spawn_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &origin,
        metadata,
    );
}

#[allow(clippy::too_many_arguments)]
fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<TestbedState>,
    mut commands: Commands,
    entities: Query<Entity, With<TestbedEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut stats: ResMut<EventStats>,
    origin: Res<BoxddWorldOrigin>,
) {
    let mut requested_scene = None;

    if state.scene_switching_enabled {
        for (index, key) in [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
            KeyCode::Digit0,
        ]
        .into_iter()
        .enumerate()
        {
            if keys.just_pressed(key) {
                requested_scene = Some(index);
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyR) {
        requested_scene = Some(state.scene_index);
    }
    if keys.just_pressed(KeyCode::KeyO) {
        state.draw_overlays = !state.draw_overlays;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        state.gravity_enabled = !state.gravity_enabled;
    }
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
        if !state.paused {
            state.cancel_single_step();
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        state.request_single_step();
    }

    let Some(scene_index) = requested_scene else {
        return;
    };

    switch_scene(
        scene_index,
        &mut state,
        &mut commands,
        &entities,
        &mut meshes,
        &mut materials,
        &mut stats,
        &origin,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn switch_scene(
    scene_index: usize,
    state: &mut TestbedState,
    commands: &mut Commands,
    entities: &Query<Entity, With<TestbedEntity>>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    stats: &mut EventStats,
    origin: &BoxddWorldOrigin,
) {
    for entity in entities {
        commands.entity(entity).despawn();
    }
    state.scene_index = scene_index.min(SCENE_REGISTRY.len() - 1);
    *stats = EventStats::default();
    let metadata = state.scene_metadata();
    log_scene_selection(metadata);
    spawn_scene(commands, meshes, materials, origin, metadata);
}

fn log_scene_selection(metadata: &TestbedSceneMetadata) {
    if !log::log_enabled!(log::Level::Info) {
        return;
    }

    let mut upstream = String::new();
    for (index, sample) in metadata.upstream.iter().enumerate() {
        if index > 0 {
            upstream.push_str(", ");
        }
        write!(
            upstream,
            "{}/{}:{}",
            sample.category,
            sample.name,
            sample.style.as_str()
        )
        .expect("writing to String cannot fail");
    }
    bevy::log::info!(
        "Testbed scene [{}] {} ({}) - {}; upstream: {}",
        metadata.category,
        metadata.name,
        metadata.id,
        metadata.description,
        upstream
    );
}

fn prepare_time_control(mut state: ResMut<TestbedState>, mut time: ResMut<Time<Virtual>>) {
    state.clamp_controls();
    if !state.paused {
        state.cancel_single_step();
        time.unpause();
        return;
    }

    if state.single_step_pending {
        state.single_step_pending = false;
        state.single_step_active = true;
        time.unpause();
    } else {
        time.pause();
    }
}

fn finish_single_step(mut state: ResMut<TestbedState>, mut time: ResMut<Time<Virtual>>) {
    if state.single_step_active {
        state.single_step_active = false;
        time.pause();
    }
}

fn apply_testbed_settings(
    mut state: ResMut<TestbedState>,
    mut step_settings: ResMut<BoxddStepSettings>,
    mut fixed_time: ResMut<Time<Fixed>>,
    mut context: NonSendMut<BoxddPhysicsContext>,
) {
    state.clamp_controls();

    let gravity = if state.gravity_enabled {
        Vec2::new(0.0, -10.0)
    } else {
        Vec2::ZERO
    };
    step_settings.sub_step_count = state.sub_step_count;
    step_settings.fallback_timestep_seconds = state.fixed_timestep_seconds() as f32;
    fixed_time.set_timestep_hz(state.hertz);

    let _ = context.set_gravity(gravity);
    let _ = context.enable_sleeping(state.sleeping_enabled);
    let _ = context.enable_warm_starting(state.warm_starting_enabled);
    let _ = context.enable_continuous(state.continuous_enabled);
}

fn update_event_counters(
    mut stats: ResMut<EventStats>,
    mut contact_begin: MessageReader<BoxddContactBeginMessage>,
    mut contact_end: MessageReader<BoxddContactEndMessage>,
    mut contact_hit: MessageReader<BoxddContactHitMessage>,
    mut joint_events: MessageReader<BoxddJointEventMessage>,
    mut sensor_begin: MessageReader<BoxddSensorBeginMessage>,
    mut sensor_end: MessageReader<BoxddSensorEndMessage>,
) {
    stats.contact_begin_frame = contact_begin.read().count() as u32;
    stats.contact_end_frame = contact_end.read().count() as u32;
    stats.contact_hit_frame = contact_hit.read().count() as u32;
    stats.joint_frame = joint_events.read().count() as u32;
    stats.sensor_begin_frame = sensor_begin.read().count() as u32;
    stats.sensor_end_frame = sensor_end.read().count() as u32;
    stats.contact_begin_total += u64::from(stats.contact_begin_frame);
    stats.contact_end_total += u64::from(stats.contact_end_frame);
    stats.contact_hit_total += u64::from(stats.contact_hit_frame);
    stats.joint_total += u64::from(stats.joint_frame);
    stats.sensor_begin_total += u64::from(stats.sensor_begin_frame);
    stats.sensor_end_total += u64::from(stats.sensor_end_frame);
}
