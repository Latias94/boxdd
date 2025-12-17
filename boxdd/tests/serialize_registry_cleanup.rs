#![cfg(feature = "serialize")]

use boxdd::prelude::*;

#[test]
fn shape_flags_removed_when_shape_destroyed() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());

    let sdef = ShapeDef::builder().enable_contact_events(true).build();
    let circle = boxdd_sys::ffi::b2Circle {
        center: Vec2::new(0.0, 0.0).into(),
        radius: 0.25,
    };
    let sid = world.create_circle_shape_for(body, &sdef, &circle);
    assert!(world.shape_flags(sid).is_some());

    world.destroy_shape_id(sid, true);
    assert!(world.shape_flags(sid).is_none());
}

#[test]
fn chain_record_removed_when_chain_destroyed() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());

    let chain_def = boxdd::shapes::chain::ChainDef::builder()
        .points([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]])
        .build();
    let chain = world.create_chain_for_owned(body, &chain_def);
    assert_eq!(world.chain_records().len(), 1);

    chain.destroy();
    assert_eq!(world.chain_records().len(), 0);
}

#[test]
fn registries_cleaned_when_body_destroyed() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());

    let sdef = ShapeDef::builder().enable_contact_events(true).build();
    let circle = boxdd_sys::ffi::b2Circle {
        center: Vec2::new(0.0, 0.0).into(),
        radius: 0.25,
    };
    let sid = world.create_circle_shape_for(body, &sdef, &circle);

    let chain_def = boxdd::shapes::chain::ChainDef::builder()
        .points([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]])
        .build();
    let _cid = world.create_chain_for_id(body, &chain_def);

    assert!(world.shape_flags(sid).is_some());
    assert_eq!(world.chain_records().len(), 1);
    assert_eq!(world.body_ids().len(), 1);

    world.destroy_body_id(body);
    assert!(world.shape_flags(sid).is_none());
    assert_eq!(world.chain_records().len(), 0);
    assert_eq!(world.body_ids().len(), 0);
}
