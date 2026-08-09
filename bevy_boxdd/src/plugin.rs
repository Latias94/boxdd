//! Bevy plugin wiring for `boxdd` fixed-step physics.

use crate::messages::{
    BoxddBodyMoveMessage, BoxddContactBeginMessage, BoxddContactEndMessage, BoxddContactHitMessage,
    BoxddErrorMessage, BoxddJointEventMessage, BoxddOperation, BoxddPluginError,
    BoxddSensorBeginMessage, BoxddSensorEndMessage, BoxddSnapshotRestoreMessage,
    WorldOriginRebased,
};
use crate::origin::BoxddWorldOrigin;
use crate::resources::{
    BoxddEcsWorldBindingState, BoxddErrorPolicy, BoxddPhysicsContext, BoxddPhysicsSettings,
    BoxddStepSettings, apply_pending_snapshot_restore,
};
use crate::systems::{
    apply_body_controls, apply_body_settings, apply_pending_world_origin_rebase,
    cleanup_removed_bodies, cleanup_removed_colliders, cleanup_removed_joints,
    context_world_binding_is_valid, create_missing_bodies, create_missing_joints,
    create_missing_shapes, reconcile_identity_projections, replace_changed_joints,
    replace_changed_shapes, step_and_publish_physics_messages, sync_bevy_transforms_to_boxdd,
    sync_boxdd_transforms_to_bevy, validate_context_world_binding, world_origin_is_settled,
};
use bevy_app::{App, FixedUpdate, Plugin};
use bevy_ecs::schedule::{ApplyDeferred, IntoScheduleConfigs, SystemSet};
use bevy_time::{Fixed, Time};
use std::time::Duration;

/// Stable ordering points for systems that integrate with the fixed physics pipeline.
///
/// Application systems can order themselves before or after these sets without depending on the
/// plugin's private system functions.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoxddPhysicsSet {
    /// Validate that the non-send context still belongs to the current Bevy world.
    Validate,
    /// Apply a pending absolute world-origin rebase.
    Rebase,
    /// Commit one queued snapshot restore before any native lifecycle system can run.
    Restore,
    /// Repair opaque ECS projections from the authoritative context graph.
    Reconcile,
    /// Remove native objects whose authored ECS state no longer exists.
    Cleanup,
    /// Create native bodies before other objects can reference them.
    CreateBodies,
    /// Apply body settings and transforms before constraint creation.
    PrepareBodies,
    /// Create or replace shapes and joints, then apply body controls.
    PrepareConstraints,
    /// Advance the native world and publish explicitly requested event families.
    Step,
    /// Write simulated transforms back to Bevy.
    Writeback,
}

/// Plugin that owns the Box2D world and registers fixed-step physics systems.
#[derive(Clone, Debug)]
pub struct BoxddPhysicsPlugin {
    settings: BoxddPhysicsSettings,
    foundation: &'static boxdd::Foundation,
}

impl BoxddPhysicsPlugin {
    /// Creates the plugin from an explicitly initialized Box2D foundation.
    pub fn new(foundation: &'static boxdd::Foundation, settings: BoxddPhysicsSettings) -> Self {
        Self {
            settings,
            foundation,
        }
    }
}

impl Plugin for BoxddPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BoxddErrorMessage>()
            .add_message::<BoxddSnapshotRestoreMessage>()
            .add_message::<BoxddBodyMoveMessage>()
            .add_message::<BoxddContactBeginMessage>()
            .add_message::<BoxddContactEndMessage>()
            .add_message::<BoxddContactHitMessage>()
            .add_message::<BoxddJointEventMessage>()
            .add_message::<BoxddSensorBeginMessage>()
            .add_message::<BoxddSensorEndMessage>()
            .add_message::<WorldOriginRebased>()
            .init_resource::<BoxddWorldOrigin>()
            .init_resource::<BoxddEcsWorldBindingState>();

        let mut step_settings = BoxddStepSettings {
            sub_step_count: self.settings.sub_step_count,
            ..Default::default()
        };
        if let Some(seconds) = self.settings.fixed_timestep_seconds {
            let fallback_seconds = seconds as f32;
            let timestep = Duration::try_from_secs_f64(seconds)
                .ok()
                .filter(|timestep| !timestep.is_zero());
            if fallback_seconds.is_finite()
                && fallback_seconds > 0.0
                && let Some(timestep) = timestep
            {
                app.insert_resource(Time::<Fixed>::from_duration(timestep));
            } else {
                let message = BoxddErrorMessage {
                    operation: BoxddOperation::ConfigureFixedTimestep,
                    entity: None,
                    error: BoxddPluginError::Api(boxdd::Error::invalid_argument(
                        "BoxddPhysicsPlugin::build",
                        "fixed_timestep_seconds",
                        "a finite positive duration representable by Bevy's fixed clock and f32",
                    )),
                };
                report_startup_error(app, self.settings.error_policy, message);
                app.insert_resource(Time::<Fixed>::default());
            }
        } else {
            app.init_resource::<Time<Fixed>>();
        }
        step_settings.fallback_timestep_seconds = app
            .world()
            .resource::<Time<Fixed>>()
            .timestep()
            .as_secs_f32();
        app.insert_resource(step_settings)
            .insert_resource(self.settings.error_policy)
            .insert_resource(self.settings.event_interests);

        let context = match BoxddPhysicsContext::new(app.world(), self.foundation, &self.settings) {
            Ok(context) => context,
            Err(error) => {
                let message = BoxddErrorMessage {
                    operation: BoxddOperation::CreateWorld,
                    entity: None,
                    error: error.into(),
                };
                report_startup_error(app, self.settings.error_policy, message);
                BoxddPhysicsContext::disabled_with_reason(
                    app.world(),
                    Some(self.foundation),
                    crate::BoxddContextDisabledReason::StartupWorldCreationFailed,
                )
            }
        };

        app.insert_non_send(context);

        app.configure_sets(
            FixedUpdate,
            (
                BoxddPhysicsSet::Validate,
                BoxddPhysicsSet::Rebase,
                BoxddPhysicsSet::Restore,
                BoxddPhysicsSet::Reconcile,
                BoxddPhysicsSet::Cleanup,
                BoxddPhysicsSet::CreateBodies,
                BoxddPhysicsSet::PrepareBodies,
                BoxddPhysicsSet::PrepareConstraints,
                BoxddPhysicsSet::Step,
                BoxddPhysicsSet::Writeback,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            validate_context_world_binding.in_set(BoxddPhysicsSet::Validate),
        )
        .add_systems(
            FixedUpdate,
            apply_pending_world_origin_rebase
                .in_set(BoxddPhysicsSet::Rebase)
                .run_if(context_world_binding_is_valid),
        )
        .add_systems(
            FixedUpdate,
            apply_pending_snapshot_restore.in_set(BoxddPhysicsSet::Restore),
        )
        .add_systems(
            FixedUpdate,
            (reconcile_identity_projections, ApplyDeferred)
                .chain()
                .in_set(BoxddPhysicsSet::Reconcile),
        )
        .add_systems(
            FixedUpdate,
            (
                cleanup_removed_joints,
                cleanup_removed_colliders,
                cleanup_removed_bodies,
            )
                .chain()
                .in_set(BoxddPhysicsSet::Cleanup)
                .distributive_run_if(context_world_binding_is_valid)
                .distributive_run_if(world_origin_is_settled),
        )
        .add_systems(
            FixedUpdate,
            (create_missing_bodies, ApplyDeferred)
                .chain()
                .in_set(BoxddPhysicsSet::CreateBodies)
                .run_if(context_world_binding_is_valid)
                .run_if(world_origin_is_settled),
        )
        .add_systems(
            FixedUpdate,
            (apply_body_settings, sync_bevy_transforms_to_boxdd)
                .chain()
                .in_set(BoxddPhysicsSet::PrepareBodies)
                .run_if(context_world_binding_is_valid)
                .run_if(world_origin_is_settled),
        )
        .add_systems(
            FixedUpdate,
            (
                replace_changed_shapes,
                create_missing_shapes,
                replace_changed_joints,
                create_missing_joints,
                apply_body_controls,
            )
                .chain()
                .in_set(BoxddPhysicsSet::PrepareConstraints)
                .distributive_run_if(context_world_binding_is_valid)
                .distributive_run_if(world_origin_is_settled),
        )
        .add_systems(
            FixedUpdate,
            step_and_publish_physics_messages
                .in_set(BoxddPhysicsSet::Step)
                .run_if(context_world_binding_is_valid)
                .run_if(world_origin_is_settled),
        )
        .add_systems(
            FixedUpdate,
            sync_boxdd_transforms_to_bevy
                .in_set(BoxddPhysicsSet::Writeback)
                .run_if(context_world_binding_is_valid)
                .run_if(world_origin_is_settled),
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
