use boxdd::{
    ApiError, BodyBuilder, ChainDef, Filter, RecordingCapacity, SurfaceMaterial, Vec2, World,
    WorldDef,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn chain_points() -> [Vec2; 4] {
    [
        Vec2::new(-2.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
    ]
}

fn invalid_chain_material() -> SurfaceMaterial {
    SurfaceMaterial::default().with_friction(f32::NAN)
}

fn invalid_material_chain_def() -> ChainDef {
    ChainDef::builder()
        .points(chain_points())
        .single_material(&invalid_chain_material())
        .build()
}

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

#[test]
fn chain_def_validation_rejects_non_finite_points_and_every_invalid_material_field() {
    assert!(
        ChainDef::builder()
            .points(chain_points())
            .build()
            .validate()
            .is_ok()
    );

    for invalid_point in [
        Vec2::new(f32::NAN, 0.0),
        Vec2::new(f32::INFINITY, 0.0),
        Vec2::new(f32::NEG_INFINITY, 0.0),
        Vec2::new(0.0, f32::NAN),
        Vec2::new(0.0, f32::INFINITY),
        Vec2::new(0.0, f32::NEG_INFINITY),
    ] {
        for invalid_index in 0..chain_points().len() {
            let mut points = chain_points();
            points[invalid_index] = invalid_point;
            let def = ChainDef::builder().points(points).build();
            assert_eq!(def.validate(), Err(ApiError::InvalidChainDef));
        }
    }

    let invalid_materials = [
        SurfaceMaterial::default().with_friction(f32::NAN),
        SurfaceMaterial::default().with_friction(f32::INFINITY),
        SurfaceMaterial::default().with_friction(-1.0),
        SurfaceMaterial::default().with_restitution(f32::NAN),
        SurfaceMaterial::default().with_restitution(f32::INFINITY),
        SurfaceMaterial::default().with_restitution(-1.0),
        SurfaceMaterial::default().with_rolling_resistance(f32::NAN),
        SurfaceMaterial::default().with_rolling_resistance(f32::INFINITY),
        SurfaceMaterial::default().with_rolling_resistance(-1.0),
        SurfaceMaterial::default().with_tangent_speed(f32::NAN),
        SurfaceMaterial::default().with_tangent_speed(f32::INFINITY),
        SurfaceMaterial::default().with_tangent_speed(f32::NEG_INFINITY),
    ];
    for material in invalid_materials {
        let def = ChainDef::builder()
            .points(chain_points())
            .single_material(&material)
            .build();
        assert_eq!(def.validate(), Err(ApiError::InvalidChainDef));
    }

    for invalid_index in 0..chain_points().len() {
        let mut materials = [SurfaceMaterial::default(); 4];
        materials[invalid_index] = invalid_chain_material();
        let def = ChainDef::builder()
            .points(chain_points())
            .materials(&materials)
            .build();
        assert_eq!(def.validate(), Err(ApiError::InvalidChainDef));
    }
}

#[test]
fn every_chain_creation_surface_rejects_invalid_def_before_native_mutation() {
    let def = invalid_material_chain_def();

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let shape_count = world.counters().shape_count;
    assert_eq!(
        world.try_create_chain_for_id(body, &def),
        Err(ApiError::InvalidChainDef)
    );
    assert_eq!(world.counters().shape_count, shape_count);
    assert!(catch_unwind(AssertUnwindSafe(|| world.create_chain_for_id(body, &def))).is_err());
    assert_eq!(world.counters().shape_count, shape_count);

    let mut scoped_body = world.body(body).unwrap();
    assert_eq!(
        scoped_body.try_create_chain(&def).err(),
        Some(ApiError::InvalidChainDef)
    );
    drop(scoped_body);
    assert_eq!(world.counters().shape_count, shape_count);

    let mut owned_body = world.create_body_owned(BodyBuilder::new().build());
    assert_eq!(
        owned_body.try_create_chain(&def).err(),
        Some(ApiError::InvalidChainDef)
    );
    assert_eq!(world.counters().shape_count, shape_count);

    let invalid_body = world.create_body_id(BodyBuilder::new().build());
    world.destroy_body_id(invalid_body);
    assert_eq!(
        world.try_create_chain_for_id(invalid_body, &def),
        Err(ApiError::InvalidChainDef)
    );

    let recording_body = world.create_body_id(BodyBuilder::new().build());
    {
        let mut session = world.start_recording(RecordingCapacity::default());
        let recording_shape_count = session.counters().shape_count;
        assert_eq!(
            session.try_create_chain(recording_body, &def),
            Err(ApiError::InvalidChainDef)
        );
        assert_eq!(session.counters().shape_count, recording_shape_count);
        assert!(
            catch_unwind(AssertUnwindSafe(
                || session.create_chain(recording_body, &def)
            ))
            .is_err()
        );
        assert_eq!(session.counters().shape_count, recording_shape_count);
    }
    assert_eq!(world.counters().shape_count, shape_count);
}

#[test]
fn chain_material_setters_reject_invalid_values_and_indices_without_mutation() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let mut chain =
        world.create_chain_for_owned(body, &ChainDef::builder().points(chain_points()).build());
    let chain_id = chain.id();
    let baseline = chain.surface_material(0);
    let invalid = invalid_chain_material();

    assert_eq!(
        chain.try_set_surface_material(0, &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(chain.surface_material(0), baseline);
    assert!(catch_unwind(AssertUnwindSafe(|| chain.set_surface_material(0, &invalid))).is_err());
    assert_eq!(chain.surface_material(0), baseline);
    assert_eq!(
        chain.try_set_surface_material(-1, &SurfaceMaterial::default()),
        Err(ApiError::IndexOutOfRange)
    );
    assert_eq!(
        chain.try_set_surface_material(1, &SurfaceMaterial::default()),
        Err(ApiError::IndexOutOfRange)
    );
    assert_eq!(chain.surface_material(0), baseline);

    let open_points = [
        Vec2::new(-3.0, 0.0),
        Vec2::new(-2.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(3.0, 0.0),
    ];
    let open_materials = [
        SurfaceMaterial::default().with_friction(0.1),
        SurfaceMaterial::default().with_friction(0.2),
        SurfaceMaterial::default().with_friction(0.3),
        SurfaceMaterial::default().with_friction(0.4),
        SurfaceMaterial::default().with_friction(0.5),
        SurfaceMaterial::default().with_friction(0.6),
        SurfaceMaterial::default().with_friction(0.7),
    ];
    let mut open_chain = world.create_chain_for_owned(
        body,
        &ChainDef::builder()
            .points(open_points)
            .materials(&open_materials)
            .build(),
    );
    let open_baseline = open_chain.surface_material(2);
    assert_eq!(
        open_chain.try_set_surface_material(2, &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(open_chain.surface_material(2), open_baseline);

    let loop_materials = [
        SurfaceMaterial::default().with_friction(0.2),
        SurfaceMaterial::default().with_friction(0.4),
        SurfaceMaterial::default().with_friction(0.6),
        SurfaceMaterial::default().with_friction(0.8),
    ];
    let mut loop_chain = world.create_chain_for_owned(
        body,
        &ChainDef::builder()
            .points([
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
            ])
            .materials(&loop_materials)
            .is_loop(true)
            .build(),
    );
    let loop_baseline = loop_chain.surface_material(2);
    assert_eq!(
        loop_chain.try_set_surface_material(2, &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(loop_chain.surface_material(2), loop_baseline);

    {
        let mut scoped = world.chain(chain_id).unwrap();
        assert_eq!(
            scoped.try_set_surface_material(0, &invalid),
            Err(ApiError::InvalidArgument)
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| scoped.set_surface_material(0, &invalid))).is_err()
        );
        assert_eq!(scoped.surface_material(0), baseline);
    }

    {
        let mut session = world.start_recording(RecordingCapacity::default());
        assert_eq!(
            session.try_chain_set_surface_material(chain_id, 0, &invalid),
            Err(ApiError::InvalidArgument)
        );
        assert_eq!(
            session.try_chain_set_surface_material(chain_id, 1, &SurfaceMaterial::default()),
            Err(ApiError::IndexOutOfRange)
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                session.chain_set_surface_material(chain_id, 0, &invalid)
            }))
            .is_err()
        );
    }
    assert_eq!(chain.surface_material(0), baseline);

    world.destroy_chain_id(chain_id);
    assert_eq!(
        chain.try_set_surface_material(0, &invalid),
        Err(ApiError::InvalidArgument)
    );
}
