use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .build()?,
    )?;
    let body = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position(Position::new(0.0, 3.0))
            .build()?,
    )?;
    let shape = world.body(body)?.create_circle(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::circle(Vec2::ZERO, 0.5)?,
    )?;

    // Snapshot is an opaque same-world checkpoint. Safe Rust never exposes its native bytes.
    let snapshot = world.snapshot()?;
    world
        .body(body)?
        .set_position_and_rotation(Position::new(20.0, 3.0), 0.0)?;
    let restored = world.restore(&snapshot)?;

    // Restore may mint new capability identities. Translate every retained pre-restore ID.
    let body = restored.body_id(body).expect("body exists in snapshot");
    let _shape = restored.shape_id(shape).expect("shape exists in snapshot");
    assert_eq!(world.body(body)?.position()?, Position::new(0.0, 3.0));

    // RecordingSession is the only world access surface while capture is active.
    let mut session = world.start_recording(RecordingLimits::DEFAULT)?;
    session
        .body(body)?
        .set_linear_velocity(Vec2::new(1.0, 0.0))?;
    drop(session.step(1.0 / 60.0, 4)?);
    let hit = session.query()?.cast_ray_closest(
        Position::new(-2.0, 3.0),
        Vec2::new(4.0, 0.0),
        QueryFilter::default(),
    )?;
    assert!(hit.is_some());
    let recording = session.finish()?;

    // Replay owns a copy of the opaque recording and requires process-exclusive foundation access.
    drop(snapshot);
    drop(world);
    let mut player = ReplayPlayer::open(foundation, &recording, ReplayConfig::default())?;
    assert_eq!(player.info().frame_count, 1);
    let status = player.step()?;

    let (replayed_position, query_kind, hit_count) = player.with_view(|view| {
        let replayed_body = view.body(0)?.expect("recorded body exists");
        let query = view.query(0)?.expect("recorded query exists");
        Ok((replayed_body.position()?, query.kind(), query.hit_count()))
    })?;
    assert_eq!(query_kind, ReplayQueryKind::CastRayClosest);
    assert_eq!(hit_count, 1);

    println!("snapshot/replay: status={status:?} position={replayed_position:?}");
    player.close()?;
    Ok(())
}
