use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;
use boxdd_sys::ffi;

#[derive(Clone)]
struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
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
fn callback_world_can_read_shape_user_data() {
    let called = Arc::new(AtomicUsize::new(0));

    let mut world = World::new(WorldDef::builder().gravity([0.0, 0.0]).build()).unwrap();
    world.set_custom_filter_with_ctx({
        let called = Arc::clone(&called);
        move |cw: &CallbackWorld, a, b| {
            called.fetch_add(1, Ordering::SeqCst);
            let av = cw.with_shape_user_data::<u32, _>(a, |v| *v).unwrap();
            let bv = cw.with_shape_user_data::<u32, _>(b, |v| *v).unwrap();
            let _ = (av, bv);
            true
        }
    });

    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0, 0.0])
            .build(),
    );

    let sdef = ShapeDef::builder()
        .enable_custom_filtering(true)
        .density(1.0)
        .build();

    let poly = shapes::box_polygon(0.5, 0.5);
    let mut shape_a = world.create_polygon_shape_for_owned(body_a, &sdef, &poly);
    let mut shape_b = world.create_polygon_shape_for_owned(body_b, &sdef, &poly);

    shape_a.set_user_data(1u32);
    shape_b.set_user_data(2u32);

    world.step(1.0 / 60.0, 1);

    assert!(called.load(Ordering::SeqCst) > 0);
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

            assert!(ffi::b2Body_IsValid(id));
        });
    }

    assert!(world.body(id).is_none());
}
