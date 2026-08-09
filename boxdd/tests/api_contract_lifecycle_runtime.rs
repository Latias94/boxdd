use boxdd::{
    Recording, RecordingLimits, RecordingSession, ReplayConfig, ReplayPlayer, Snapshot,
    SnapshotRestore, World,
};

#[test]
fn snapshot_recording_and_replay_lifecycles_have_runtime_evidence() {
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");

    let snapshot: Snapshot = World::snapshot(&world).expect("snapshot capture should succeed");
    let restored: SnapshotRestore =
        World::restore(&mut world, &snapshot).expect("snapshot restore should succeed");
    std::mem::drop(restored);

    let dropped_session: RecordingSession<'_> =
        World::start_recording(&mut world, RecordingLimits::default())
            .expect("recording start should succeed");
    std::mem::drop(dropped_session);
    let mut finished_session: RecordingSession<'_> =
        World::start_recording(&mut world, RecordingLimits::default())
            .expect("recording restart should succeed");
    std::mem::drop(
        RecordingSession::step(&mut finished_session, 0.0, 1)
            .expect("recording step should succeed"),
    );
    let recording: Recording =
        RecordingSession::finish(finished_session).expect("recording finish should succeed");
    std::mem::drop(world);

    let player: ReplayPlayer = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .expect("replay open should succeed");
    std::mem::drop(player);
}
