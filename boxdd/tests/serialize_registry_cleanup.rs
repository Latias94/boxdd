#![cfg(feature = "serialize")]

use boxdd::prelude::*;

#[test]
fn shape_flags_removed_when_shape_destroyed() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());

    let sdef = ShapeDef::builder().enable_contact_events(true).build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.25);
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
    let circle = shapes::circle([0.0_f32, 0.0], 0.25);
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

#[test]
fn chain_records_use_crate_owned_values_and_material_variants() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());

    let base_points = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(3.0, 0.0),
    ];
    let filter = Filter {
        category_bits: 0x0002,
        mask_bits: 0x0004,
        group_index: -7,
    };

    let default_material = boxdd::shapes::chain::ChainDef::builder()
        .points(base_points)
        .filter(filter)
        .enable_sensor_events(true)
        .materials(&[])
        .build();
    let single_material = boxdd::shapes::chain::ChainDef::builder()
        .points(base_points.map(|p| Vec2::new(p.x, p.y + 1.0)))
        .single_material(&boxdd::shapes::SurfaceMaterial::default().friction(0.3))
        .build();
    let multiple_materials = boxdd::shapes::chain::ChainDef::builder()
        .points(base_points.map(|p| Vec2::new(p.x, p.y + 2.0)))
        .materials(&[
            boxdd::shapes::SurfaceMaterial::default().friction(0.1),
            boxdd::shapes::SurfaceMaterial::default().friction(0.2),
            boxdd::shapes::SurfaceMaterial::default().friction(0.3),
            boxdd::shapes::SurfaceMaterial::default().friction(0.4),
        ])
        .build();

    let _ = world.create_chain_for_id(body, &default_material);
    let _ = world.create_chain_for_id(body, &single_material);
    let _ = world.create_chain_for_id(body, &multiple_materials);

    let records = world.chain_records();
    assert_eq!(records.len(), 3);

    let default_record = &records[0];
    assert_eq!(default_record.body.index1, body.index1);
    assert_eq!(default_record.body.world0, body.world0);
    assert_eq!(default_record.body.generation, body.generation);
    assert_eq!(default_record.filter, filter);
    assert_eq!(default_record.points, base_points.to_vec());
    assert!(default_record.enable_sensor_events);
    assert!(matches!(
        &default_record.materials,
        &boxdd::world::ChainMaterialsRecord::Default
    ));

    assert!(matches!(
        &records[1].materials,
        &boxdd::world::ChainMaterialsRecord::Single(_)
    ));

    match &records[2].materials {
        boxdd::world::ChainMaterialsRecord::Multiple(materials) => {
            assert_eq!(materials.len(), 4);
        }
        other => panic!("expected multiple materials record, got {other:?}"),
    }
}
