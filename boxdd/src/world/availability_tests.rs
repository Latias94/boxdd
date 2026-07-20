use super::{World, WorldDef};
use crate::core::world_core::ActivityState;
use crate::error::ApiError;
use crate::{DebugDraw, DebugDrawCmd, DebugDrawOptions, ExplosionDef, HexColor, Position, Vec2};
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

struct GravityConversionProbe {
    converted: Rc<Cell<bool>>,
}

impl From<GravityConversionProbe> for Vec2 {
    fn from(probe: GravityConversionProbe) -> Self {
        probe.converted.set(true);
        Vec2::ZERO
    }
}

#[derive(Default)]
struct CountingDebugDraw {
    calls: usize,
}

impl DebugDraw for CountingDebugDraw {
    fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {
        self.calls += 1;
    }
}

fn debug_draw_sentinel() -> DebugDrawCmd {
    DebugDrawCmd::Point {
        p: Position::ZERO,
        size: 17.0,
        color: HexColor::GREEN,
    }
}

#[test]
fn world_entries_reject_busy_worlds_before_access_or_mutation() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();
    world.set_user_data(String::from("original"));

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(world.try_is_valid().unwrap_err(), ApiError::WorldBusy);
    assert_eq!(handle.try_gravity().unwrap_err(), ApiError::WorldBusy);
    assert_eq!(
        world
            .try_set_user_data(String::from("replacement"))
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world.try_clear_user_data().unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world.try_take_user_data::<String>().unwrap_err(),
        ApiError::WorldBusy
    );

    assert!(catch_unwind(AssertUnwindSafe(|| world.world_id_raw())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| handle.world_id_raw())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| world.is_valid())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| handle.gravity())).is_err());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            world.set_user_data(String::from("replacement"));
        }))
        .is_err()
    );
    assert!(catch_unwind(AssertUnwindSafe(|| world.handle())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| world.owned_handle_counts())).is_err());

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
    assert_eq!(
        world.with_user_data::<String, _>(Clone::clone).as_deref(),
        Some("original")
    );

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Restoring)
        .unwrap();
    assert_eq!(world.try_is_valid().unwrap_err(), ApiError::WorldBusy);
    assert_eq!(handle.try_gravity().unwrap_err(), ApiError::WorldBusy);
    world
        .core()
        .set_activity(ActivityState::Restoring, ActivityState::Idle)
        .unwrap();
}

#[test]
fn runtime_control_entries_gate_before_validation_and_conversion() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let original_gravity = world.gravity();
    let converted = Rc::new(Cell::new(false));

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(
        world.try_step(f32::NAN, 0).unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world.try_flush_deferred_destroys().unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world
            .try_set_gravity(GravityConversionProbe {
                converted: Rc::clone(&converted),
            })
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert!(!converted.get());
    assert_eq!(world.try_gravity().unwrap_err(), ApiError::WorldBusy);
    assert_eq!(world.try_counters().unwrap_err(), ApiError::WorldBusy);
    assert_eq!(
        world.try_enable_sleeping(false).unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world
            .try_set_contact_tuning(f32::NAN, f32::NAN, f32::NAN)
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        world.try_is_continuous_enabled().unwrap_err(),
        ApiError::WorldBusy
    );

    let infallible_converted = Rc::new(Cell::new(false));
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            world.set_gravity(GravityConversionProbe {
                converted: Rc::clone(&infallible_converted),
            });
        }))
        .is_err()
    );
    assert!(!infallible_converted.get());
    assert!(catch_unwind(AssertUnwindSafe(|| world.step(f32::NAN, 0))).is_err());

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
    assert_eq!(world.gravity(), original_gravity);
}

#[test]
fn callback_entries_leave_host_and_material_registries_unchanged_when_busy() {
    let mut world = World::new(WorldDef::default()).unwrap();
    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(
        world.try_set_custom_filter(|_, _| true).unwrap_err(),
        ApiError::WorldBusy
    );
    assert!(world.core().custom_filter.lock().unwrap().is_none());
    assert_eq!(
        world
            .try_set_friction_callback(|a, b| a.coefficient.max(b.coefficient))
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert!(world.core().material_mix_slot.lock().unwrap().is_none());

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
    world
        .try_set_friction_callback(|a, b| a.coefficient.max(b.coefficient))
        .unwrap();
    let slot = world.core().material_mix_slot.lock().unwrap().unwrap();
    assert!(crate::core::material_mix_registry::has_any_callback(slot));

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Restoring)
        .unwrap();
    assert_eq!(
        world.try_clear_friction_callback().unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(*world.core().material_mix_slot.lock().unwrap(), Some(slot));
    assert!(crate::core::material_mix_registry::has_any_callback(slot));

    world
        .core()
        .set_activity(ActivityState::Restoring, ActivityState::Idle)
        .unwrap();
    world.try_clear_friction_callback().unwrap();
    assert!(world.core().material_mix_slot.lock().unwrap().is_none());
}

#[test]
fn debug_draw_and_explosion_entries_gate_before_outputs_or_user_code() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut commands = vec![debug_draw_sentinel()];
    let mut drawer = CountingDebugDraw::default();
    let explosion = ExplosionDef::new();

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(
        world
            .try_debug_draw_collect_into(&mut commands, DebugDrawOptions::default())
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        commands[0],
        DebugDrawCmd::Point { size: 17.0, .. }
    ));
    assert_eq!(
        world
            .try_debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(drawer.calls, 0);
    assert_eq!(
        world.try_explode(&explosion).unwrap_err(),
        ApiError::WorldBusy
    );

    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            world.debug_draw_collect_into(&mut commands, DebugDrawOptions::default());
        }))
        .is_err()
    );
    assert_eq!(commands.len(), 1);
    assert!(catch_unwind(AssertUnwindSafe(|| world.explode(&explosion))).is_err());

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
}

#[cfg(feature = "serialize")]
#[test]
fn registry_entries_reject_busy_worlds_without_rewriting_outputs() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(crate::BodyBuilder::new().build());
    let mut bodies = vec![body];

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(
        world.try_body_ids_into(&mut bodies).unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(bodies, [body]);
    assert_eq!(world.try_chain_records().unwrap_err(), ApiError::WorldBusy);
    assert!(catch_unwind(AssertUnwindSafe(|| world.body_ids())).is_err());

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
}

#[test]
fn world_entries_reject_poisoned_worlds() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();
    let mut drawer = CountingDebugDraw::default();
    let explosion = ExplosionDef::new();
    world.core().poison();

    assert_eq!(world.try_is_valid().unwrap_err(), ApiError::WorldPoisoned);
    assert_eq!(
        world.try_has_user_data().unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert_eq!(handle.try_gravity().unwrap_err(), ApiError::WorldPoisoned);
    assert_eq!(
        world.try_step(1.0 / 60.0, 1).unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert_eq!(
        world.try_set_custom_filter(|_, _| true).unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert_eq!(
        world
            .try_debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert_eq!(
        world.try_explode(&explosion).unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert!(catch_unwind(AssertUnwindSafe(|| world.world_id_raw())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| handle.gravity())).is_err());
}

#[test]
fn callback_error_precedes_world_activity_errors() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();
    let converted = Rc::new(Cell::new(false));
    let mut commands = vec![debug_draw_sentinel()];
    let explosion = ExplosionDef::new();
    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    {
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(world.try_is_valid().unwrap_err(), ApiError::InCallback);
        assert_eq!(handle.try_gravity().unwrap_err(), ApiError::InCallback);
        assert_eq!(
            world.try_step(f32::NAN, 0).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_set_gravity(GravityConversionProbe {
                    converted: Rc::clone(&converted),
                })
                .unwrap_err(),
            ApiError::InCallback
        );
        assert!(!converted.get());
        assert_eq!(
            world
                .try_set_user_data(String::from("blocked"))
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world.try_set_custom_filter(|_, _| true).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world
                .try_debug_draw_collect_into(&mut commands, DebugDrawOptions::default())
                .unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(
            world.try_explode(&explosion).unwrap_err(),
            ApiError::InCallback
        );
    }

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
}

#[test]
fn creating_a_world_from_a_callback_panics_before_native_creation() {
    let _guard = crate::core::callback_state::CallbackGuard::enter();
    assert!(catch_unwind(AssertUnwindSafe(|| World::new(WorldDef::default()))).is_err());
}
