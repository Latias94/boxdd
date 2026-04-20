use boxdd::{BodyBuilder, Filter, Vec2, World, WorldDef};

#[test]
fn chain_def_single_material_is_owned_and_clone_safe() {
    let mut world = World::new(WorldDef::default()).expect("create world");
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 0.0]).build());

    let def = {
        let m = boxdd::shapes::SurfaceMaterial::default().with_friction(0.3);
        let def = boxdd::shapes::chain::ChainDef::builder()
            // Minimal non-loop chain: 4 points (includes ghost points at ends)
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .single_material(&m)
            .build();
        def.clone()
    };

    let chain = world.create_chain_for_id(body, &def);
    world.destroy_chain_id(chain);
}

#[test]
fn chain_def_materials_empty_uses_default() {
    let mut world = World::new(WorldDef::default()).expect("create world");
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 0.0]).build());

    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        // Empty slice should mean "use upstream default material".
        .materials(&[])
        .build();

    let chain = world.create_chain_for_id(body, &def);
    world.destroy_chain_id(chain);
}

#[test]
fn chain_def_filter_uses_safe_filter_type() {
    let mut world = World::new(WorldDef::default()).expect("create world");
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 0.0]).build());

    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .filter(Filter {
            category_bits: 0x0002,
            mask_bits: 0x0004,
            group_index: -1,
        })
        .build();

    let chain = world.create_chain_for_id(body, &def);
    world.destroy_chain_id(chain);
}

#[cfg(feature = "serialize")]
#[test]
fn scene_snapshot_roundtrip_includes_chains() {
    let mut world = World::new(WorldDef::default()).expect("create world");
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 0.0]).build());

    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();
    let _chain = world.create_chain_for_id(body, &def);

    let scene = boxdd::serialize::SceneSnapshot::take(&world);
    assert_eq!(scene.chains.len(), 1);

    let world2 = scene.rebuild();
    let scene2 = boxdd::serialize::SceneSnapshot::take(&world2);
    assert_eq!(scene2.chains.len(), 1);
}

#[cfg(feature = "serialize")]
#[test]
fn scene_snapshot_roundtrip_includes_scoped_chains() {
    let mut world = World::new(WorldDef::default()).expect("create world");
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 0.0]).build());

    let def = boxdd::shapes::chain::ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();

    {
        let mut b = world.body(body).unwrap();
        let _ = b.create_chain(&def);
    }

    let scene = boxdd::serialize::SceneSnapshot::take(&world);
    assert_eq!(scene.chains.len(), 1);

    let world2 = scene.rebuild();
    let scene2 = boxdd::serialize::SceneSnapshot::take(&world2);
    assert_eq!(scene2.chains.len(), 1);
}
