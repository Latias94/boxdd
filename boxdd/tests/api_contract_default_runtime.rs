#[test]
fn default_definition_constructors_are_runtime_witnesses() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let body_def = boxdd::Foundation::body_def(foundation);
    let world_def = boxdd::Foundation::world_def(foundation);
    let chain_builder = boxdd::ChainDefBuilder::points(
        boxdd::ChainDef::builder(),
        [
            [-2.0_f32, 0.0],
            [-1.0_f32, 0.0],
            [1.0_f32, 0.0],
            [2.0_f32, 0.0],
        ],
    );
    let explosion_def = boxdd::ExplosionDef::new();
    let filter = boxdd::Filter::new();
    let query_filter = boxdd::QueryFilter::new();
    let shape_builder = boxdd::ShapeDef::builder();
    let surface_material = boxdd::SurfaceMaterial::new();

    let chain_def = boxdd::ChainDefBuilder::build(chain_builder).unwrap();
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder).unwrap();
    let body_type = boxdd::BodyDef::body_type(&body_def);
    let world_gravity = boxdd::WorldDef::gravity(&world_def);
    let chain_points_are_empty = boxdd::ChainDef::points(&chain_def).is_empty();
    let blast_radius = boxdd::ExplosionDef::blast_radius(&explosion_def);
    let query_category_bits = boxdd::QueryFilter::category_bits(&query_filter);
    let query_mask_bits = boxdd::QueryFilter::mask_bits(&query_filter);
    let shape_density = boxdd::ShapeDef::density(&shape_def);
    let material_friction = boxdd::SurfaceMaterial::friction(&surface_material);

    assert_eq!(body_type, boxdd::BodyType::Static);
    assert!(world_gravity.is_valid());
    assert!(!chain_points_are_empty);
    assert!(blast_radius.is_finite());
    assert_eq!(filter.category_bits, 1);
    assert_eq!(filter.mask_bits, u64::MAX);
    assert_eq!(filter.group_index, 0);
    assert_eq!(query_category_bits, 1);
    assert_eq!(query_mask_bits, u64::MAX);
    assert!(shape_density.is_finite());
    assert!(material_friction.is_finite());
}
