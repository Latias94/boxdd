use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(World: Send, Sync);
assert_not_impl_any!(Body<'static>: Send, Sync);
assert_not_impl_any!(Shape<'static>: Send, Sync);
assert_not_impl_any!(Joint<'static>: Send, Sync);
assert_not_impl_any!(Chain<'static>: Send, Sync);
assert_not_impl_any!(Query<'static>: Send, Sync);
assert_not_impl_any!(RecordingSession<'static>: Send, Sync);
assert_not_impl_any!(CompletedStep<'static>: Send, Sync);

#[derive(Clone)]
struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn public_world_and_capabilities_remain_single_threaded() {
    // Compile-time assertions above are the behavior under test.
}

#[test]
fn explicit_destroy_paths_drop_typed_user_data() {
    let body_drops = Arc::new(AtomicUsize::new(0));
    let shape_drops = Arc::new(AtomicUsize::new(0));
    let joint_drops = Arc::new(AtomicUsize::new(0));

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape = world
        .body(body_a)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let joint = world
        .create_revolute_joint_world(body_a, body_b, [0.0_f32, 0.0])
        .unwrap();

    {
        let mut body = world.body(body_a).unwrap();
        body.set_user_data(DropCounter(Arc::clone(&body_drops)))
            .unwrap();
    }
    {
        let mut shape = world.shape(shape).unwrap();
        shape
            .set_user_data(DropCounter(Arc::clone(&shape_drops)))
            .unwrap();
    }
    {
        let mut joint = world.joint(joint).unwrap();
        joint
            .set_user_data(DropCounter(Arc::clone(&joint_drops)))
            .unwrap();
    }

    world.joint(joint).unwrap().destroy(true).unwrap();
    assert_eq!(joint_drops.load(Ordering::SeqCst), 1);
    assert!(matches!(world.joint(joint), Err(Error::InvalidJointId)));

    world.shape(shape).unwrap().destroy(true).unwrap();
    assert_eq!(shape_drops.load(Ordering::SeqCst), 1);
    assert!(matches!(world.shape(shape), Err(Error::InvalidShapeId)));

    world.body(body_a).unwrap().destroy().unwrap();
    assert_eq!(body_drops.load(Ordering::SeqCst), 1);
    assert!(matches!(world.body(body_a), Err(Error::InvalidBodyId)));
}

#[test]
fn raw_user_data_pointer_replacement_drops_typed_value_without_owning_pointer() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut marker = 7_u32;

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
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();
    body.set_user_data(DropCounter(Arc::clone(&drops))).unwrap();

    let marker_ptr = (&mut marker as *mut u32).cast::<c_void>();
    body.set_user_data_ptr_raw(marker_ptr).unwrap();

    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(body.user_data_ptr_raw().unwrap(), marker_ptr);

    body.destroy().unwrap();

    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(marker, 7);
}
