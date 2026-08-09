use crate::Vec2;
use crate::error::Error;
#[cfg(not(target_arch = "wasm32"))]
use crate::{DebugDraw, DebugDrawCmd, DebugDrawOptions, ExplosionDef, HexColor, Position};
use std::cell::Cell;
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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct CountingDebugDraw {
    calls: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl DebugDraw for CountingDebugDraw {
    fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {
        self.calls += 1;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn debug_draw_sentinel() -> DebugDrawCmd {
    DebugDrawCmd::Point {
        p: Position::ZERO,
        size: 17.0,
        color: HexColor::GREEN,
    }
}

#[test]
fn world_entries_reject_busy_worlds_before_access_or_mutation() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world.set_user_data(String::from("original")).unwrap();

    let recording = world.core().begin_recording_activity().unwrap();

    assert_eq!(world.gravity().unwrap_err(), Error::WorldBusy);
    assert_eq!(
        world
            .set_user_data(String::from("replacement"))
            .unwrap_err(),
        Error::WorldBusy
    );
    assert_eq!(world.clear_user_data().unwrap_err(), Error::WorldBusy);
    assert_eq!(
        world.take_user_data::<String>().unwrap_err(),
        Error::WorldBusy
    );

    drop(recording);
    assert_eq!(
        world
            .with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("original")
    );

    let restoring = world.core().begin_restore_activity().unwrap();
    assert_eq!(world.gravity().unwrap_err(), Error::WorldBusy);
    drop(restoring);
}

#[test]
fn object_capability_acquisition_gates_activity_before_identity_checks() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let live = world
        .create_body(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let stale = world
        .create_body(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world.body(stale).unwrap().destroy().unwrap();

    let mut foreign_world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let foreign = foreign_world
        .create_body(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();

    let recording = world.core().begin_recording_activity().unwrap();
    assert_eq!(world.body(stale).err().unwrap(), Error::WorldBusy);
    assert_eq!(world.body(foreign).err().unwrap(), Error::WorldBusy);
    drop(recording);

    assert!(
        world
            .body(live)
            .unwrap()
            .is_contact_recycling_enabled()
            .unwrap()
    );

    world.core().poison();
    assert_eq!(world.body(stale).err().unwrap(), Error::WorldPoisoned);
    assert_eq!(world.body(foreign).err().unwrap(), Error::WorldPoisoned);
}

#[test]
fn runtime_control_entries_gate_before_validation_and_conversion() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let original_gravity = world.gravity().unwrap();
    let converted = Rc::new(Cell::new(false));

    let recording = world.core().begin_recording_activity().unwrap();

    assert_eq!(world.step(f32::NAN, 0).unwrap_err(), Error::WorldBusy);
    assert_eq!(
        world
            .set_gravity(GravityConversionProbe {
                converted: Rc::clone(&converted),
            })
            .unwrap_err(),
        Error::WorldBusy
    );
    assert!(!converted.get());
    assert_eq!(world.gravity().unwrap_err(), Error::WorldBusy);
    assert_eq!(world.counters().unwrap_err(), Error::WorldBusy);
    assert_eq!(world.enable_sleeping(false).unwrap_err(), Error::WorldBusy);
    assert_eq!(
        world
            .set_contact_tuning(f32::NAN, f32::NAN, f32::NAN)
            .unwrap_err(),
        Error::WorldBusy
    );
    assert_eq!(world.is_continuous_enabled().unwrap_err(), Error::WorldBusy);

    drop(recording);
    assert_eq!(world.gravity().unwrap(), original_gravity);
}

#[test]
fn callback_entries_leave_host_and_material_registries_unchanged_when_busy() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let recording = world.core().begin_recording_activity().unwrap();

    assert_eq!(
        world.set_custom_filter(|_, _| true).unwrap_err(),
        Error::WorldBusy
    );
    assert!(world.core().custom_filter.lock().unwrap().is_none());
    assert_eq!(
        world
            .set_friction_callback(crate::MixerId::from_bytes([0x51; 32]), |a, b| {
                a.coefficient.max(b.coefficient)
            })
            .unwrap_err(),
        Error::WorldBusy
    );
    assert!(
        world
            .core()
            .material_mix
            .lock()
            .unwrap()
            .slot_for_test()
            .is_none()
    );

    drop(recording);
    world
        .set_friction_callback(crate::MixerId::from_bytes([0x51; 32]), |a, b| {
            a.coefficient.max(b.coefficient)
        })
        .unwrap();
    let slot = world
        .core()
        .material_mix
        .lock()
        .unwrap()
        .slot_for_test()
        .unwrap();
    assert_eq!(
        world.core().material_mix.lock().unwrap().presence(),
        (true, false)
    );

    let restoring = world.core().begin_restore_activity().unwrap();
    assert_eq!(
        world.clear_friction_callback().unwrap_err(),
        Error::WorldBusy
    );
    assert_eq!(
        world.core().material_mix.lock().unwrap().slot_for_test(),
        Some(slot)
    );
    assert_eq!(
        world.core().material_mix.lock().unwrap().presence(),
        (true, false)
    );

    drop(restoring);
    world.clear_friction_callback().unwrap();
    assert!(
        world
            .core()
            .material_mix
            .lock()
            .unwrap()
            .slot_for_test()
            .is_none()
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn debug_draw_and_explosion_leave_outputs_untouched_when_busy() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut commands = vec![debug_draw_sentinel()];
    let mut drawer = CountingDebugDraw::default();
    let explosion = ExplosionDef::new();

    let recording = world.core().begin_recording_activity().unwrap();

    assert_eq!(
        world
            .debug_draw_collect_into(&mut commands, DebugDrawOptions::default())
            .unwrap_err(),
        Error::WorldBusy
    );
    assert!(matches!(
        commands.as_slice(),
        [DebugDrawCmd::Point { size: 17.0, .. }]
    ));
    assert_eq!(
        world
            .debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap_err(),
        Error::WorldBusy
    );
    assert_eq!(drawer.calls, 0);
    assert_eq!(world.explode(&explosion).unwrap_err(), Error::WorldBusy);

    drop(recording);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn world_entries_reject_poisoned_worlds() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut drawer = CountingDebugDraw::default();
    let explosion = ExplosionDef::new();
    world.core().poison();

    assert_eq!(world.has_user_data().unwrap_err(), Error::WorldPoisoned);
    assert_eq!(world.gravity().unwrap_err(), Error::WorldPoisoned);
    assert_eq!(world.step(1.0 / 60.0, 1).unwrap_err(), Error::WorldPoisoned);
    assert_eq!(
        world.set_custom_filter(|_, _| true).unwrap_err(),
        Error::WorldPoisoned
    );
    assert_eq!(
        world
            .debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap_err(),
        Error::WorldPoisoned
    );
    assert_eq!(world.explode(&explosion).unwrap_err(), Error::WorldPoisoned);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn callback_error_precedes_world_activity_and_argument_errors() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let converted = Rc::new(Cell::new(false));
    let mut commands = vec![debug_draw_sentinel()];
    let explosion = ExplosionDef::new();
    let recording = world.core().begin_recording_activity().unwrap();

    {
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(world.gravity().unwrap_err(), Error::InCallback);
        assert_eq!(world.step(f32::NAN, 0).unwrap_err(), Error::InCallback);
        assert_eq!(
            world
                .set_gravity(GravityConversionProbe {
                    converted: Rc::clone(&converted),
                })
                .unwrap_err(),
            Error::InCallback
        );
        assert!(!converted.get());
        assert_eq!(
            world.set_user_data(String::from("blocked")).unwrap_err(),
            Error::InCallback
        );
        assert_eq!(
            world.set_custom_filter(|_, _| true).unwrap_err(),
            Error::InCallback
        );
        assert_eq!(
            world
                .debug_draw_collect_into(&mut commands, DebugDrawOptions::default())
                .unwrap_err(),
            Error::InCallback
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(world.explode(&explosion).unwrap_err(), Error::InCallback);
    }

    drop(recording);
}

#[test]
fn creating_a_world_from_a_callback_returns_before_native_creation() {
    let _guard = crate::core::callback_state::CallbackGuard::enter();
    assert!(matches!(
        crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def()
            ),
        Err(Error::InCallback)
    ));
}
