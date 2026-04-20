use boxdd::{clip_vector, prelude::*, shapes, solve_planes};

#[test]
fn mover_queries_and_solver_are_safe_and_reusable() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(20.0, 0.5),
    );

    let wall = world.create_body_id(BodyBuilder::new().position([1.0_f32, 1.0]).build());
    let _wall_shape = world.create_polygon_shape_for(
        wall,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(0.25, 1.0),
    );

    let c1 = Vec2::new(0.0, 0.7);
    let c2 = Vec2::new(0.0, 1.5);
    let radius = 0.25;

    let fraction = world.cast_mover(c1, c2, radius, [2.0_f32, 0.0], QueryFilter::default());
    assert!(fraction >= 0.0 && fraction < 1.0);

    let mut plane_results = Vec::with_capacity(8);
    let plane_results_ptr = plane_results.as_ptr();
    world.collide_mover_into(c1, c2, radius, QueryFilter::default(), &mut plane_results);
    assert_eq!(plane_results.as_ptr(), plane_results_ptr);
    assert!(!plane_results.is_empty());
    assert!(plane_results.iter().any(|plane| plane.hit));
    assert!(plane_results.iter().any(|plane| plane.plane.normal.y > 0.5));

    let handle_results = world
        .handle()
        .collide_mover(c1, c2, radius, QueryFilter::default());
    assert_eq!(handle_results.len(), plane_results.len());

    let mut collision_planes: Vec<CollisionPlane> = plane_results
        .iter()
        .copied()
        .filter_map(MoverPlaneResult::into_rigid_collision_plane)
        .collect();
    assert!(!collision_planes.is_empty());

    let solved = solve_planes([0.0_f32, -0.2], &mut collision_planes);
    assert!(solved.iteration_count >= 0);
    assert!(solved.translation.y >= -1.0e-4);
    assert!(collision_planes.iter().any(|plane| plane.push > 0.0));

    let clipped = clip_vector([0.0_f32, -1.0], &collision_planes);
    assert!(clipped.y >= -1.0e-4);
}
