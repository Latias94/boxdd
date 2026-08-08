use boxdd::prelude::*;

fn chain_points() -> [Vec2; 4] {
    [
        Vec2::new(-2.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
    ]
}

fn invalid_chain_points() -> [Vec2; 4] {
    let mut points = chain_points();
    points[0] = Vec2::new(f32::NAN, 0.0);
    points
}

#[test]
fn chain_def_owns_points_and_materials_across_clone_and_creation() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let material = SurfaceMaterial::default().with_friction(0.3).unwrap();
    let original = ChainDef::builder()
        .points(chain_points())
        .single_material(&material)
        .filter(Filter {
            category_bits: 0x0002,
            mask_bits: 0x0004,
            group_index: -1,
        })
        .build()
        .unwrap();
    let cloned = original.clone();
    assert_eq!(original.points(), cloned.points());
    assert_eq!(original.material_count(), 1);
    assert!(matches!(
        cloned.material_layout(),
        ChainDefMaterialLayout::Single(value) if value == material
    ));

    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let body = world.create_body(foundation.body_def()).unwrap();
    let chain = world.body(body).unwrap().create_chain(&cloned).unwrap();
    assert!(world.chain(chain).unwrap().segment_count().unwrap() > 0);

    let defaulted = ChainDef::builder()
        .points(chain_points())
        .materials(&[])
        .build()
        .unwrap();
    assert!(matches!(
        defaulted.material_layout(),
        ChainDefMaterialLayout::Default(_)
    ));
    world.body(body).unwrap().create_chain(&defaulted).unwrap();
}

#[test]
fn chain_def_validation_rejects_non_finite_points() {
    boxdd::Foundation::initialize_default().unwrap();
    assert!(ChainDef::builder().points(chain_points()).build().is_ok());

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
            assert_eq!(
                ChainDef::builder().points(points).build().unwrap_err(),
                Error::InvalidChainDef
            );
        }
    }
}

#[test]
fn surface_material_setters_reject_invalid_values() {
    for (result, operation, field, requirement) in [
        (
            SurfaceMaterial::default().with_friction(f32::NAN),
            "SurfaceMaterial::with_friction",
            "friction",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_friction(f32::INFINITY),
            "SurfaceMaterial::with_friction",
            "friction",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_friction(-1.0),
            "SurfaceMaterial::with_friction",
            "friction",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_restitution(f32::NAN),
            "SurfaceMaterial::with_restitution",
            "restitution",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_restitution(f32::INFINITY),
            "SurfaceMaterial::with_restitution",
            "restitution",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_restitution(-1.0),
            "SurfaceMaterial::with_restitution",
            "restitution",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_rolling_resistance(f32::NAN),
            "SurfaceMaterial::with_rolling_resistance",
            "rolling_resistance",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_rolling_resistance(f32::INFINITY),
            "SurfaceMaterial::with_rolling_resistance",
            "rolling_resistance",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_rolling_resistance(-1.0),
            "SurfaceMaterial::with_rolling_resistance",
            "rolling_resistance",
            "a finite value greater than or equal to zero",
        ),
        (
            SurfaceMaterial::default().with_tangent_speed(f32::NAN),
            "SurfaceMaterial::with_tangent_speed",
            "tangent_speed",
            "a finite value",
        ),
        (
            SurfaceMaterial::default().with_tangent_speed(f32::INFINITY),
            "SurfaceMaterial::with_tangent_speed",
            "tangent_speed",
            "a finite value",
        ),
        (
            SurfaceMaterial::default().with_tangent_speed(f32::NEG_INFINITY),
            "SurfaceMaterial::with_tangent_speed",
            "tangent_speed",
            "a finite value",
        ),
    ] {
        assert_eq!(
            result,
            Err(Error::invalid_argument(operation, field, requirement))
        );
    }
}

#[test]
fn invalid_chain_build_leaves_world_and_recording_native_state_unchanged() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let before = world.counters().unwrap().shape_count;

    assert_eq!(
        ChainDef::builder()
            .points(invalid_chain_points())
            .build()
            .unwrap_err(),
        Error::InvalidChainDef
    );
    assert_eq!(world.counters().unwrap().shape_count, before);

    let session = world.start_recording(RecordingLimits::default()).unwrap();
    let recording_before = session.counters().unwrap().shape_count;
    assert_eq!(
        ChainDef::builder()
            .points(invalid_chain_points())
            .build()
            .unwrap_err(),
        Error::InvalidChainDef
    );
    assert_eq!(session.counters().unwrap().shape_count, recording_before);
    session.finish().unwrap();
    assert_eq!(world.counters().unwrap().shape_count, before);
}

#[test]
fn chain_material_mutation_checks_indices_before_changing_native_state() {
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
    let chain_id = world
        .body(body)
        .unwrap()
        .create_chain(&ChainDef::builder().points(chain_points()).build().unwrap())
        .unwrap();
    let mut chain = world.chain(chain_id).unwrap();
    let baseline = chain.surface_material(0).unwrap();
    assert_eq!(
        chain.set_surface_material(-1, &SurfaceMaterial::default()),
        Err(Error::index_out_of_range(
            "Chain::set_surface_material",
            -1,
            1,
        ))
    );
    assert_eq!(
        chain.set_surface_material(1, &SurfaceMaterial::default()),
        Err(Error::index_out_of_range(
            "Chain::set_surface_material",
            1,
            1,
        ))
    );
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    let mut chain = session.chain(chain_id).unwrap();
    assert_eq!(
        chain.set_surface_material(1, &SurfaceMaterial::default()),
        Err(Error::index_out_of_range(
            "Chain::set_surface_material",
            1,
            1,
        ))
    );
    assert_eq!(chain.surface_material(0).unwrap(), baseline);
    session.finish().unwrap();

    assert_eq!(
        world.chain(chain_id).unwrap().surface_material(0).unwrap(),
        baseline
    );
}
