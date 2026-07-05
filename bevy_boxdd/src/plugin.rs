//! Bevy plugin wiring for `boxdd` fixed-step physics.

use crate::messages::{
    BoxddBodyMoveMessage, BoxddContactBeginMessage, BoxddContactEndMessage, BoxddContactHitMessage,
    BoxddErrorMessage, BoxddOperation, BoxddPluginError, BoxddSensorBeginMessage,
    BoxddSensorEndMessage,
};
use crate::resources::{BoxddErrorPolicy, BoxddPhysicsContext, BoxddPhysicsSettings};
use crate::systems::{
    apply_body_controls, apply_body_settings, cleanup_removed_bodies, cleanup_removed_colliders,
    create_missing_bodies, create_missing_shapes, publish_physics_messages, step_world,
    sync_bevy_transforms_to_boxdd, sync_boxdd_transforms_to_bevy,
};
use bevy_app::{App, FixedUpdate, Plugin};
use bevy_ecs::schedule::{ApplyDeferred, IntoScheduleConfigs};
use bevy_time::{Fixed, Time};

/// Plugin that owns the Box2D world and registers fixed-step physics systems.
#[derive(Clone, Debug, Default)]
pub struct BoxddPhysicsPlugin {
    settings: BoxddPhysicsSettings,
}

impl BoxddPhysicsPlugin {
    /// Creates the plugin with custom physics settings.
    pub fn new(settings: BoxddPhysicsSettings) -> Self {
        Self { settings }
    }
}

impl Plugin for BoxddPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BoxddErrorMessage>()
            .add_message::<BoxddBodyMoveMessage>()
            .add_message::<BoxddContactBeginMessage>()
            .add_message::<BoxddContactEndMessage>()
            .add_message::<BoxddContactHitMessage>()
            .add_message::<BoxddSensorBeginMessage>()
            .add_message::<BoxddSensorEndMessage>();

        app.insert_resource(self.settings.clone());

        if let Some(seconds) = self.settings.fixed_timestep_seconds {
            if seconds.is_finite() && seconds > 0.0 {
                app.insert_resource(Time::<Fixed>::from_seconds(seconds));
            } else {
                let message = BoxddErrorMessage {
                    operation: BoxddOperation::ConfigureFixedTimestep,
                    entity: None,
                    error: BoxddPluginError::Api(boxdd::ApiError::InvalidArgument),
                };
                report_startup_error(app, self.settings.error_policy, message);
                app.insert_resource(Time::<Fixed>::default());
            }
        }

        let context = match BoxddPhysicsContext::new(&self.settings) {
            Ok(context) => context,
            Err(error) => {
                let message = BoxddErrorMessage {
                    operation: BoxddOperation::CreateWorld,
                    entity: None,
                    error: error.into(),
                };
                report_startup_error(app, self.settings.error_policy, message);
                BoxddPhysicsContext::disabled()
            }
        };

        app.insert_non_send(context);

        app.add_systems(
            FixedUpdate,
            (
                cleanup_removed_colliders,
                cleanup_removed_bodies,
                create_missing_bodies,
                ApplyDeferred,
                apply_body_settings,
                create_missing_shapes,
                apply_body_controls,
                sync_bevy_transforms_to_boxdd,
                step_world,
                publish_physics_messages,
                sync_boxdd_transforms_to_bevy,
            )
                .chain(),
        );
    }
}

fn report_startup_error(app: &mut App, policy: BoxddErrorPolicy, message: BoxddErrorMessage) {
    match policy {
        BoxddErrorPolicy::MessageOnly => {
            app.world_mut().write_message(message);
        }
        BoxddErrorPolicy::MessageAndLog => {
            log::error!("{message:?}");
            app.world_mut().write_message(message);
        }
        BoxddErrorPolicy::Panic => {
            panic!("{message:?}");
        }
    }
}
