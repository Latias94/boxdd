use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build())?;
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(Position::new(0.0, 3.0))
            .build(),
    );
    let shape = world.create_circle_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle(Vec2::ZERO, 0.5),
    );

    // Snapshot is an in-process restore capability. Its image is ABI-, precision-, upstream-,
    // layout-, and target-bound rather than a cross-version save-game format.
    let snapshot = world.try_snapshot()?;
    let snapshot_bytes = snapshot.image().as_bytes().to_vec();

    world.try_set_body_position_and_rotation(body, Position::new(20.0, 3.0), 0.0)?;
    let restored = world.try_restore(&snapshot)?;

    // Every restore creates a new identity generation. Replace every pre-restore Safe ID with the
    // corresponding ID from the restore map before touching the world again.
    let body = restored.body_id(body).expect("body exists in snapshot");
    let _shape = restored.shape_id(shape).expect("shape exists in snapshot");
    assert_eq!(world.body_position(body), Position::new(0.0, 3.0));

    // External bytes can only create a fresh-token world. Host callbacks, userdata, and material
    // mixers are deliberately not reconstructed by SnapshotImage::load.
    let image = SnapshotImage::from_bytes(&snapshot_bytes)?;
    let loaded = image.load(WorkerCount::default())?;
    assert_eq!(loaded.body_ids().len(), 1);
    let loaded_world = loaded.into_world();
    drop(loaded_world);

    // RecordingSession exclusively borrows the ordinary world, so mutations and queries during
    // capture go through its explicit public surface.
    let mut session = world.try_start_recording(RecordingCapacity::DEFAULT)?;
    session.try_set_body_linear_velocity(body, Vec2::new(1.0, 0.0))?;
    session.try_step(1.0 / 60.0, 4)?;
    let hit = session.try_cast_ray_closest(
        Position::new(-2.0, 3.0),
        Vec2::new(4.0, 0.0),
        QueryFilter::default(),
    )?;
    assert!(hit.is_some());
    let recording = session.try_finish()?;

    // Raw recording bytes need the separately persisted mixer sidecar. Replay owns a native
    // player exclusively, so all ordinary worlds and snapshot authorities must be gone first.
    let recording_bytes = recording.as_bytes().to_vec();
    let mixer_requirements = recording.mixer_requirements();
    drop(snapshot);
    drop(world);

    let mut player = ReplayPlayer::open_bytes(
        &recording_bytes,
        mixer_requirements,
        ReplayConfig::default(),
    )?;
    assert_eq!(player.info().frame_count, 1);
    let status = player.step()?;

    let (replayed_position, query_kind, hit_count) = player.with_view(|view| {
        let replayed_body = view.body(0).expect("recorded body");
        let query = view.query(0).expect("recorded closest-ray query");
        (replayed_body.position(), query.kind(), query.hit_count())
    })?;
    assert_eq!(query_kind, ReplayQueryKind::CastRayClosest);
    assert_eq!(hit_count, 1);

    println!(
        "persistence: snapshot_bytes={} recording_bytes={} status={status:?} position={replayed_position:?}",
        snapshot_bytes.len(),
        recording_bytes.len(),
    );
    player.close()?;
    Ok(())
}
