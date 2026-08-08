//! Public contract tests for opaque process-local recordings.

use boxdd::{Error, MixerId, Recording, RecordingLimits, ReplayConfig, ReplayPlayer};
use static_assertions::{assert_impl_all, assert_not_impl_any};

const FRICTION_V1: MixerId = MixerId::from_bytes([0x11; 32]);
const FRICTION_V2: MixerId = MixerId::from_bytes([0x22; 32]);

assert_impl_all!(Recording: Send, Sync);
assert_not_impl_any!(Recording: Clone, PartialEq, AsRef<[u8]>);

fn friction(left: boxdd::MaterialMixInput, right: boxdd::MaterialMixInput) -> f32 {
    left.coefficient.max(right.coefficient)
}

#[test]
fn process_local_recording_replays_with_matching_mixer_identity() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    world.set_friction_callback(FRICTION_V1, friction).unwrap();
    let recording = world
        .start_recording(RecordingLimits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(recording.mixer_identities().friction(), Some(FRICTION_V1));
    drop(world);

    let config = ReplayConfig::default().with_friction_mixer(FRICTION_V1, friction);
    let player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        config,
    )
    .unwrap();
    assert_eq!(player.info().frame_count, 0);
}

#[test]
fn replay_rejects_same_callback_presence_with_different_identity() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world.set_friction_callback(FRICTION_V1, friction).unwrap();
    let recording = world
        .start_recording(RecordingLimits::default())
        .unwrap()
        .finish()
        .unwrap();
    drop(world);

    let config = ReplayConfig::default().with_friction_mixer(FRICTION_V2, friction);
    assert!(matches!(
        ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            config
        ),
        Err(Error::ReplayMixerIdentityMismatch)
    ));
}
