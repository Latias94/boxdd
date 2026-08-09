use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::thread::{self, ThreadId};

use boxdd::prelude::*;

#[derive(Clone, Default)]
struct LocalDropLog {
    count: Rc<Cell<usize>>,
    threads: Rc<RefCell<Vec<ThreadId>>>,
}

impl LocalDropLog {
    fn probe(&self) -> LocalDropProbe {
        LocalDropProbe {
            count: Rc::clone(&self.count),
            threads: Rc::clone(&self.threads),
        }
    }

    fn assert_not_dropped(&self) {
        assert_eq!(self.count.get(), 0);
        assert!(self.threads.borrow().is_empty());
    }

    fn assert_dropped_once_on(&self, owner: ThreadId) {
        assert_eq!(self.count.get(), 1);
        assert_eq!(self.threads.borrow().as_slice(), &[owner]);
    }
}

struct LocalDropProbe {
    count: Rc<Cell<usize>>,
    threads: Rc<RefCell<Vec<ThreadId>>>,
}

impl Drop for LocalDropProbe {
    fn drop(&mut self) {
        self.count.set(self.count.get() + 1);
        self.threads.borrow_mut().push(thread::current().id());
    }
}

const USER_DATA_DROP_PANIC: &str = "intentional user-data destructor panic";

struct PanickingDropProbe(Rc<Cell<usize>>);

impl Drop for PanickingDropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
        std::panic::panic_any(USER_DATA_DROP_PANIC);
    }
}

fn body_pair(world: &mut World) -> (BodyId, BodyId) {
    (
        world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap(),
        world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap(),
    )
}

#[test]
fn raw_user_data_pointer_escape_hatches_are_explicit() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body_a, body_b) = body_pair(&mut world);
    let shape = world
        .body(body_a)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();

    let mut body_marker = 10_u32;
    let mut shape_marker = 20_u32;
    let mut joint_marker = 30_u32;
    let body_ptr = (&mut body_marker as *mut u32).cast::<c_void>();
    let shape_ptr = (&mut shape_marker as *mut u32).cast::<c_void>();
    let joint_ptr = (&mut joint_marker as *mut u32).cast::<c_void>();

    world
        .body(body_a)
        .unwrap()
        .set_user_data_ptr_raw(body_ptr)
        .unwrap();
    world
        .shape(shape)
        .unwrap()
        .set_user_data_ptr_raw(shape_ptr)
        .unwrap();
    world
        .joint(joint)
        .unwrap()
        .set_user_data_ptr_raw(joint_ptr)
        .unwrap();

    assert_eq!(
        world.body(body_a).unwrap().user_data_ptr_raw().unwrap(),
        body_ptr
    );
    assert_eq!(
        world.shape(shape).unwrap().user_data_ptr_raw().unwrap(),
        shape_ptr
    );
    assert_eq!(
        world.joint(joint).unwrap().user_data_ptr_raw().unwrap(),
        joint_ptr
    );

    world
        .body(body_a)
        .unwrap()
        .set_user_data_ptr_raw(core::ptr::null_mut())
        .unwrap();
    world
        .shape(shape)
        .unwrap()
        .set_user_data_ptr_raw(core::ptr::null_mut())
        .unwrap();
    world
        .joint(joint)
        .unwrap()
        .set_user_data_ptr_raw(core::ptr::null_mut())
        .unwrap();
    assert!(
        world
            .body(body_a)
            .unwrap()
            .user_data_ptr_raw()
            .unwrap()
            .is_null()
    );
}

#[test]
fn typed_user_data_checks_types_mutates_takes_and_recovers_after_panic() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();
    body.set_user_data(11_u32).unwrap();

    assert_eq!(
        body.with_user_data::<u32, _>(|value| *value).unwrap(),
        Some(11)
    );
    assert_eq!(
        body.with_user_data::<String, _>(String::len),
        Err(Error::UserDataTypeMismatch)
    );

    let panic = catch_unwind(AssertUnwindSafe(|| {
        body.with_user_data::<u32, ()>(|_| panic!("intentional user-data closure panic"))
            .unwrap();
    }));
    assert!(panic.is_err());

    body.with_user_data_mut::<u32, _>(|value| *value += 5)
        .unwrap();
    assert_eq!(body.take_user_data::<u32>().unwrap(), Some(16));
    assert_eq!(body.with_user_data::<u32, _>(|value| *value).unwrap(), None);
}

#[test]
fn replacing_and_clearing_user_data_drops_each_value_once_on_the_owner_thread() {
    let owner = thread::current().id();
    let replaced = LocalDropLog::default();
    let replacement = LocalDropLog::default();
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();

    body.set_user_data(replaced.probe()).unwrap();
    body.set_user_data(replacement.probe()).unwrap();
    replaced.assert_dropped_once_on(owner);
    replacement.assert_not_dropped();

    assert!(body.clear_user_data().unwrap());
    replacement.assert_dropped_once_on(owner);
    drop(world);
    replaced.assert_dropped_once_on(owner);
    replacement.assert_dropped_once_on(owner);
}

#[test]
fn replacing_and_clearing_user_data_resume_destructor_panics_after_committing() {
    let drops = Rc::new(Cell::new(0));
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();

    body.set_user_data(PanickingDropProbe(Rc::clone(&drops)))
        .unwrap();
    let replacement_panic = catch_unwind(AssertUnwindSafe(|| {
        body.set_user_data(41_u32).unwrap();
    }))
    .expect_err("replacing user data must resume its destructor panic");
    assert_eq!(
        replacement_panic.downcast_ref::<&'static str>(),
        Some(&USER_DATA_DROP_PANIC)
    );
    assert_eq!(drops.get(), 1);
    assert_eq!(
        body.with_user_data::<u32, _>(|value| *value).unwrap(),
        Some(41)
    );

    body.set_user_data(PanickingDropProbe(Rc::clone(&drops)))
        .unwrap();
    let clear_panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = body.clear_user_data().unwrap();
    }))
    .expect_err("clearing user data must resume its destructor panic");
    assert_eq!(
        clear_panic.downcast_ref::<&'static str>(),
        Some(&USER_DATA_DROP_PANIC)
    );
    assert_eq!(drops.get(), 2);
    assert!(body.user_data_ptr_raw().unwrap().is_null());
    assert_eq!(body.with_user_data::<u32, _>(|value| *value).unwrap(), None);
}

#[test]
fn explicit_object_destruction_drops_local_user_data_once() {
    let owner = thread::current().id();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
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
    world
        .body(body)
        .unwrap()
        .set_user_data(body_log.probe())
        .unwrap();
    world.body(body).unwrap().destroy().unwrap();
    body_log.assert_dropped_once_on(owner);

    let shape_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape = world
        .body(shape_body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    world
        .shape(shape)
        .unwrap()
        .set_user_data(shape_log.probe())
        .unwrap();
    world.shape(shape).unwrap().destroy(false).unwrap();
    shape_log.assert_dropped_once_on(owner);

    let (joint_body_a, joint_body_b) = body_pair(&mut world);
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(joint_body_a, joint_body_b).unwrap(),
        ))
        .unwrap();
    world
        .joint(joint)
        .unwrap()
        .set_user_data(joint_log.probe())
        .unwrap();
    world.joint(joint).unwrap().destroy(false).unwrap();
    joint_log.assert_dropped_once_on(owner);

    drop(world);
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
}

#[test]
fn body_destruction_cascades_attached_local_user_data_once() {
    let owner = thread::current().id();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let (body, other_body) = body_pair(&mut world);
    let shape = world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body, other_body).unwrap(),
        ))
        .unwrap();

    world
        .body(body)
        .unwrap()
        .set_user_data(body_log.probe())
        .unwrap();
    world
        .shape(shape)
        .unwrap()
        .set_user_data(shape_log.probe())
        .unwrap();
    world
        .joint(joint)
        .unwrap()
        .set_user_data(joint_log.probe())
        .unwrap();

    world.body(body).unwrap().destroy().unwrap();
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
    assert_eq!(world.body(body).err().unwrap(), Error::InvalidBodyId);
    assert_eq!(world.shape(shape).err().unwrap(), Error::InvalidShapeId);
    assert_eq!(world.joint(joint).err().unwrap(), Error::InvalidJointId);
}

#[test]
fn chain_and_world_destruction_release_every_registered_value_on_the_owner_thread() {
    let owner = thread::current().id();
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let chain_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let chain_id = world
        .body(chain_body)
        .unwrap()
        .create_chain(
            &ChainDef::builder()
                .points([
                    [-2.0_f32, 0.0],
                    [-1.0, 0.0],
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [2.0, 0.0],
                ])
                .build()
                .unwrap(),
        )
        .unwrap();
    let segments = world.chain(chain_id).unwrap().segments().unwrap();
    assert!(!segments.is_empty());
    let segment_logs: Vec<_> = segments
        .iter()
        .map(|&segment| {
            let log = LocalDropLog::default();
            world
                .shape(segment)
                .unwrap()
                .set_user_data(log.probe())
                .unwrap();
            log
        })
        .collect();

    world.chain(chain_id).unwrap().destroy().unwrap();
    for log in &segment_logs {
        log.assert_dropped_once_on(owner);
    }

    let world_log = LocalDropLog::default();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
    let (body_a, body_b) = body_pair(&mut world);
    let shape = world
        .body(body_a)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    world.set_user_data(world_log.probe()).unwrap();
    world
        .body(body_a)
        .unwrap()
        .set_user_data(body_log.probe())
        .unwrap();
    world
        .shape(shape)
        .unwrap()
        .set_user_data(shape_log.probe())
        .unwrap();
    world
        .joint(joint)
        .unwrap()
        .set_user_data(joint_log.probe())
        .unwrap();

    world_log.assert_not_dropped();
    body_log.assert_not_dropped();
    shape_log.assert_not_dropped();
    joint_log.assert_not_dropped();
    drop(world);

    world_log.assert_dropped_once_on(owner);
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
    for log in &segment_logs {
        log.assert_dropped_once_on(owner);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod outer_unwind_subprocess {
    use super::*;
    use std::process::Command;

    const CHILD_CASE: &str = "BOXDD_OUTER_UNWIND_USER_DATA_CASE";
    const PRIMARY_PANIC: &str = "outer user-data unwind remains primary";
    const TEST_NAME: &str =
        "outer_unwind_subprocess::user_data_destructors_during_outer_unwind_do_not_abort";
    const CASES: &[&str] = &[
        "world",
        "body",
        "shape",
        "joint",
        "world-rejected",
        "recording-capabilities-rejected",
        "accessors-rejected",
    ];

    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    fn new_world() -> World {
        let foundation = boxdd::Foundation::initialize_default().unwrap();
        foundation.create_world(foundation.world_def()).unwrap()
    }

    fn during_outer_unwind(invoke: impl FnOnce()) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _invoke = InvokeOnDrop(Some(invoke));
            std::panic::panic_any(PRIMARY_PANIC);
        }));
        let payload = result.expect_err("the outer panic must keep unwinding");
        assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
    }

    fn record(completed: &Cell<usize>, succeeded: bool) {
        if succeeded {
            completed.set(completed.get() + 1);
        }
    }

    fn terminalize(world: &mut World) {
        let snapshot = world.snapshot().unwrap();
        let error = world
            .prepare_restore(&snapshot)
            .unwrap()
            .commit_with(|_| Err(Error::SnapshotManifestMismatch))
            .unwrap_err();
        assert_eq!(error, Error::SnapshotManifestMismatch);
    }

    fn rejected_world_input_case() {
        let drops = Rc::new(Cell::new(0));
        let rejected = Rc::new(Cell::new(false));
        let observed_drops = Rc::clone(&drops);
        let observed_rejected = Rc::clone(&rejected);
        let mut world = new_world();
        terminalize(&mut world);

        during_outer_unwind(move || {
            observed_rejected.set(matches!(
                world.set_user_data(PanickingDropProbe(observed_drops)),
                Err(Error::WorldDestroyed)
            ));
        });

        assert_eq!(drops.get(), 1);
        assert!(rejected.get());
    }

    struct RecordingFixture {
        world: World,
        body: BodyId,
        shape: ShapeId,
        joint: JointId,
    }

    fn recording_fixture() -> RecordingFixture {
        let mut world = new_world();
        let (body, other_body) = body_pair(&mut world);
        let shape = world
            .body(body)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        let joint = world
            .create_distance_joint(&DistanceJointDef::new(
                world.joint_base(body, other_body).unwrap(),
            ))
            .unwrap();
        RecordingFixture {
            world,
            body,
            shape,
            joint,
        }
    }

    fn minimum_recording_limit() -> u32 {
        const SEARCH_CEILING_BYTES: u32 = 1024 * 1024;

        let mut lower = 1;
        let mut upper = SEARCH_CEILING_BYTES;
        while lower < upper {
            let midpoint = lower + (upper - lower) / 2;
            let mut fixture = recording_fixture();
            match fixture
                .world
                .start_recording(RecordingLimits::new(u64::from(midpoint)).unwrap())
            {
                Ok(session) => {
                    drop(session);
                    upper = midpoint;
                }
                Err(Error::RecordingLimitExceeded) => lower = midpoint + 1,
                Err(error) => {
                    panic!("unexpected recording-start error at {midpoint} bytes: {error}")
                }
            }
        }
        lower
    }

    fn assert_rejected_body_input(limit: u32) {
        let mut fixture = recording_fixture();
        let body_id = fixture.body;
        let mut session = fixture
            .world
            .start_recording(RecordingLimits::new(u64::from(limit)).unwrap())
            .unwrap();
        let mut body = session.body(body_id).unwrap();
        assert_eq!(
            body.set_linear_velocity([1.0_f32, 0.0]),
            Err(Error::RecordingLimitExceeded)
        );
        let drops = Rc::new(Cell::new(0));
        let rejected = Rc::new(Cell::new(false));
        let observed_drops = Rc::clone(&drops);
        let observed_rejected = Rc::clone(&rejected);

        during_outer_unwind(|| {
            observed_rejected.set(matches!(
                body.set_user_data(PanickingDropProbe(observed_drops)),
                Err(Error::RecordingLimitExceeded)
            ));
        });

        assert_eq!(drops.get(), 1);
        assert!(rejected.get());
    }

    fn assert_rejected_shape_input(limit: u32) {
        let mut fixture = recording_fixture();
        let shape_id = fixture.shape;
        let mut session = fixture
            .world
            .start_recording(RecordingLimits::new(u64::from(limit)).unwrap())
            .unwrap();
        let mut shape = session.shape(shape_id).unwrap();
        assert_eq!(shape.set_friction(0.75), Err(Error::RecordingLimitExceeded));
        let drops = Rc::new(Cell::new(0));
        let rejected = Rc::new(Cell::new(false));
        let observed_drops = Rc::clone(&drops);
        let observed_rejected = Rc::clone(&rejected);

        during_outer_unwind(|| {
            observed_rejected.set(matches!(
                shape.set_user_data(PanickingDropProbe(observed_drops)),
                Err(Error::RecordingLimitExceeded)
            ));
        });

        assert_eq!(drops.get(), 1);
        assert!(rejected.get());
    }

    fn assert_rejected_joint_input(limit: u32) {
        let mut fixture = recording_fixture();
        let joint_id = fixture.joint;
        let mut session = fixture
            .world
            .start_recording(RecordingLimits::new(u64::from(limit)).unwrap())
            .unwrap();
        let mut joint = session.joint(joint_id).unwrap();
        assert_eq!(
            joint.set_collide_connected(true),
            Err(Error::RecordingLimitExceeded)
        );
        let drops = Rc::new(Cell::new(0));
        let rejected = Rc::new(Cell::new(false));
        let observed_drops = Rc::clone(&drops);
        let observed_rejected = Rc::clone(&rejected);

        during_outer_unwind(|| {
            observed_rejected.set(matches!(
                joint.set_user_data(PanickingDropProbe(observed_drops)),
                Err(Error::RecordingLimitExceeded)
            ));
        });

        assert_eq!(drops.get(), 1);
        assert!(rejected.get());
    }

    fn rejected_recording_capability_inputs_case() {
        let limit = minimum_recording_limit();
        assert_rejected_body_input(limit);
        assert_rejected_shape_input(limit);
        assert_rejected_joint_input(limit);
    }

    fn rejected_user_data_accessors_case() {
        let drops = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observed_drops = Rc::clone(&drops);
        let observed_completed = Rc::clone(&completed);
        let mut world = new_world();
        let (body_id, other_body) = body_pair(&mut world);
        let shape_id = world
            .body(body_id)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        let joint_id = world
            .create_distance_joint(&DistanceJointDef::new(
                world.joint_base(body_id, other_body).unwrap(),
            ))
            .unwrap();
        world.set_user_data(1_u32).unwrap();
        world.body(body_id).unwrap().set_user_data(2_u32).unwrap();
        world.shape(shape_id).unwrap().set_user_data(3_u32).unwrap();
        world.joint(joint_id).unwrap().set_user_data(4_u32).unwrap();

        during_outer_unwind(move || {
            let probe = PanickingDropProbe(Rc::clone(&observed_drops));
            record(
                &observed_completed,
                matches!(
                    world.with_user_data::<u64, _>(move |_| {
                        let _ = &probe;
                    }),
                    Err(Error::UserDataTypeMismatch)
                ),
            );

            {
                let mut body = world.body(body_id).unwrap();
                let probe = PanickingDropProbe(Rc::clone(&observed_drops));
                record(
                    &observed_completed,
                    matches!(
                        body.with_user_data::<u64, _>(move |_| {
                            let _ = &probe;
                        }),
                        Err(Error::UserDataTypeMismatch)
                    ),
                );
                let probe = PanickingDropProbe(Rc::clone(&observed_drops));
                record(
                    &observed_completed,
                    matches!(
                        body.with_user_data_mut::<u64, _>(move |_| {
                            let _ = &probe;
                        }),
                        Err(Error::UserDataTypeMismatch)
                    ),
                );
            }

            {
                let mut shape = world.shape(shape_id).unwrap();
                let probe = PanickingDropProbe(Rc::clone(&observed_drops));
                record(
                    &observed_completed,
                    matches!(
                        shape.with_user_data::<u64, _>(move |_| {
                            let _ = &probe;
                        }),
                        Err(Error::UserDataTypeMismatch)
                    ),
                );
                let probe = PanickingDropProbe(Rc::clone(&observed_drops));
                record(
                    &observed_completed,
                    matches!(
                        shape.with_user_data_mut::<u64, _>(move |_| {
                            let _ = &probe;
                        }),
                        Err(Error::UserDataTypeMismatch)
                    ),
                );
            }

            let mut joint = world.joint(joint_id).unwrap();
            let probe = PanickingDropProbe(Rc::clone(&observed_drops));
            record(
                &observed_completed,
                matches!(
                    joint.with_user_data::<u64, _>(move |_| {
                        let _ = &probe;
                    }),
                    Err(Error::UserDataTypeMismatch)
                ),
            );
            let probe = PanickingDropProbe(Rc::clone(&observed_drops));
            record(
                &observed_completed,
                matches!(
                    joint.with_user_data_mut::<u64, _>(move |_| {
                        let _ = &probe;
                    }),
                    Err(Error::UserDataTypeMismatch)
                ),
            );
        });

        assert_eq!(completed.get(), 7);
        assert_eq!(drops.get(), 7);
    }

    fn world_case() {
        let drops = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observed_drops = Rc::clone(&drops);
        let observed_completed = Rc::clone(&completed);
        let mut world = new_world();
        world
            .set_user_data(PanickingDropProbe(Rc::clone(&drops)))
            .unwrap();

        during_outer_unwind(move || {
            record(&observed_completed, world.set_user_data(()).is_ok());
            record(
                &observed_completed,
                world
                    .set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                matches!(world.clear_user_data(), Ok(true)),
            );
        });

        assert_eq!(completed.get(), 3);
        assert_eq!(drops.get(), 2);
    }

    fn body_case() {
        let drops = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observed_drops = Rc::clone(&drops);
        let observed_completed = Rc::clone(&completed);
        let mut world = new_world();
        let body_id = world.create_body(world.body_def()).unwrap();
        world
            .body(body_id)
            .unwrap()
            .set_user_data(PanickingDropProbe(Rc::clone(&drops)))
            .unwrap();

        during_outer_unwind(move || {
            let mut body = world.body(body_id).unwrap();
            record(&observed_completed, body.set_user_data(()).is_ok());
            record(
                &observed_completed,
                body.set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                body.set_user_data_ptr_raw(core::ptr::null_mut()).is_ok(),
            );
            record(
                &observed_completed,
                body.set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                matches!(body.clear_user_data(), Ok(true)),
            );
        });

        assert_eq!(completed.get(), 5);
        assert_eq!(drops.get(), 3);
    }

    fn shape_case() {
        let drops = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observed_drops = Rc::clone(&drops);
        let observed_completed = Rc::clone(&completed);
        let mut world = new_world();
        let body_id = world.create_body(world.body_def()).unwrap();
        let shape_id = world
            .body(body_id)
            .unwrap()
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        world
            .shape(shape_id)
            .unwrap()
            .set_user_data(PanickingDropProbe(Rc::clone(&drops)))
            .unwrap();

        during_outer_unwind(move || {
            let mut shape = world.shape(shape_id).unwrap();
            record(&observed_completed, shape.set_user_data(()).is_ok());
            record(
                &observed_completed,
                shape
                    .set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                shape.set_user_data_ptr_raw(core::ptr::null_mut()).is_ok(),
            );
            record(
                &observed_completed,
                shape
                    .set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                matches!(shape.clear_user_data(), Ok(true)),
            );
        });

        assert_eq!(completed.get(), 5);
        assert_eq!(drops.get(), 3);
    }

    fn joint_case() {
        let drops = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let observed_drops = Rc::clone(&drops);
        let observed_completed = Rc::clone(&completed);
        let mut world = new_world();
        let (body_a, body_b) = body_pair(&mut world);
        let joint_id = world
            .create_distance_joint(&DistanceJointDef::new(
                world.joint_base(body_a, body_b).unwrap(),
            ))
            .unwrap();
        world
            .joint(joint_id)
            .unwrap()
            .set_user_data(PanickingDropProbe(Rc::clone(&drops)))
            .unwrap();

        during_outer_unwind(move || {
            let mut joint = world.joint(joint_id).unwrap();
            record(&observed_completed, joint.set_user_data(()).is_ok());
            record(
                &observed_completed,
                joint
                    .set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                joint.set_user_data_ptr_raw(core::ptr::null_mut()).is_ok(),
            );
            record(
                &observed_completed,
                joint
                    .set_user_data(PanickingDropProbe(Rc::clone(&observed_drops)))
                    .is_ok(),
            );
            record(
                &observed_completed,
                matches!(joint.clear_user_data(), Ok(true)),
            );
        });

        assert_eq!(completed.get(), 5);
        assert_eq!(drops.get(), 3);
    }

    fn run_child(case: &str) {
        match case {
            "world" => world_case(),
            "body" => body_case(),
            "shape" => shape_case(),
            "joint" => joint_case(),
            "world-rejected" => rejected_world_input_case(),
            "recording-capabilities-rejected" => rejected_recording_capability_inputs_case(),
            "accessors-rejected" => rejected_user_data_accessors_case(),
            other => panic!("unknown outer-unwind user-data child case: {other}"),
        }
        eprintln!("boxdd-outer-unwind-user-data: completed {case}");
    }

    #[test]
    fn user_data_destructors_during_outer_unwind_do_not_abort() {
        if let Some(case) = std::env::var_os(CHILD_CASE) {
            if case == "all" {
                for case in CASES {
                    run_child(case);
                }
            } else {
                run_child(&case.to_string_lossy());
            }
            return;
        }

        let output =
            Command::new(std::env::current_exe().expect("test executable path must be available"))
                .args(["--exact", TEST_NAME, "--nocapture"])
                .env(CHILD_CASE, "all")
                .output()
                .expect("outer-unwind user-data child process must start");

        assert!(
            output.status.success(),
            "outer-unwind user-data child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        for case in CASES {
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains(&format!("boxdd-outer-unwind-user-data: completed {case}")),
                "outer-unwind user-data child {case} did not complete its assertion path\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
