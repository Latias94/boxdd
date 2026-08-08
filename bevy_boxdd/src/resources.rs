//! Bevy resources that own the native physics world and plugin settings.

mod identity;
mod native_creation;
mod queries;
mod snapshot;

use crate::math::to_boxdd_vec2;
use crate::messages::{BoxddContextDisabledReason, BoxddPluginError};
use crate::origin::BoxddWorldOrigin;
use bevy_ecs::{
    prelude::{Resource, World as EcsWorld},
    world::WorldId,
};
use bevy_math::Vec2 as BevyVec2;
use boxdd::{
    CompletedStep, Error as BoxddError, Foundation, Position, Result as BoxddResult, World,
    WorldBuilder,
};
#[cfg(not(target_arch = "wasm32"))]
use boxdd::{RayQueryBuffer, ShapeQueryBuffer};
use identity::{EventEntityLookup, IdentityGraph};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

pub(crate) use identity::BodyDependents;
pub(crate) use native_creation::{BodyDescriptor, ShapeDescriptor, ShapeLocalTransform};
pub use queries::{BoxddClosestRayCastResult, BoxddRayHit, BoxddShapeHit};
pub use snapshot::BoxddPhysicsSnapshot;
pub(crate) use snapshot::apply_pending_snapshot_restore;

pub(crate) struct StepEventErrors {
    pub(crate) step: Option<BoxddError>,
    pub(crate) read_events: Option<BoxddError>,
}

/// How the plugin reports recoverable errors from fixed-update systems.
#[derive(Resource, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum BoxddErrorPolicy {
    /// Emit [`crate::BoxddErrorMessage`] only.
    #[default]
    MessageOnly,
    /// Emit [`crate::BoxddErrorMessage`] and log the error.
    MessageAndLog,
    /// Panic immediately when a recoverable plugin error is observed.
    Panic,
}

/// Runtime settings for the fixed Box2D step.
///
/// This resource is separate from [`BoxddPhysicsSettings`] because the Bevy fixed clock is startup
/// configuration, while sub-step count is intentionally mutable at runtime. Gravity is used to
/// create the native world and can later be changed through [`BoxddPhysicsContext::set_gravity`].
#[derive(Resource, Copy, Clone, Debug, PartialEq)]
pub struct BoxddStepSettings {
    /// Box2D sub-step count used for each fixed step.
    pub sub_step_count: i32,
    /// Fallback step duration used when Bevy's fixed clock has not advanced yet.
    pub fallback_timestep_seconds: f32,
}

impl Default for BoxddStepSettings {
    fn default() -> Self {
        Self {
            sub_step_count: 4,
            fallback_timestep_seconds: 1.0 / 60.0,
        }
    }
}

/// Event families materialized and published by the Bevy integration.
///
/// The default has no interests, so an ordinary fixed step does not call any native event getter.
/// Applications opt in only to the message families they consume.
#[derive(Resource, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct BoxddEventInterests {
    /// Publish [`crate::BoxddBodyMoveMessage`] values.
    pub body_moves: bool,
    /// Publish contact begin, end, and hit messages.
    pub contacts: bool,
    /// Publish [`crate::BoxddJointEventMessage`] values.
    pub joints: bool,
    /// Publish sensor begin and end messages.
    pub sensors: bool,
}

impl BoxddEventInterests {
    /// No event families are materialized.
    pub const NONE: Self = Self {
        body_moves: false,
        contacts: false,
        joints: false,
        sensors: false,
    };

    /// Every event family is materialized and published.
    pub const ALL: Self = Self {
        body_moves: true,
        contacts: true,
        joints: true,
        sensors: true,
    };

    /// Enables or disables body-move messages.
    pub const fn with_body_moves(mut self, enabled: bool) -> Self {
        self.body_moves = enabled;
        self
    }

    /// Enables or disables contact messages.
    pub const fn with_contacts(mut self, enabled: bool) -> Self {
        self.contacts = enabled;
        self
    }

    /// Enables or disables joint messages.
    pub const fn with_joints(mut self, enabled: bool) -> Self {
        self.joints = enabled;
        self
    }

    /// Enables or disables sensor messages.
    pub const fn with_sensors(mut self, enabled: bool) -> Self {
        self.sensors = enabled;
        self
    }
}

/// Startup configuration used by [`crate::BoxddPhysicsPlugin`].
///
/// The plugin consumes this value and installs separate runtime resources for step settings,
/// event interests, and error policy. `gravity` is the initial native-world value; use
/// [`BoxddPhysicsContext::set_gravity`] to change it at runtime. The fixed-clock field is consumed
/// only during plugin construction.
#[derive(Clone, Debug)]
pub struct BoxddPhysicsSettings {
    /// Gravity used when creating the native Box2D world.
    pub gravity: BevyVec2,
    /// Box2D sub-step count used for each fixed step.
    pub sub_step_count: i32,
    /// Optional Bevy fixed timestep override in seconds.
    ///
    /// `None` preserves an existing [`bevy_time::Time`] resource specialized for
    /// [`bevy_time::Fixed`], or installs Bevy's default fixed clock when the app has none.
    pub fixed_timestep_seconds: Option<f64>,
    /// Error reporting policy for plugin systems.
    pub error_policy: BoxddErrorPolicy,
    /// Event families that the plugin should materialize after a successful step.
    pub event_interests: BoxddEventInterests,
}

/// Cached per-App binding validated at the head of each fixed-update pipeline.
///
/// Bevy run conditions must be send, so they compare the public origin against this snapshot.
/// Each physics system separately compares its live non-send context to the same snapshot, closing
/// replacement races between scheduled systems without accessing non-send state from a condition.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct BoxddEcsWorldBindingState {
    valid: bool,
    owner_world: Option<WorldId>,
    committed_world_origin: Position,
    world_origin_revision: u64,
    context_identity: Option<Arc<()>>,
}

impl Default for BoxddPhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: BevyVec2::new(0.0, -10.0),
            sub_step_count: 4,
            fixed_timestep_seconds: Some(1.0 / 60.0),
            error_policy: BoxddErrorPolicy::MessageOnly,
            event_interests: BoxddEventInterests::NONE,
        }
    }
}

/// Non-send resource that owns the native Box2D world and ECS id mappings.
///
/// `boxdd::World` is intentionally `!Send`/`!Sync`; Bevy apps must access this resource from
/// main-thread systems. The bidirectional maps are the sole native ownership authority; runtime
/// ID components are read-only ECS projections.
pub struct BoxddPhysicsContext {
    context_identity: Arc<()>,
    owner_world: WorldId,
    committed_world_origin: Position,
    world_origin_revision: u64,
    foundation: Option<&'static Foundation>,
    world: Option<World>,
    graph: IdentityGraph,
    #[cfg(not(target_arch = "wasm32"))]
    ray_hits: RayQueryBuffer,
    #[cfg(not(target_arch = "wasm32"))]
    shape_hits: ShapeQueryBuffer,
    pending_snapshot_restore: Option<snapshot::PendingSnapshotRestore>,
    next_snapshot_restore_ticket: u64,
    pub(crate) last_step_failed: bool,
    disabled_reason: Option<BoxddContextDisabledReason>,
}

impl std::fmt::Debug for BoxddPhysicsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxddPhysicsContext")
            .field("owner_world", &self.owner_world)
            .field("committed_world_origin", &self.committed_world_origin)
            .field("world_origin_revision", &self.world_origin_revision)
            .field("foundation_initialized", &self.foundation.is_some())
            .field("world_enabled", &self.world.is_some())
            .field("body_count", &self.graph.entity_to_body.len())
            .field("shape_count", &self.graph.entity_to_shape.len())
            .field("joint_count", &self.graph.entity_to_joint.len())
            .field(
                "retired_shape_count",
                &self.graph.retired_shape_to_entity.len(),
            )
            .field(
                "pending_snapshot_restore",
                &self.pending_snapshot_restore.is_some(),
            )
            .field("last_step_failed", &self.last_step_failed)
            .field("disabled_reason", &self.disabled_reason)
            .finish()
    }
}

impl BoxddPhysicsContext {
    /// Creates a context bound to `ecs_world` and a native Box2D world from plugin settings.
    pub fn new(
        ecs_world: &EcsWorld,
        foundation: &'static Foundation,
        settings: &BoxddPhysicsSettings,
    ) -> BoxddResult<Self> {
        let origin = ecs_world
            .get_resource::<BoxddWorldOrigin>()
            .copied()
            .unwrap_or_default();
        let world = foundation.create_world(
            WorldBuilder::from(foundation.world_def())
                .gravity(to_boxdd_vec2(settings.gravity))
                .build()?,
        )?;
        Ok(Self {
            context_identity: Arc::new(()),
            owner_world: ecs_world.id(),
            committed_world_origin: origin.active(),
            world_origin_revision: origin.revision(),
            foundation: Some(foundation),
            world: Some(world),
            graph: IdentityGraph::default(),
            #[cfg(not(target_arch = "wasm32"))]
            ray_hits: RayQueryBuffer::new(),
            #[cfg(not(target_arch = "wasm32"))]
            shape_hits: ShapeQueryBuffer::new(),
            pending_snapshot_restore: None,
            next_snapshot_restore_ticket: 0,
            last_step_failed: false,
            disabled_reason: None,
        })
    }

    /// Creates a context without a native world.
    ///
    /// This is used after startup world creation fails so the app can keep
    /// running while reporting the failure through the configured error policy.
    pub fn disabled(ecs_world: &EcsWorld) -> Self {
        Self::disabled_with_reason(ecs_world, None, BoxddContextDisabledReason::Explicit)
    }

    pub(crate) fn disabled_with_reason(
        ecs_world: &EcsWorld,
        foundation: Option<&'static Foundation>,
        reason: BoxddContextDisabledReason,
    ) -> Self {
        let origin = ecs_world
            .get_resource::<BoxddWorldOrigin>()
            .copied()
            .unwrap_or_default();
        Self {
            context_identity: Arc::new(()),
            owner_world: ecs_world.id(),
            committed_world_origin: origin.active(),
            world_origin_revision: origin.revision(),
            foundation,
            world: None,
            graph: IdentityGraph::default(),
            #[cfg(not(target_arch = "wasm32"))]
            ray_hits: RayQueryBuffer::new(),
            #[cfg(not(target_arch = "wasm32"))]
            shape_hits: ShapeQueryBuffer::new(),
            pending_snapshot_restore: None,
            next_snapshot_restore_ticket: 0,
            last_step_failed: true,
            disabled_reason: Some(reason),
        }
    }

    /// Returns the native world, if startup succeeded.
    pub fn world(&self) -> Option<&World> {
        self.world.as_ref()
    }

    /// Returns the explicit Box2D foundation root, if initialization succeeded.
    pub const fn foundation(&self) -> Option<&'static Foundation> {
        self.foundation
    }

    /// Returns why this context has no live native world.
    pub const fn disabled_reason(&self) -> Option<BoxddContextDisabledReason> {
        self.disabled_reason
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.world.is_some() && self.disabled_reason.is_none()
    }

    pub(crate) const fn owner_world(&self) -> WorldId {
        self.owner_world
    }

    pub(crate) const fn identity_token(&self) -> &Arc<()> {
        &self.context_identity
    }

    pub(crate) fn world_origin_matches(&self, origin: &BoxddWorldOrigin) -> bool {
        self.committed_world_origin == origin.active()
            && self.world_origin_revision == origin.revision()
    }

    pub(crate) fn commit_world_origin(&mut self, origin: Position, revision: u64) {
        self.committed_world_origin = origin;
        self.world_origin_revision = revision;
    }

    /// Updates native gravity without exposing mutable access to the owned world.
    pub fn set_gravity(&mut self, gravity: BevyVec2) -> Result<(), BoxddPluginError> {
        self.plugin_world_mut()?
            .set_gravity(to_boxdd_vec2(gravity))?;
        Ok(())
    }

    /// Enables or disables native body sleeping.
    pub fn enable_sleeping(&mut self, enabled: bool) -> Result<(), BoxddPluginError> {
        self.plugin_world_mut()?.enable_sleeping(enabled)?;
        Ok(())
    }

    /// Enables or disables native warm starting.
    pub fn enable_warm_starting(&mut self, enabled: bool) -> Result<(), BoxddPluginError> {
        self.plugin_world_mut()?.enable_warm_starting(enabled)?;
        Ok(())
    }

    /// Enables or disables native continuous collision detection.
    pub fn enable_continuous(&mut self, enabled: bool) -> Result<(), BoxddPluginError> {
        self.plugin_world_mut()?.enable_continuous(enabled)?;
        Ok(())
    }

    pub(crate) fn step_with_events(
        &mut self,
        time_step: f32,
        sub_step_count: i32,
        publish: impl FnOnce(&CompletedStep<'_>, EventEntityLookup<'_>) -> BoxddResult<()>,
    ) -> Option<StepEventErrors> {
        let (world, graph, last_step_failed) =
            (&mut self.world, &mut self.graph, &mut self.last_step_failed);
        let world = world.as_mut()?;
        let completed = match world.step(time_step, sub_step_count) {
            Ok(completed) => completed,
            Err(error) => {
                *last_step_failed = true;
                graph.release_retired_event_identities();
                return Some(StepEventErrors {
                    step: Some(error),
                    read_events: None,
                });
            }
        };
        *last_step_failed = false;
        let post_step_error = completed.post_step_error();

        let published = catch_unwind(AssertUnwindSafe(|| {
            publish(&completed, graph.event_lookup())
        }));
        drop(completed);
        graph.release_retired_event_identities();

        match published {
            Ok(result) => Some(StepEventErrors {
                step: post_step_error,
                read_events: result.err(),
            }),
            Err(payload) => resume_unwind(payload),
        }
    }

    fn live_world_mut(&mut self) -> BoxddResult<&mut World> {
        self.world.as_mut().ok_or(BoxddError::WorldDestroyed)
    }

    pub(super) fn plugin_world(&self) -> Result<&World, BoxddPluginError> {
        self.world.as_ref().ok_or(self.disabled_error())
    }

    pub(super) fn plugin_world_mut(&mut self) -> Result<&mut World, BoxddPluginError> {
        let reason = self
            .disabled_reason
            .unwrap_or(BoxddContextDisabledReason::Explicit);
        self.world
            .as_mut()
            .ok_or(BoxddPluginError::ContextDisabled { reason })
    }

    fn disabled_error(&self) -> BoxddPluginError {
        BoxddPluginError::ContextDisabled {
            reason: self
                .disabled_reason
                .unwrap_or(BoxddContextDisabledReason::Explicit),
        }
    }
}

impl BoxddEcsWorldBindingState {
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
        self.owner_world = None;
        self.context_identity = None;
    }

    pub(crate) fn validate(
        &mut self,
        actual_world: WorldId,
        origin: &BoxddWorldOrigin,
        context: &BoxddPhysicsContext,
    ) {
        self.valid = true;
        self.owner_world = Some(actual_world);
        self.committed_world_origin = origin.active();
        self.world_origin_revision = origin.revision();
        self.context_identity = Some(Arc::clone(&context.context_identity));
    }

    pub(crate) fn allows_origin(
        &self,
        actual_world: WorldId,
        origin: Option<&BoxddWorldOrigin>,
    ) -> bool {
        self.valid
            && self.owner_world == Some(actual_world)
            && origin.is_some_and(|origin| {
                self.committed_world_origin == origin.active()
                    && self.world_origin_revision == origin.revision()
            })
    }

    pub(crate) fn allows_context(
        &self,
        actual_world: WorldId,
        context: &BoxddPhysicsContext,
    ) -> bool {
        self.valid
            && self.owner_world == Some(actual_world)
            && self
                .context_identity
                .as_ref()
                .is_some_and(|identity| Arc::ptr_eq(identity, &context.context_identity))
            && context.owner_world == actual_world
            && context.committed_world_origin == self.committed_world_origin
            && context.world_origin_revision == self.world_origin_revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_step_releases_retired_event_identity_mappings() {
        let mut ecs_world = EcsWorld::new();
        let entity = ecs_world.spawn_empty().id();
        let foundation = Foundation::initialize_default().expect("foundation should initialize");
        let mut context =
            BoxddPhysicsContext::new(&ecs_world, foundation, &BoxddPhysicsSettings::default())
                .expect("native world creation should succeed");
        assert!(std::ptr::eq(
            context
                .foundation()
                .expect("live context should retain root"),
            foundation,
        ));
        let body_id = context
            .world
            .as_mut()
            .expect("context should own its native world")
            .create_body(foundation.body_def())
            .expect("native body creation should succeed");
        context.graph.retired_body_to_entity.insert(body_id, entity);

        let result = context.step_with_events(f32::NAN, 1, |_, _| Ok(()));

        let errors = result.expect("a live context reports the rejected step");
        assert!(errors.step.is_some());
        assert!(errors.read_events.is_none());
        assert!(context.graph.retired_body_to_entity.is_empty());
    }

    #[test]
    fn publish_panic_releases_retired_event_identities_and_preserves_context() {
        let mut ecs_world = EcsWorld::new();
        let entity = ecs_world.spawn_empty().id();
        let foundation = Foundation::initialize_default().expect("foundation should initialize");
        let mut context =
            BoxddPhysicsContext::new(&ecs_world, foundation, &BoxddPhysicsSettings::default())
                .expect("native world creation should succeed");
        let body_id = context
            .world
            .as_mut()
            .expect("context should own its native world")
            .create_body(foundation.body_def())
            .expect("native body creation should succeed");
        context.graph.retired_body_to_entity.insert(body_id, entity);

        let result = catch_unwind(AssertUnwindSafe(|| {
            context.step_with_events(1.0 / 60.0, 1, |_, lookup| -> BoxddResult<()> {
                assert_eq!(lookup.body(body_id), Some(entity));
                panic!("intentional event publication panic");
            })
        }));

        let payload = match result {
            Ok(_) => panic!("event publication panic should resume after cleanup"),
            Err(payload) => payload,
        };
        assert_eq!(
            payload.downcast_ref::<&'static str>(),
            Some(&"intentional event publication panic")
        );
        assert!(context.graph.retired_body_to_entity.is_empty());

        let errors = context
            .step_with_events(1.0 / 60.0, 1, |_, lookup| {
                assert_eq!(lookup.body(body_id), None);
                Ok(())
            })
            .expect("the context should remain live after publication panic cleanup");
        assert!(errors.step.is_none());
        assert!(errors.read_events.is_none());
    }
}
