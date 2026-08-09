use crate::World;
use crate::error::Error;

fn query_error(world: &World) -> Error {
    match world.query() {
        Ok(_) => panic!("query capability acquisition unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
fn query_acquisition_rejects_busy_worlds() {
    let world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let core = world.core();

    let recording = core.begin_recording_activity().unwrap();
    assert_eq!(query_error(&world), Error::WorldBusy);
    drop(recording);

    let restoring = core.begin_restore_activity().unwrap();
    assert_eq!(query_error(&world), Error::WorldBusy);
    drop(restoring);
}

#[test]
fn query_acquisition_rejects_poisoned_worlds() {
    let world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world.core().poison();
    assert_eq!(query_error(&world), Error::WorldPoisoned);
}

#[test]
fn callback_error_takes_precedence_over_world_activity() {
    let world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let core = world.core();
    let recording = core.begin_recording_activity().unwrap();

    {
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(query_error(&world), Error::InCallback);
    }

    drop(recording);
}
