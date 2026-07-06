use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(World: Send, Sync);
assert_not_impl_any!(WorldHandle: Send, Sync);
assert_not_impl_any!(OwnedBody: Send, Sync);
assert_not_impl_any!(OwnedShape: Send, Sync);
assert_not_impl_any!(OwnedJoint: Send, Sync);
assert_not_impl_any!(OwnedChain: Send, Sync);

#[derive(Clone)]
struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn public_world_and_owned_handles_remain_single_threaded() {
    // Compile-time assertions above are the behavior under test.
}

#[test]
fn explicit_destroy_paths_drop_typed_user_data() {
    let body_drops = Arc::new(AtomicUsize::new(0));
    let shape_drops = Arc::new(AtomicUsize::new(0));
    let joint_drops = Arc::new(AtomicUsize::new(0));

    let mut world = World::new(WorldDef::default()).unwrap();
    let body_a = world.create_body_id(BodyBuilder::new().build());
    let body_b = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for(
        body_a,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let joint = world.create_revolute_joint_world_id(body_a, body_b, [0.0_f32, 0.0]);

    {
        let mut body = world.body(body_a).unwrap();
        body.set_user_data(DropCounter(Arc::clone(&body_drops)));
    }
    {
        let mut shape = world.shape(shape).unwrap();
        shape.set_user_data(DropCounter(Arc::clone(&shape_drops)));
    }
    {
        let mut joint = world.joint(joint).unwrap();
        joint.set_user_data(DropCounter(Arc::clone(&joint_drops)));
    }

    world.destroy_joint_id(joint, true);
    assert_eq!(joint_drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        world.try_joint(joint).unwrap_err(),
        ApiError::InvalidJointId
    );

    world.destroy_shape_id(shape, true);
    assert_eq!(shape_drops.load(Ordering::SeqCst), 1);
    assert_eq!(world.try_shape(shape).err(), Some(ApiError::InvalidShapeId));

    world.destroy_body_id(body_a);
    assert_eq!(body_drops.load(Ordering::SeqCst), 1);
    assert_eq!(world.try_body(body_a).err(), Some(ApiError::InvalidBodyId));
}

#[test]
fn raw_user_data_pointer_replacement_drops_typed_value_without_owning_pointer() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut marker = 7_u32;

    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().build());
    body.set_user_data(DropCounter(Arc::clone(&drops)));

    let marker_ptr = (&mut marker as *mut u32).cast::<c_void>();
    unsafe {
        body.set_user_data_ptr_raw(marker_ptr);
    }

    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(body.user_data_ptr_raw(), marker_ptr);

    drop(body);

    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(marker, 7);
}
