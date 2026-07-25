use boxdd::{
    Recording, RecordingCapacity, RecordingSession, ReplayConfig, ReplayPlayer, Snapshot,
    SnapshotImage, SnapshotLoad, SnapshotRestore, WorkerCount, World, WorldDef,
};

#[test]
fn snapshot_recording_and_replay_lifecycles_have_runtime_evidence() {
    let mut world: World = World::new(WorldDef::default()).expect("world creation should succeed");

    let snapshot: Snapshot = World::try_snapshot(&world).expect("snapshot capture should succeed");
    let image: &SnapshotImage = Snapshot::image(&snapshot);
    let loaded: SnapshotLoad =
        SnapshotImage::load(image, WorkerCount::default()).expect("snapshot load should succeed");
    std::mem::drop(loaded);
    let restored: SnapshotRestore =
        World::try_restore(&mut world, &snapshot).expect("snapshot restore should succeed");
    std::mem::drop(restored);

    let dropped_session: RecordingSession<'_> =
        World::try_start_recording(&mut world, RecordingCapacity::default())
            .expect("recording start should succeed");
    std::mem::drop(dropped_session);
    let mut finished_session: RecordingSession<'_> =
        World::try_start_recording(&mut world, RecordingCapacity::default())
            .expect("recording restart should succeed");
    RecordingSession::step(&mut finished_session, 0.0, 1);
    let recording: Recording =
        RecordingSession::try_finish(finished_session).expect("recording finish should succeed");
    let recording_is_empty = Recording::is_empty(&recording);
    std::mem::drop(world);

    let player: ReplayPlayer = ReplayPlayer::open_recording(&recording, ReplayConfig::default())
        .expect("replay open should succeed");
    std::mem::drop(player);

    assert!(!recording_is_empty);
}
