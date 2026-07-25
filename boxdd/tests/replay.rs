use boxdd::{
    Aabb, ApiError, BodyBuilder, BodyType, ChainDef, DebugDraw, DebugDrawOptions, DistanceJointDef,
    FoundationActivityError, JointBase, MixerRequirements, Position, QueryFilter, Recording,
    RecordingCapacity, ReplayConfig, ReplayError, ReplayKeyframePolicy, ReplayPlayer,
    ReplayQueryKind, ReplayStatus, RevoluteJointDef, ShapeDef, SurfaceMaterial, Vec2, World,
    WorldDef, foundation, shapes,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

static REPLAY_TEST_LOCK: Mutex<()> = Mutex::new(());

fn replay_test_guard() -> MutexGuard<'static, ()> {
    REPLAY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn recorded_world(frame_count: usize) -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    world.create_circle_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut session = world.start_recording(RecordingCapacity::default());
    for _ in 0..frame_count {
        session.step(1.0 / 60.0, 4);
    }
    let recording = session.finish();
    drop(world);
    recording
}

fn query_recording() -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut session = world.start_recording(RecordingCapacity::default());
    session.step(1.0 / 60.0, 1);
    let hit = session.cast_ray_closest(
        Position::new(-2.0, 0.0),
        Vec2::new(4.0, 0.0),
        QueryFilter::default(),
    );
    assert!(hit.is_some());
    let recording = session.finish();
    drop(world);
    recording
}

fn all_query_kinds_recording() -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut session = world.start_recording(RecordingCapacity::default());
    session.step(1.0 / 60.0, 1);

    let filter = QueryFilter::default();
    let bounds = Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]);
    let proxy = [
        Vec2::new(-0.1, -0.1),
        Vec2::new(0.1, -0.1),
        Vec2::new(0.1, 0.1),
        Vec2::new(-0.1, 0.1),
    ];

    assert_eq!(
        session.overlap_aabb(Position::ZERO, bounds, filter),
        vec![shape]
    );
    assert_eq!(
        session.overlap_polygon_points(Position::ZERO, proxy, 0.0, filter),
        vec![shape]
    );
    assert!(
        !session
            .cast_ray_all(Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0), filter)
            .is_empty()
    );
    assert!(
        !session
            .cast_shape_points(
                Position::new(-2.0, 0.0),
                proxy,
                0.0,
                Vec2::new(4.0, 0.0),
                filter,
            )
            .is_empty()
    );
    let _ = session.collide_mover(
        Position::ZERO,
        Vec2::new(0.0, -0.25),
        Vec2::new(0.0, 0.25),
        0.25,
        filter,
    );
    assert!(
        session
            .cast_ray_closest(Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0), filter)
            .is_some()
    );
    let mover_fraction = session.cast_mover(
        Position::new(-2.0, 0.0),
        Vec2::new(0.0, -0.25),
        Vec2::new(0.0, 0.25),
        0.25,
        Vec2::new(4.0, 0.0),
        filter,
    );
    assert!((0.0..=1.0).contains(&mover_fraction));
    assert!(session.shape_test_point(shape, Position::ZERO));
    assert!(
        session
            .shape_ray_cast(shape, Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0))
            .hit
    );

    let recording = session.finish();
    drop(world);
    recording
}

fn body_slot_reuse_recording() -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let mut session = world.start_recording(RecordingCapacity::default());
    for _ in 0..3 {
        let body = session.create_body(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        session.destroy_body(body);
    }
    session.create_body(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 2.0])
            .build(),
    );
    session.step(1.0 / 60.0, 1);
    let recording = session.finish();
    drop(world);
    recording
}

fn chain_joint_lifecycle_recording() -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let chain_body = world.create_body_id(BodyBuilder::new().build());
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([2.0_f32, 0.0])
            .build(),
    );
    let mut session = world.start_recording(RecordingCapacity::default());
    let chain_def = ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();
    let chain = session.create_chain(chain_body, &chain_def);
    session.chain_set_surface_material(
        chain,
        0,
        &SurfaceMaterial::default()
            .with_friction(0.75)
            .with_user_material_id(17),
    );
    session.destroy_chain(chain);

    let distance_def = DistanceJointDef::new(JointBase::new(body_a, body_b));
    let distance = session.create_distance_joint(&distance_def);
    session.distance_joint_set_length(distance, 2.5);
    session.destroy_joint(distance, true);

    let revolute_def = RevoluteJointDef::new(JointBase::new(body_a, body_b));
    let revolute = session.create_revolute_joint(&revolute_def);
    session.revolute_joint_set_target_angle(revolute, 0.25);
    session.destroy_joint(revolute, true);

    let cascade_joint = session.create_distance_joint(&distance_def);
    session.destroy_body(body_a);
    assert_eq!(
        session.try_destroy_joint(cascade_joint, true),
        Err(ApiError::InvalidJointId)
    );

    let first_replacement =
        session.create_body(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let first_replacement_raw = first_replacement.unbind();
    session.destroy_body(first_replacement);
    let second_replacement =
        session.create_body(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let second_replacement_raw = second_replacement.unbind();
    assert_eq!(first_replacement_raw.index1, second_replacement_raw.index1);
    assert_ne!(
        first_replacement_raw.generation,
        second_replacement_raw.generation
    );
    session.step(1.0 / 60.0, 1);

    let recording = session.finish();
    drop(world);
    recording
}

fn recording_opcodes(recording: &Recording) -> Vec<u8> {
    const HEADER_BYTES: usize = 32;

    let bytes = recording.as_bytes();
    let snapshot_size = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
    let mut cursor = HEADER_BYTES + snapshot_size;
    let mut opcodes = Vec::new();
    while cursor < bytes.len() {
        let payload_size = usize::from(bytes[cursor + 1])
            | (usize::from(bytes[cursor + 2]) << 8)
            | (usize::from(bytes[cursor + 3]) << 16);
        opcodes.push(bytes[cursor]);
        cursor += 4 + payload_size;
    }
    assert_eq!(cursor, bytes.len());
    opcodes
}

fn assert_opcode_subsequence(actual: &[u8], expected: &[u8]) {
    let mut cursor = 0usize;
    for &opcode in actual {
        if expected.get(cursor) == Some(&opcode) {
            cursor += 1;
        }
    }
    assert_eq!(
        cursor,
        expected.len(),
        "missing ordered opcode suffix: actual={actual:02X?}, expected={expected:02X?}"
    );
}

fn mixer_recording() -> Recording {
    mixer_recording_with_restitution(false)
}

fn dual_mixer_recording() -> Recording {
    mixer_recording_with_restitution(true)
}

fn mixer_recording_with_restitution(include_restitution: bool) -> Recording {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    world.set_friction_callback(|a, b| a.coefficient.max(b.coefficient));
    if include_restitution {
        world.set_restitution_callback(|a, b| a.coefficient.max(b.coefficient));
    }
    let material = SurfaceMaterial::default()
        .with_friction(0.5)
        .with_user_material_id(7);
    let shape_def = ShapeDef::builder().density(1.0).material(material).build();
    let polygon = shapes::box_polygon(0.5, 0.5);
    let first = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let second = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.25_f32, 0.0])
            .build(),
    );
    world.create_polygon_shape_for(first, &shape_def, &polygon);
    world.create_polygon_shape_for(second, &shape_def, &polygon);
    let mut session = world.start_recording(RecordingCapacity::default());
    session.step(1.0 / 60.0, 2);
    let recording = session.finish();
    world.clear_friction_callback();
    if include_restitution {
        world.clear_restitution_callback();
    }
    drop(world);
    recording
}

fn corrupt_second_state_hash(recording: &Recording) -> Vec<u8> {
    const HEADER_BYTES: usize = 32;
    const STATE_HASH_OPCODE: u8 = 0xF1;

    let mut bytes = recording.as_bytes().to_vec();
    let snapshot_size = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
    let mut cursor = HEADER_BYTES + snapshot_size;
    let mut state_hashes = 0usize;
    while cursor < bytes.len() {
        let payload_size = usize::from(bytes[cursor + 1])
            | (usize::from(bytes[cursor + 2]) << 8)
            | (usize::from(bytes[cursor + 3]) << 16);
        if bytes[cursor] == STATE_HASH_OPCODE {
            state_hashes += 1;
            if state_hashes == 2 {
                assert_eq!(payload_size, 12);
                bytes[cursor + 8] ^= 1;
                return bytes;
            }
        }
        cursor += 4 + payload_size;
    }
    panic!("recording did not contain a post-step state hash");
}

struct PanicOnDrop(Arc<AtomicBool>);

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if !self.0.swap(true, Ordering::SeqCst) {
            panic!("replay mixer closure drop panic");
        }
    }
}

struct ReplayContractDrawer;

impl DebugDraw for ReplayContractDrawer {}

#[test]
fn replay_contract_paths_have_straight_line_ufcs_runtime_evidence() {
    let _guard = replay_test_guard();
    let recording = query_recording();
    let policy = ReplayKeyframePolicy::new(1024 * 1024, 1).unwrap();
    let mut player: ReplayPlayer =
        ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    let step_status = ReplayPlayer::step(&mut player).unwrap();
    let (body_present, hit_present) = ReplayPlayer::with_view(&player, |view| {
        let body = boxdd::ReplayView::body(&view, 0);
        let query = boxdd::ReplayView::query(&view, 0).expect("recorded closest-ray query");
        let hit = boxdd::ReplayQueryView::hit(&query, 0);
        (body.is_some(), hit.is_some())
    })
    .unwrap();
    let mut drawer = ReplayContractDrawer;
    ReplayPlayer::draw(
        &mut player,
        &mut drawer,
        DebugDrawOptions::default(),
        Some(0),
    )
    .unwrap();
    ReplayPlayer::set_keyframe_policy(&mut player, policy).unwrap();
    let seek_status = ReplayPlayer::seek(&mut player, 1).unwrap();
    let restart_status = ReplayPlayer::restart(&mut player).unwrap();

    assert_eq!(step_status, ReplayStatus::End { frame: 1 });
    assert!(body_present);
    assert!(hit_present);
    assert_eq!(seek_status, ReplayStatus::End { frame: 1 });
    assert_eq!(restart_status, ReplayStatus::Advanced { frame: 0 });
}

#[test]
fn replay_owns_a_copy_and_exposes_only_scoped_views() {
    let _guard = replay_test_guard();
    let recording = recorded_world(3);
    let requirements = recording.mixer_requirements();
    let mut source = recording.as_bytes().to_vec();
    let expected_len = source.len();
    let mut player =
        ReplayPlayer::open_bytes(&source, requirements, ReplayConfig::default()).unwrap();

    source.fill(0xA5);
    drop(source);
    assert_eq!(player.input_len(), expected_len);
    assert_eq!(player.frame(), 0);
    assert_eq!(player.info().frame_count, 3);

    player
        .with_view(|view| {
            assert_eq!(view.epoch(), player.epoch());
            assert_eq!(view.body_count(), 1);
            let body = view.body(0).expect("seed body");
            assert_eq!(body.ordinal(), 0);
            assert!(body.is_valid());
            assert_eq!(body.body_type(), BodyType::Dynamic);
            assert!(body.position().is_valid());
            assert_eq!(view.query_count(), 0);
        })
        .unwrap();

    assert_eq!(player.step().unwrap(), ReplayStatus::Advanced { frame: 1 });
    assert_eq!(player.seek(3).unwrap(), ReplayStatus::End { frame: 3 });
}

#[test]
fn mixer_requirements_must_match_and_are_installed_before_first_step() {
    let _guard = replay_test_guard();
    let recording = mixer_recording();
    assert!(recording.mixer_requirements().requires_friction());

    assert!(matches!(
        ReplayPlayer::open_recording(&recording, ReplayConfig::default()),
        Err(ReplayError::MixerSetMismatch { .. })
    ));

    let config =
        ReplayConfig::default().with_friction_mixer(|a, b| a.coefficient.max(b.coefficient));
    let mut player = ReplayPlayer::open_recording(&recording, config).unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    assert!(!player.has_diverged());
}

#[test]
fn replay_mixer_panic_resumes_after_native_step_and_preserves_teardown() {
    let _guard = replay_test_guard();
    let recording = mixer_recording();
    let config = ReplayConfig::default().with_friction_mixer(|_, _| -> f32 {
        panic!("replay friction mixer panic");
    });
    let mut player = ReplayPlayer::open_recording(&recording, config).unwrap();
    let epoch = player.epoch();

    let panic = catch_unwind(AssertUnwindSafe(|| player.step()));

    assert!(panic.is_err());
    assert!(player.epoch() > epoch);
    assert!(player.is_healthy());
    drop(player);
    assert!(!foundation().activity().replay_active);
}

#[test]
fn replay_mixer_drop_panics_run_all_cleanup_before_resuming() {
    let _guard = replay_test_guard();
    let recording = dual_mixer_recording();
    let friction_dropped = Arc::new(AtomicBool::new(false));
    let restitution_dropped = Arc::new(AtomicBool::new(false));
    let friction_marker = PanicOnDrop(Arc::clone(&friction_dropped));
    let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
    let config = ReplayConfig::default()
        .with_friction_mixer(move |a, b| {
            let _ = &friction_marker;
            a.coefficient.max(b.coefficient)
        })
        .with_restitution_mixer(move |a, b| {
            let _ = &restitution_marker;
            a.coefficient.max(b.coefficient)
        });
    let player = ReplayPlayer::open_recording(&recording, config).unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| drop(player)));

    assert!(panic.is_err());
    assert!(friction_dropped.load(Ordering::SeqCst));
    assert!(restitution_dropped.load(Ordering::SeqCst));
    assert!(!foundation().activity().replay_active);
    drop(World::new(WorldDef::default()).unwrap());
}

#[test]
fn explicit_bytes_sidecar_is_checked_exactly() {
    let _guard = replay_test_guard();
    let recording = recorded_world(1);
    let unnecessary = MixerRequirements::new(true, false);
    let config =
        ReplayConfig::default().with_friction_mixer(|a, b| a.coefficient.max(b.coefficient));
    let mut player = ReplayPlayer::open_bytes(recording.as_bytes(), unnecessary, config).unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });

    assert!(matches!(
        ReplayPlayer::open_bytes(
            recording.as_bytes(),
            MixerRequirements::default(),
            ReplayConfig::default()
                .with_restitution_mixer(|a, b| { a.coefficient.max(b.coefficient) }),
        ),
        Err(ReplayError::MixerSetMismatch { .. })
    ));
}

#[test]
fn replay_lease_excludes_worlds_and_is_released_on_rejection_and_drop() {
    let _guard = replay_test_guard();
    let recording = recorded_world(1);

    let live_world = World::new(WorldDef::default()).unwrap();
    assert!(matches!(
        ReplayPlayer::open_recording(&recording, ReplayConfig::default()),
        Err(ReplayError::Foundation(
            FoundationActivityError::ReplayUnavailable { .. }
        ))
    ));
    assert!(!foundation().activity().replay_active);
    drop(live_world);

    let def_while_replaying = WorldDef::default();
    let player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    assert!(foundation().activity().replay_active);
    assert!(matches!(
        ReplayPlayer::open_recording(&recording, ReplayConfig::default()),
        Err(ReplayError::Foundation(
            FoundationActivityError::ReplayActive
        ))
    ));
    let world_error = World::new(def_while_replaying)
        .err()
        .expect("replay exclusivity must reject ordinary world creation");
    assert!(matches!(
        world_error,
        boxdd::world::Error::FoundationActivity(FoundationActivityError::ReplayActive)
            | boxdd::world::Error::InvalidDefinition(ApiError::FoundationActivity(
                FoundationActivityError::ReplayActive
            ))
    ));
    drop(player);
    assert!(!foundation().activity().replay_active);
    drop(World::new(WorldDef::default()).unwrap());
}

#[test]
fn seek_restart_and_policy_mutations_advance_the_epoch() {
    let _guard = replay_test_guard();
    let recording = recorded_world(5);
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    let initial_epoch = player.epoch();

    assert_eq!(player.seek(4).unwrap(), ReplayStatus::Advanced { frame: 4 });
    assert!(player.epoch() > initial_epoch);
    let forward_epoch = player.epoch();

    assert_eq!(player.seek(2).unwrap(), ReplayStatus::Advanced { frame: 2 });
    assert!(player.epoch() > forward_epoch);
    let backward_epoch = player.epoch();

    assert_eq!(
        player.restart().unwrap(),
        ReplayStatus::Advanced { frame: 0 }
    );
    assert!(player.epoch() > backward_epoch);
    let restart_epoch = player.epoch();

    let policy = ReplayKeyframePolicy::new(4 * 1024 * 1024, 2).unwrap();
    player.set_keyframe_policy(policy).unwrap();
    assert!(player.epoch() > restart_epoch);
    assert_eq!(player.keyframe_policy().budget_bytes, 4 * 1024 * 1024);
    assert_eq!(player.keyframe_policy().min_interval_frames, 2);

    let before_failure = player.epoch();
    assert_eq!(player.seek(u64::MAX), Err(ReplayError::FrameOutOfRange));
    assert!(player.epoch() > before_failure);
    assert_eq!(player.frame(), 0);
}

#[test]
fn divergence_is_distinct_from_end_and_latches_the_first_frame() {
    let _guard = replay_test_guard();
    let recording = recorded_world(2);
    let bytes = corrupt_second_state_hash(&recording);
    let mut player = ReplayPlayer::open_bytes(
        &bytes,
        recording.mixer_requirements(),
        ReplayConfig::default(),
    )
    .unwrap();

    assert_eq!(
        player.step().unwrap(),
        ReplayStatus::Diverged {
            frame: 1,
            first_divergence: 1,
        }
    );
    assert!(player.has_diverged());
    assert!(!player.is_at_end());
    assert_eq!(
        player.seek(2).unwrap(),
        ReplayStatus::Diverged {
            frame: 2,
            first_divergence: 1,
        }
    );
    assert!(player.is_at_end());
}

struct PanicDrawer;

impl DebugDraw for PanicDrawer {
    fn draw_solid_circle(
        &mut self,
        _transform: boxdd::WorldTransform,
        _center: Vec2,
        _radius: f32,
        _color: boxdd::HexColor,
    ) {
        panic!("replay debug draw panic");
    }
}

#[test]
fn replay_debug_draw_panic_resumes_at_player_boundary_without_invalidating_state() {
    let _guard = replay_test_guard();
    let recording = recorded_world(2);
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    player.step().unwrap();
    let epoch = player.epoch();
    let frame = player.frame();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        player.draw(&mut PanicDrawer, DebugDrawOptions::default(), None)
    }));
    assert!(panic.is_err());
    assert_eq!(player.epoch(), epoch);
    assert_eq!(player.frame(), frame);
    assert!(player.is_healthy());
    assert!(player.step().is_ok());
}

#[test]
fn malformed_input_is_distinct_and_never_takes_the_replay_lease() {
    let _guard = replay_test_guard();
    let live_world = World::new(WorldDef::default()).unwrap();
    let error = ReplayPlayer::open_bytes(
        b"not a recording",
        MixerRequirements::default(),
        ReplayConfig::default(),
    )
    .unwrap_err();
    assert!(matches!(error, ReplayError::Malformed(_)));
    let activity = foundation().activity();
    assert_eq!(activity.ordinary_worlds, 1);
    assert_eq!(activity.transient_calls, 0);
    assert!(!activity.replay_active);
    drop(live_world);
}

#[test]
fn replay_query_values_are_checked_and_do_not_expose_native_ids() {
    let _guard = replay_test_guard();
    let recording = query_recording();
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.query_count(), 1);
            let query = view.query(0).expect("recorded closest-ray query");
            assert_eq!(query.kind(), ReplayQueryKind::CastRayClosest);
            assert_eq!(query.origin(), Position::new(-2.0, 0.0));
            assert_eq!(query.translation(), Vec2::new(4.0, 0.0));
            assert_eq!(query.hit_count(), 1);

            let hit = query.hit(0).expect("recorded closest-ray hit");
            assert!(hit.point().expect("ray hit point").is_valid());
            assert!(hit.normal().expect("ray hit normal").is_valid());
            let fraction = hit.fraction().expect("ray hit fraction");
            assert!(fraction.is_finite());
            assert!((0.0..=1.0).contains(&fraction));
            assert!(query.hit(1).is_none());
        })
        .unwrap();
    assert!(!player.has_diverged());

    struct NoopDrawer;
    impl DebugDraw for NoopDrawer {}
    player
        .draw(&mut NoopDrawer, DebugDrawOptions::default(), Some(0))
        .unwrap();
}

#[test]
fn replay_preserves_every_recorded_query_kind_in_wire_order() {
    let _guard = replay_test_guard();
    let recording = all_query_kinds_recording();
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();

    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            let expected = [
                ReplayQueryKind::OverlapAabb,
                ReplayQueryKind::OverlapShape,
                ReplayQueryKind::CastRay,
                ReplayQueryKind::CastShape,
                ReplayQueryKind::CollideMover,
                ReplayQueryKind::CastRayClosest,
                ReplayQueryKind::CastMover,
                ReplayQueryKind::ShapeTestPoint,
                ReplayQueryKind::ShapeRayCast,
            ];
            assert_eq!(view.query_count(), expected.len());
            for (index, expected_kind) in expected.into_iter().enumerate() {
                let query = view.query(index).expect("recorded query");
                assert_eq!(query.kind(), expected_kind);
                if expected_kind == ReplayQueryKind::ShapeRayCast {
                    assert_eq!(query.hit_count(), 0);
                    assert!(query.hit(0).is_none());
                }
            }
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn destroyed_body_ordinals_remain_holes_when_native_slots_are_reused() {
    let _guard = replay_test_guard();
    let recording = body_slot_reuse_recording();
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();

    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.body_count(), 4);
            assert!(view.body(0).is_none());
            assert!(view.body(1).is_none());
            assert!(view.body(2).is_none());
            let live = view.body(3).expect("latest body creation ordinal");
            assert_eq!(live.ordinal(), 3);
            assert_eq!(live.body_type(), BodyType::Dynamic);
            assert!(live.position().is_valid());
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn replay_accepts_real_chain_joint_cascade_and_slot_reuse_lifecycle() {
    let _guard = replay_test_guard();
    let recording = chain_joint_lifecycle_recording();
    let opcodes = recording_opcodes(&recording);
    assert_opcode_subsequence(
        &opcodes,
        &[
            0x70, // CreateChain
            0x72, // ChainSetSurfaceMaterial
            0x71, // DestroyChain
            0x90, // CreateDistanceJoint
            0xA0, // DistanceJointSetLength
            0x97, // DestroyJoint
            0x94, // CreateRevoluteJoint
            0xC0, // RevoluteJointSetTargetAngle
            0x97, // DestroyJoint
            0x90, // CreateDistanceJoint, destroyed by the body cascade
            0x11, // DestroyBody
            0x10, // CreateBody, reusing the destroyed body's slot
            0x11, // DestroyBody
            0x10, // CreateBody, reusing that slot again
            0x80, // Step
        ],
    );

    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    assert_eq!(player.info().frame_count, 1);
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.body_count(), 5);
            let live_bodies = (0..view.body_count())
                .filter_map(|ordinal| view.body(ordinal))
                .collect::<Vec<_>>();
            assert_eq!(live_bodies.len(), 3);
            assert!(live_bodies.iter().all(|body| body.is_valid()));
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn replay_api_does_not_alias_an_ordinary_world_error_surface() {
    let _guard = replay_test_guard();
    let recording = recorded_world(0);
    let mut player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 0 });
    assert_ne!(
        ReplayError::FrameOutOfRange,
        ReplayError::Api(ApiError::InvalidArgument)
    );
    let bounds = player.info().bounds;
    assert!(bounds.is_valid());
    player
        .with_view(|view| assert_eq!(view.info().bounds, bounds))
        .unwrap();
    assert_eq!(Position::ZERO, Position::new(0.0, 0.0));
}
