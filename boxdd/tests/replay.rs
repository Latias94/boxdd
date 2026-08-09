use boxdd::{
    Aabb, BodyType, ChainDef, DebugDraw, DebugDrawOptions, DistanceJointDef, Error, Foundation,
    FoundationActivityError, MixerId, Position, QueryFilter, Recording, RecordingLimits,
    ReplayConfig, ReplayKeyframePolicy, ReplayPlayer, ReplayQueryKind, ReplayStatus,
    RevoluteJointDef, ShapeDef, ShapeProxy, SurfaceMaterial, Vec2, shapes,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

static REPLAY_TEST_LOCK: Mutex<()> = Mutex::new(());
const FRICTION_MIXER_ID: MixerId = MixerId::from_bytes([0x31; 32]);
const RESTITUTION_MIXER_ID: MixerId = MixerId::from_bytes([0x32; 32]);

fn replay_test_guard() -> MutexGuard<'static, ()> {
    REPLAY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn recorded_world(frame_count: usize) -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 3.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_circle(
            &ShapeDef::builder().density(1.0).build().unwrap(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    for _ in 0..frame_count {
        drop(session.step(1.0 / 60.0, 4).unwrap());
    }
    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn recorded_empty_world(frame_count: usize) -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    for _ in 0..frame_count {
        drop(session.step(1.0 / 60.0, 1).unwrap());
    }
    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn query_recording() -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    drop(session.step(1.0 / 60.0, 1).unwrap());
    let hit = session
        .query()
        .unwrap()
        .cast_ray_closest(
            Position::new(-2.0, 0.0),
            Vec2::new(4.0, 0.0),
            QueryFilter::default(),
        )
        .unwrap();
    assert!(hit.is_some());
    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn all_query_kinds_recording() -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape = world
        .body(body)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    drop(session.step(1.0 / 60.0, 1).unwrap());

    let filter = QueryFilter::default();
    let bounds = Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap();
    let proxy = ShapeProxy::new(
        [
            Vec2::new(-0.1, -0.1),
            Vec2::new(0.1, -0.1),
            Vec2::new(0.1, 0.1),
            Vec2::new(-0.1, 0.1),
        ],
        0.0,
    )
    .unwrap();

    {
        let query = session.query().unwrap();
        assert_eq!(
            query.overlap_aabb(Position::ZERO, bounds, filter).unwrap(),
            vec![shape]
        );
        assert_eq!(
            query.overlap_shape(Position::ZERO, proxy, filter).unwrap(),
            vec![shape]
        );
        assert!(
            !query
                .cast_ray_all(Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0), filter)
                .unwrap()
                .is_empty()
        );
        assert!(
            !query
                .cast_shape(Position::new(-2.0, 0.0), proxy, Vec2::new(4.0, 0.0), filter,)
                .unwrap()
                .is_empty()
        );
        let _ = query
            .collide_mover(
                Position::ZERO,
                Vec2::new(0.0, -0.25),
                Vec2::new(0.0, 0.25),
                0.25,
                filter,
            )
            .unwrap();
        assert!(
            query
                .cast_ray_closest(Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0), filter)
                .unwrap()
                .is_some()
        );
        let mover_fraction = query
            .cast_mover(
                Position::new(-2.0, 0.0),
                Vec2::new(0.0, -0.25),
                Vec2::new(0.0, 0.25),
                0.25,
                Vec2::new(4.0, 0.0),
                filter,
            )
            .unwrap();
        assert!((0.0..=1.0).contains(&mover_fraction));
    }
    assert!(
        session
            .shape(shape)
            .unwrap()
            .test_point(Position::ZERO)
            .unwrap()
    );
    assert!(
        session
            .shape(shape)
            .unwrap()
            .ray_cast(Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0))
            .unwrap()
            .hit
    );

    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn body_slot_reuse_recording() -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    for _ in 0..3 {
        let body = session
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        session.body(body).unwrap().destroy().unwrap();
    }
    session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 2.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    drop(session.step(1.0 / 60.0, 1).unwrap());
    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn chain_joint_lifecycle_recording() -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let chain_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([2.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    let chain_def = ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build()
        .unwrap();
    let chain = session
        .body(chain_body)
        .unwrap()
        .create_chain(&chain_def)
        .unwrap();
    session
        .chain(chain)
        .unwrap()
        .set_surface_material(
            0,
            &SurfaceMaterial::default()
                .with_friction(0.75)
                .unwrap()
                .with_user_material_id(17),
        )
        .unwrap();
    session.chain(chain).unwrap().destroy().unwrap();

    let distance_def = DistanceJointDef::new(session.joint_base(body_a, body_b).unwrap());
    let distance = session.create_distance_joint(&distance_def).unwrap();
    session
        .joint(distance)
        .unwrap()
        .into_distance()
        .unwrap()
        .set_length(2.5)
        .unwrap();
    session.joint(distance).unwrap().destroy(true).unwrap();

    let revolute_def = RevoluteJointDef::new(session.joint_base(body_a, body_b).unwrap());
    let revolute = session.create_revolute_joint(&revolute_def).unwrap();
    session
        .joint(revolute)
        .unwrap()
        .into_revolute()
        .unwrap()
        .set_target_angle(0.25)
        .unwrap();
    session.joint(revolute).unwrap().destroy(true).unwrap();

    let cascade_joint = session.create_distance_joint(&distance_def).unwrap();
    session.body(body_a).unwrap().destroy().unwrap();
    assert_eq!(
        session.joint(cascade_joint).err(),
        Some(Error::InvalidJointId)
    );

    let first_replacement = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    session.body(first_replacement).unwrap().destroy().unwrap();
    let second_replacement = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    assert_ne!(first_replacement, second_replacement);
    drop(session.step(1.0 / 60.0, 1).unwrap());

    let recording = session.finish().unwrap();
    drop(world);
    recording
}

fn mixer_recording() -> Recording {
    mixer_recording_with_restitution(false)
}

fn dual_mixer_recording() -> Recording {
    mixer_recording_with_restitution(true)
}

fn mixer_recording_with_restitution(include_restitution: bool) -> Recording {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .set_friction_callback(FRICTION_MIXER_ID, |a, b| a.coefficient.max(b.coefficient))
        .unwrap();
    if include_restitution {
        world
            .set_restitution_callback(RESTITUTION_MIXER_ID, |a, b| {
                a.coefficient.max(b.coefficient)
            })
            .unwrap();
    }
    let material = SurfaceMaterial::default()
        .with_friction(0.5)
        .unwrap()
        .with_user_material_id(7);
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .material(material)
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let first = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let second = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.25_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(first)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    world
        .body(second)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    drop(session.step(1.0 / 60.0, 2).unwrap());
    let recording = session.finish().unwrap();
    world.clear_friction_callback().unwrap();
    if include_restitution {
        world.clear_restitution_callback().unwrap();
    }
    drop(world);
    recording
}

struct PanicOnDrop(Arc<AtomicUsize>);

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
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
    let mut player: ReplayPlayer = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    let step_status = ReplayPlayer::step(&mut player).unwrap();
    let (body_present, hit_present) = ReplayPlayer::with_view(&player, |view| {
        let body = boxdd::ReplayView::body(&view, 0)?;
        let query = boxdd::ReplayView::query(&view, 0)?.expect("recorded closest-ray query");
        let hit = boxdd::ReplayQueryView::hit(&query, 0)?;
        Ok((body.is_some(), hit.is_some()))
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
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    drop(recording);
    assert_eq!(player.frame(), 0);
    assert_eq!(player.info().frame_count, 3);

    player
        .with_view(|view| {
            assert_eq!(view.epoch(), player.epoch());
            assert_eq!(view.body_count(), 1);
            let body = view.body(0)?.expect("seed body");
            assert_eq!(body.ordinal(), 0);
            assert!(body.is_valid()?);
            assert_eq!(body.body_type()?, BodyType::Dynamic);
            assert!(body.position()?.is_valid());
            assert_eq!(view.query_count(), 0);
            Ok(())
        })
        .unwrap();

    assert_eq!(player.step().unwrap(), ReplayStatus::Advanced { frame: 1 });
    assert_eq!(player.seek(3).unwrap(), ReplayStatus::End { frame: 3 });
}

#[test]
fn mixer_identities_must_match_and_are_installed_before_first_step() {
    let _guard = replay_test_guard();
    let recording = mixer_recording();
    assert_eq!(
        recording.mixer_identities().friction(),
        Some(FRICTION_MIXER_ID)
    );

    assert!(matches!(
        ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default()
        ),
        Err(Error::ReplayMixerIdentityMismatch)
    ));

    let config = ReplayConfig::default()
        .with_friction_mixer(FRICTION_MIXER_ID, |a, b| a.coefficient.max(b.coefficient));
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        config,
    )
    .unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    assert!(!player.has_diverged());
}

#[test]
fn replay_mixer_panic_resumes_after_native_step_and_preserves_teardown() {
    let _guard = replay_test_guard();
    let recording = mixer_recording();
    let config = ReplayConfig::default().with_friction_mixer(FRICTION_MIXER_ID, |_, _| -> f32 {
        panic!("replay friction mixer panic");
    });
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        config,
    )
    .unwrap();
    let epoch = player.epoch();

    let panic = catch_unwind(AssertUnwindSafe(|| player.step()));

    assert!(panic.is_err());
    assert!(player.epoch() > epoch);
    assert!(player.is_healthy().unwrap());
    drop(player);
    assert!(
        !Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );
}

#[test]
fn replay_mixer_drop_panics_run_all_cleanup_before_resuming() {
    let _guard = replay_test_guard();
    let recording = dual_mixer_recording();
    let friction_dropped = Arc::new(AtomicUsize::new(0));
    let restitution_dropped = Arc::new(AtomicUsize::new(0));
    let friction_marker = PanicOnDrop(Arc::clone(&friction_dropped));
    let restitution_marker = PanicOnDrop(Arc::clone(&restitution_dropped));
    let config = ReplayConfig::default()
        .with_friction_mixer(FRICTION_MIXER_ID, move |a, b| {
            let _ = &friction_marker;
            a.coefficient.max(b.coefficient)
        })
        .with_restitution_mixer(RESTITUTION_MIXER_ID, move |a, b| {
            let _ = &restitution_marker;
            a.coefficient.max(b.coefficient)
        });
    let player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        config,
    )
    .unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| drop(player)));

    assert!(panic.is_err());
    assert_eq!(friction_dropped.load(Ordering::SeqCst), 1);
    assert_eq!(restitution_dropped.load(Ordering::SeqCst), 1);
    assert!(
        !Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );
    drop(
        boxdd::Foundation::initialize_default()
            .unwrap()
            .create_world(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap(),
    );
}

#[test]
fn replay_lease_excludes_worlds_and_is_released_on_rejection_and_drop() {
    let _guard = replay_test_guard();
    let recording = recorded_world(1);

    let live_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    assert!(matches!(
        ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default()
        ),
        Err(Error::FoundationActivity(
            FoundationActivityError::ReplayUnavailable { .. }
        ))
    ));
    assert!(
        !Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );
    drop(live_world);

    let def_while_replaying = boxdd::Foundation::get()
        .expect("Foundation must be initialized before constructing a WorldDef")
        .world_def();
    let player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    assert!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );
    assert!(matches!(
        ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            ReplayConfig::default()
        ),
        Err(Error::FoundationActivity(
            FoundationActivityError::ReplayActive
        ))
    ));
    let world_error = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(def_while_replaying)
        .err()
        .expect("replay exclusivity must reject ordinary world creation");
    assert!(matches!(
        world_error,
        Error::FoundationActivity(FoundationActivityError::ReplayActive)
    ));
    drop(player);
    assert!(
        !Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );
    drop(
        boxdd::Foundation::initialize_default()
            .unwrap()
            .create_world(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap(),
    );
}

#[test]
fn keyframe_policy_uses_the_shared_persistence_ceiling() {
    let _guard = replay_test_guard();
    assert_eq!(
        ReplayKeyframePolicy::new(0, 1),
        Err(Error::InvalidReplayKeyframePolicy)
    );
    assert_eq!(
        ReplayKeyframePolicy::new(ReplayKeyframePolicy::MAX_BYTES + 1, 1),
        Err(Error::InvalidReplayKeyframePolicy)
    );

    let maximum = ReplayKeyframePolicy::new(ReplayKeyframePolicy::MAX_BYTES, 1).unwrap();
    assert_eq!(
        maximum.budget_bytes(),
        ReplayKeyframePolicy::MAX_BYTES as usize
    );

    let recording = recorded_world(1);
    let player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    assert_eq!(
        player.keyframe_policy().budget_bytes,
        ReplayKeyframePolicy::MAX_BYTES as usize
    );
}

#[test]
fn keyframe_usage_includes_metadata_and_policy_reset_releases_it() {
    let _guard = replay_test_guard();
    let recording = recorded_empty_world(2);

    let mut constrained = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    constrained
        .set_keyframe_policy(ReplayKeyframePolicy::new(1, 1).unwrap())
        .unwrap();
    assert_eq!(
        constrained.step().unwrap(),
        ReplayStatus::Advanced { frame: 1 }
    );
    assert_eq!(constrained.keyframe_policy().allocated_bytes, 0);
    drop(constrained);

    let policy = ReplayKeyframePolicy::new(1024 * 1024, 1).unwrap();
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    player.set_keyframe_policy(policy).unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::Advanced { frame: 1 });

    let populated = player.keyframe_policy();
    assert!(populated.allocated_bytes > 0);
    assert!(populated.allocated_bytes <= populated.budget_bytes);

    player.set_keyframe_policy(policy).unwrap();
    assert_eq!(player.keyframe_policy().allocated_bytes, 0);
}

#[test]
fn every_authorized_mutation_attempt_advances_the_epoch() {
    let _guard = replay_test_guard();
    let recording = recorded_world(5);
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
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
    assert_eq!(player.seek(u64::MAX), Err(Error::ReplayFrameOutOfRange));
    assert!(player.epoch() > before_failure);
    assert_eq!(player.frame(), 0);
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
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    player.step().unwrap();
    let epoch = player.epoch();
    let frame = player.frame();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        player.draw(&mut PanicDrawer, DebugDrawOptions::default(), None)
    }));
    assert!(panic.is_err());
    assert_eq!(player.epoch(), epoch);
    assert_eq!(player.frame(), frame);
    assert!(player.is_healthy().unwrap());
    assert!(player.step().is_ok());
}

#[test]
fn replay_query_values_are_checked_and_do_not_expose_native_ids() {
    let _guard = replay_test_guard();
    let recording = query_recording();
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.query_count(), 1);
            let query = view.query(0)?.expect("recorded closest-ray query");
            assert_eq!(query.kind(), ReplayQueryKind::CastRayClosest);
            assert_eq!(query.origin(), Position::new(-2.0, 0.0));
            assert_eq!(query.translation(), Vec2::new(4.0, 0.0));
            assert_eq!(query.hit_count(), 1);

            let hit = query.hit(0)?.expect("recorded closest-ray hit");
            assert!(hit.point().expect("ray hit point").is_valid());
            assert!(hit.normal().expect("ray hit normal").is_valid());
            let fraction = hit.fraction().expect("ray hit fraction");
            assert!(fraction.is_finite());
            assert!((0.0..=1.0).contains(&fraction));
            assert!(query.hit(1)?.is_none());
            Ok(())
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
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();

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
                let query = view.query(index)?.expect("recorded query");
                assert_eq!(query.kind(), expected_kind);
                if expected_kind == ReplayQueryKind::ShapeRayCast {
                    assert_eq!(query.hit_count(), 0);
                    assert!(query.hit(0)?.is_none());
                }
            }
            Ok(())
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn destroyed_body_ordinals_remain_holes_when_native_slots_are_reused() {
    let _guard = replay_test_guard();
    let recording = body_slot_reuse_recording();
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();

    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.body_count(), 4);
            assert!(view.body(0)?.is_none());
            assert!(view.body(1)?.is_none());
            assert!(view.body(2)?.is_none());
            let live = view.body(3)?.expect("latest body creation ordinal");
            assert_eq!(live.ordinal(), 3);
            assert_eq!(live.body_type()?, BodyType::Dynamic);
            assert!(live.position()?.is_valid());
            Ok(())
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn replay_accepts_real_chain_joint_cascade_and_slot_reuse_lifecycle() {
    let _guard = replay_test_guard();
    let recording = chain_joint_lifecycle_recording();

    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    assert_eq!(player.info().frame_count, 1);
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 1 });
    player
        .with_view(|view| {
            assert_eq!(view.body_count(), 5);
            let live_bodies = (0..view.body_count())
                .map(|ordinal| view.body(ordinal))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            assert_eq!(live_bodies.len(), 3);
            for body in &live_bodies {
                assert!(body.is_valid()?);
            }
            Ok(())
        })
        .unwrap();
    assert!(!player.has_diverged());
}

#[test]
fn replay_api_does_not_alias_an_ordinary_world_error_surface() {
    let _guard = replay_test_guard();
    let recording = recorded_world(0);
    let mut player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    assert_eq!(player.step().unwrap(), ReplayStatus::End { frame: 0 });
    assert_ne!(Error::ReplayFrameOutOfRange, Error::InvalidBodyId);
    let bounds = player.info().bounds;
    assert!(bounds.is_valid());
    player
        .with_view(|view| {
            assert_eq!(view.info().bounds, bounds);
            Ok(())
        })
        .unwrap();
    assert_eq!(Position::ZERO, Position::new(0.0, 0.0));
}
