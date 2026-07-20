use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, ThreadId};

use boxdd::prelude::*;
use boxdd_sys::ffi;

#[derive(Clone)]
struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

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

#[test]
fn typed_user_data_drops_with_owned_body() {
    let drops = Arc::new(AtomicUsize::new(0));

    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    body.set_user_data(DropCounter(Arc::clone(&drops)));
    drop(body);

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn raw_user_data_pointer_escape_hatches_are_explicit() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    let body_b = world.create_body_id(BodyBuilder::new().build());
    let mut shape = world.create_circle_shape_for_owned(
        body.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut joint = world
        .revolute(body.id(), body_b)
        .anchor_world([0.0_f32, 0.0])
        .build_owned();

    let mut body_marker = 10_u32;
    let mut shape_marker = 20_u32;
    let mut joint_marker = 30_u32;
    let body_ptr = (&mut body_marker as *mut u32).cast::<c_void>();
    let shape_ptr = (&mut shape_marker as *mut u32).cast::<c_void>();
    let joint_ptr = (&mut joint_marker as *mut u32).cast::<c_void>();

    unsafe {
        body.set_user_data_ptr_raw(body_ptr);
        shape.set_user_data_ptr_raw(shape_ptr);
        joint.set_user_data_ptr_raw(joint_ptr);
    }

    assert_eq!(body.user_data_ptr_raw(), body_ptr);
    assert_eq!(shape.user_data_ptr_raw(), shape_ptr);
    assert_eq!(joint.user_data_ptr_raw(), joint_ptr);

    unsafe {
        body.try_set_user_data_ptr_raw(core::ptr::null_mut())
            .unwrap();
        shape
            .try_set_user_data_ptr_raw(core::ptr::null_mut())
            .unwrap();
        joint
            .try_set_user_data_ptr_raw(core::ptr::null_mut())
            .unwrap();
    }

    assert!(body.try_user_data_ptr_raw().unwrap().is_null());
    assert!(shape.try_user_data_ptr_raw().unwrap().is_null());
    assert!(joint.try_user_data_ptr_raw().unwrap().is_null());
}

#[test]
fn events_view_defers_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let id = body.id();

    world.with_contact_events_view(|_, _, _| {
        drop(body);
    });

    assert!(world.body(id).is_none());
}

#[test]
fn raw_events_view_defers_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let id = body.id();

    unsafe {
        world.with_contact_events_raw(|begin, end, hit| {
            let _ = (begin.len(), end.len(), hit.len());
            drop(body);
        });
    }

    assert!(world.body(id).is_none());
}

#[test]
fn raw_body_events_view_defers_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let id = body.id();

    unsafe {
        world.with_body_events_raw(|moves| {
            let _ = moves.len();
            drop(body);
        });
    }

    assert!(world.body(id).is_none());
}

#[test]
fn raw_sensor_events_view_defers_shape_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::builder().sensor(true).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let id = shape.id();

    unsafe {
        world.with_sensor_events_raw(|begin, end| {
            let _ = (begin.len(), end.len());
            drop(shape);
        });
    }

    assert!(world.shape(id).is_none());
}

#[test]
fn raw_joint_events_view_defers_joint_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = world.create_body_id(BodyBuilder::new().build());
    let body_b = world.create_body_id(BodyBuilder::new().build());
    let joint = world
        .revolute(body_a, body_b)
        .anchor_world([0.0_f32, 0.0])
        .build_owned();
    let id = joint.id();

    unsafe {
        world.with_joint_events_raw(|events| {
            let _ = events.len();
            drop(joint);
        });
    }

    assert!(world.joint(id).is_none());
}

#[test]
fn nested_raw_event_views_delay_destroy_until_outermost_scope() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let id = body.id();

    unsafe {
        world.with_contact_events_raw(|begin, end, hit| {
            let _ = (begin.len(), end.len(), hit.len());

            world.with_body_events_raw(|moves| {
                let _ = moves.len();
                drop(body);
            });

            assert!(ffi::b2Body_IsValid(id.unbind().into_ffi()));
        });
    }

    assert!(world.body(id).is_none());
}

#[test]
fn readonly_userdata_access_can_reenter_distinct_entries() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body_a = world.create_body_owned(BodyBuilder::new().build());
    let mut body_b = world.create_body_owned(BodyBuilder::new().build());
    let mut shape = world.create_circle_shape_for_owned(
        body_a.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut joint = world
        .revolute(body_a.id(), body_b.id())
        .anchor_world([0.0_f32, 0.0])
        .build_owned();

    world.set_user_data(10_u32);
    body_a.set_user_data(20_u32);
    body_b.set_user_data(30_u32);
    shape.set_user_data(40_u32);
    joint.set_user_data(50_u32);

    let values = body_a
        .try_with_user_data::<u32, _>(|body_a_value| {
            let body_b_value = body_b
                .try_with_user_data::<u32, _>(|value| *value)
                .unwrap()
                .unwrap();
            let shape_value = shape
                .try_with_user_data::<u32, _>(|value| *value)
                .unwrap()
                .unwrap();
            let joint_value = joint
                .try_with_user_data::<u32, _>(|value| *value)
                .unwrap()
                .unwrap();
            let world_value = world
                .try_with_user_data::<u32, _>(|value| *value)
                .unwrap()
                .unwrap();
            (
                *body_a_value,
                body_b_value,
                shape_value,
                joint_value,
                world_value,
            )
        })
        .unwrap()
        .unwrap();

    assert_eq!(values, (20, 30, 40, 50, 10));
}

#[test]
fn unrelated_userdata_borrow_does_not_delay_owned_drop() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut borrowed = world.create_body_owned(BodyBuilder::new().build());
    let doomed = world.create_body_owned(BodyBuilder::new().build());
    let doomed_id = doomed.id();
    borrowed.set_user_data(7_u32);

    borrowed
        .try_with_user_data::<u32, _>(|value| {
            assert_eq!(*value, 7);
            drop(doomed);
            assert_eq!(
                world.try_body_position(doomed_id),
                Err(ApiError::InvalidBodyId)
            );
        })
        .unwrap()
        .unwrap();
}

#[test]
fn same_userdata_entry_rejects_mutable_reentry() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    body.set_user_data(7_u32);

    let mut alias = world.body(body.id()).unwrap();
    let nested = body
        .try_with_user_data::<u32, _>(|_| {
            alias.try_with_user_data_mut::<u32, _>(|value| *value += 1)
        })
        .unwrap()
        .unwrap();

    assert_eq!(nested.unwrap_err(), ApiError::ReentrantAccess);
    assert_eq!(
        body.try_with_user_data::<u32, _>(|value| *value).unwrap(),
        Some(7)
    );
}

#[test]
fn userdata_entry_recovers_after_panicking_closure() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    body.set_user_data(11_u32);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        body.with_user_data::<u32, ()>(|_| panic!("intentional userdata closure panic"));
    }));
    assert!(panic.is_err());

    body.try_with_user_data_mut::<u32, _>(|value| *value += 5)
        .unwrap()
        .unwrap();
    assert_eq!(
        body.try_with_user_data::<u32, _>(|value| *value).unwrap(),
        Some(16)
    );
}

#[test]
fn replacing_local_userdata_drops_each_value_once_on_owner_thread() {
    let owner = thread::current().id();
    let replaced = LocalDropLog::default();
    let replacement = LocalDropLog::default();
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());

    body.set_user_data(replaced.probe());
    body.set_user_data(replacement.probe());

    replaced.assert_dropped_once_on(owner);
    replacement.assert_not_dropped();

    assert!(body.clear_user_data());
    replacement.assert_dropped_once_on(owner);

    drop(body);
    drop(world);
    replaced.assert_dropped_once_on(owner);
    replacement.assert_dropped_once_on(owner);
}

#[test]
fn explicit_object_destroys_drop_local_userdata_once_on_owner_thread() {
    let owner = thread::current().id();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
    let mut world = World::new(WorldDef::default()).unwrap();

    let mut body = world.create_body_owned(BodyBuilder::new().build());
    body.set_user_data(body_log.probe());
    body.destroy();
    body_log.assert_dropped_once_on(owner);

    let shape_body = world.create_body_id(BodyBuilder::new().build());
    let mut shape = world.create_circle_shape_for_owned(
        shape_body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    shape.set_user_data(shape_log.probe());
    shape.destroy(false);
    shape_log.assert_dropped_once_on(owner);

    let joint_body_a = world.create_body_id(BodyBuilder::new().build());
    let joint_body_b = world.create_body_id(BodyBuilder::new().build());
    let mut joint = world
        .revolute(joint_body_a, joint_body_b)
        .anchor_world([0.0_f32, 0.0])
        .build_owned();
    joint.set_user_data(joint_log.probe());
    joint.destroy(false);
    joint_log.assert_dropped_once_on(owner);

    drop(world);
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
}

#[test]
fn body_destroy_cascades_attached_local_userdata_once() {
    let owner = thread::current().id();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    let other_body = world.create_body_id(BodyBuilder::new().build());
    let mut shape = world.create_circle_shape_for_owned(
        body.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut joint = world
        .revolute(body.id(), other_body)
        .anchor_world([0.0_f32, 0.0])
        .build_owned();

    body.set_user_data(body_log.probe());
    shape.set_user_data(shape_log.probe());
    joint.set_user_data(joint_log.probe());

    body.destroy();

    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
    assert!(!shape.is_valid());
    assert!(!joint.is_valid());

    drop(shape);
    drop(joint);
    drop(world);
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
}

#[test]
fn chain_destroy_drops_all_segment_userdata_once() {
    let owner = thread::current().id();
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let chain_def = ChainDef::builder()
        .points([
            [-2.0_f32, 0.0],
            [-1.0, 0.0],
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
        ])
        .build();
    let chain = world.create_chain_for_owned(body, &chain_def);
    let segments = chain.segments();
    assert!(!segments.is_empty());

    let logs: Vec<_> = segments
        .iter()
        .map(|&segment| {
            let log = LocalDropLog::default();
            world
                .shape(segment)
                .expect("chain segment should be valid")
                .set_user_data(log.probe());
            log
        })
        .collect();

    chain.destroy();
    for log in &logs {
        log.assert_dropped_once_on(owner);
    }

    drop(world);
    for log in &logs {
        log.assert_dropped_once_on(owner);
    }
}

#[test]
fn final_world_drop_releases_all_local_userdata_on_owner_thread() {
    let owner = thread::current().id();
    let world_log = LocalDropLog::default();
    let body_log = LocalDropLog::default();
    let shape_log = LocalDropLog::default();
    let joint_log = LocalDropLog::default();
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = world.create_body_id(BodyBuilder::new().build());
    let body_b = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for(
        body_a,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let joint = world
        .revolute(body_a, body_b)
        .anchor_world([0.0_f32, 0.0])
        .build_owned()
        .into_id();

    world.set_user_data(world_log.probe());
    world
        .body(body_a)
        .expect("body should be valid")
        .set_user_data(body_log.probe());
    world
        .shape(shape)
        .expect("shape should be valid")
        .set_user_data(shape_log.probe());
    world
        .joint(joint)
        .expect("joint should be valid")
        .set_user_data(joint_log.probe());

    world_log.assert_not_dropped();
    body_log.assert_not_dropped();
    shape_log.assert_not_dropped();
    joint_log.assert_not_dropped();

    drop(world);

    world_log.assert_dropped_once_on(owner);
    body_log.assert_dropped_once_on(owner);
    shape_log.assert_dropped_once_on(owner);
    joint_log.assert_dropped_once_on(owner);
}
