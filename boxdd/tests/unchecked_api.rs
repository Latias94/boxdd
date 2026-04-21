#![cfg(feature = "unchecked")]

use boxdd::prelude::*;

#[test]
fn unchecked_hotpath_methods_compile_and_work() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    unsafe {
        let _p0 = body.position_unchecked();
        body.set_linear_velocity_unchecked(Vec2::new(1.0, 0.0));
        world.set_body_angular_velocity_unchecked(body.id(), 2.0);
        let _t = world.body_transform_unchecked(body.id());
    }
}

#[test]
fn unchecked_chain_material_access_uses_visible_segment_indexing() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let materials = [
        SurfaceMaterial::default().with_friction(0.05),
        SurfaceMaterial::default().with_friction(0.10),
        SurfaceMaterial::default().with_friction(0.20),
        SurfaceMaterial::default().with_friction(0.30),
        SurfaceMaterial::default().with_friction(0.40),
        SurfaceMaterial::default().with_friction(0.50),
        SurfaceMaterial::default().with_friction(0.60),
    ];
    let mut chain = world.create_chain_for_owned(
        body_id,
        &ChainDef::builder()
            .points([
                [-3.0_f32, 0.0],
                [-2.0, 0.0],
                [-1.0, 0.0],
                [0.0, 0.0],
                [1.0, 0.0],
                [2.0, 0.0],
                [3.0, 0.0],
            ])
            .materials(&materials)
            .build(),
    );

    unsafe {
        assert_eq!(chain.segment_count_unchecked(), 4);
        assert_eq!(chain.surface_material_unchecked(0), materials[1]);
        assert_eq!(chain.surface_material_unchecked(3), materials[4]);

        let updated = SurfaceMaterial::default().with_friction(0.9);
        chain.set_surface_material_unchecked(1, &updated);
        assert_eq!(chain.surface_material_unchecked(1), updated);
    }
}
