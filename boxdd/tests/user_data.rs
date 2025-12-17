use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boxdd::prelude::*;

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
fn events_view_defers_destroys() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().build());
    let id = body.id();

    world.with_contact_events_view(|_, _, _| {
        drop(body);
    });

    assert!(world.body(id).is_none());
}
