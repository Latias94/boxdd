use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;
use static_assertions::assert_not_impl_any;

const SNAPSHOT_MIXER_ID: MixerId = MixerId::from_bytes([0x91; 32]);

assert_not_impl_any!(Snapshot: Clone, Send, Sync, AsRef<[u8]>);

#[derive(Clone)]
struct DropProbe {
    value: u32,
    drops: Arc<AtomicUsize>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn world_with_circle() -> (World, BodyId, ShapeId) {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape = world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    (world, body, shape)
}

#[test]
fn callback_generation_and_mixer_identity_requirements_are_fail_closed() {
    let mut callback_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    callback_world.set_custom_filter(|_, _| true).unwrap();
    let callback_snapshot = callback_world.snapshot().unwrap();
    callback_world.restore(&callback_snapshot).unwrap();

    callback_world.set_gravity(Vec2::ZERO).unwrap();
    callback_world.clear_custom_filter().unwrap();
    assert_eq!(
        callback_world.restore(&callback_snapshot).unwrap_err(),
        Error::SnapshotHostWiringMismatch
    );
    assert_eq!(callback_world.gravity().unwrap(), Vec2::ZERO);
    callback_world.set_custom_filter(|_, _| true).unwrap();
    assert_eq!(
        callback_world.restore(&callback_snapshot).unwrap_err(),
        Error::SnapshotHostWiringMismatch
    );

    let mut mixer_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    mixer_world
        .set_friction_callback(SNAPSHOT_MIXER_ID, |left, right| {
            left.coefficient.max(right.coefficient)
        })
        .unwrap();
    let mixer_snapshot = mixer_world.snapshot().unwrap();
    mixer_world.clear_friction_callback().unwrap();
    assert_eq!(
        mixer_world.restore(&mixer_snapshot).unwrap_err(),
        Error::SnapshotHostWiringMismatch
    );
    mixer_world
        .set_friction_callback(SNAPSHOT_MIXER_ID, |left, right| {
            left.coefficient.max(right.coefficient)
        })
        .unwrap();
    mixer_world.restore(&mixer_snapshot).unwrap();
}

#[test]
fn in_place_restore_recovers_state_remaps_destroyed_objects_and_rejects_foreign_worlds() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position(Position::new(2.0, 3.0))
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape = world
        .body(body)
        .unwrap()
        .create_centered_circle(
            &ShapeDef::builder()
                .enable_sensor_events(true)
                .enable_contact_events(false)
                .build()
                .unwrap(),
            0.5,
        )
        .unwrap();
    let snapshot = world.snapshot().unwrap();

    world
        .body(body)
        .unwrap()
        .set_position_and_rotation(Position::new(9.0, 8.0), 0.4)
        .unwrap();
    world.shape(shape).unwrap().destroy(true).unwrap();
    let later_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();

    let prepared = world.prepare_restore(&snapshot).unwrap();
    assert_eq!(prepared.mappings().body_id(body), Some(body));
    let restored = prepared.commit().unwrap();
    let restored_body = restored.body_id(body).unwrap();
    let restored_shape = restored.shape_id(shape).unwrap();
    assert_eq!(restored_body, body);
    assert_ne!(restored_shape, shape);
    assert_eq!(
        world.body(restored_body).unwrap().position().unwrap(),
        Position::new(2.0, 3.0)
    );
    let restored_shape_capability = world.shape(restored_shape).unwrap();
    assert!(restored_shape_capability.sensor_events_enabled().unwrap());
    assert!(!restored_shape_capability.contact_events_enabled().unwrap());
    assert_eq!(world.shape(shape).err().unwrap(), Error::InvalidShapeId);
    assert_eq!(world.body(later_body).err().unwrap(), Error::InvalidBodyId);

    let (mut foreign, foreign_body, _) = world_with_circle();
    let foreign_before = foreign.body(foreign_body).unwrap().position().unwrap();
    assert_eq!(
        foreign.restore(&snapshot).unwrap_err(),
        Error::ForeignSnapshot
    );
    assert_eq!(
        foreign.body(foreign_body).unwrap().position().unwrap(),
        foreign_before
    );
}

#[test]
fn restore_reattaches_only_unchanged_user_data_versions() {
    let keep_drops = Arc::new(AtomicUsize::new(0));
    let replace_drops = Arc::new(AtomicUsize::new(0));
    let later_drops = Arc::new(AtomicUsize::new(0));
    let world_drops = Arc::new(AtomicUsize::new(0));
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let keep_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let replace_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();

    world
        .set_user_data(DropProbe {
            value: 10,
            drops: Arc::clone(&world_drops),
        })
        .unwrap();
    world
        .body(keep_body)
        .unwrap()
        .set_user_data(DropProbe {
            value: 20,
            drops: Arc::clone(&keep_drops),
        })
        .unwrap();
    world
        .body(replace_body)
        .unwrap()
        .set_user_data(DropProbe {
            value: 30,
            drops: Arc::clone(&replace_drops),
        })
        .unwrap();
    let snapshot = world.snapshot().unwrap();

    world
        .body(replace_body)
        .unwrap()
        .set_user_data(DropProbe {
            value: 31,
            drops: Arc::clone(&replace_drops),
        })
        .unwrap();
    let later_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
        .body(later_body)
        .unwrap()
        .set_user_data(DropProbe {
            value: 60,
            drops: Arc::clone(&later_drops),
        })
        .unwrap();

    let restored = world.restore(&snapshot).unwrap();
    let keep_body = restored.body_id(keep_body).unwrap();
    let replace_body = restored.body_id(replace_body).unwrap();
    assert_eq!(
        world
            .with_user_data::<DropProbe, _>(|value| value.value)
            .unwrap(),
        Some(10)
    );
    assert_eq!(
        world
            .body(keep_body)
            .unwrap()
            .with_user_data::<DropProbe, _>(|value| value.value)
            .unwrap(),
        Some(20)
    );
    assert_eq!(
        world
            .body(replace_body)
            .unwrap()
            .with_user_data::<DropProbe, _>(|value| value.value)
            .unwrap(),
        None
    );
    assert_eq!(replace_drops.load(Ordering::SeqCst), 2);
    assert_eq!(later_drops.load(Ordering::SeqCst), 1);
    assert_eq!(keep_drops.load(Ordering::SeqCst), 0);
    assert_eq!(world_drops.load(Ordering::SeqCst), 0);

    drop(world);
    assert_eq!(keep_drops.load(Ordering::SeqCst), 1);
    assert_eq!(world_drops.load(Ordering::SeqCst), 1);
}
