use boxdd::{clip_vector, prelude::*, shapes, solve_planes};

fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn mover_queries_and_solver_are_safe_and_reusable() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ground = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .build()
                .unwrap(),
        )
        .unwrap();
    let _ground_shape = world
        .body(ground)
        .unwrap()
        .create_polygon(
            &ShapeDef::builder().density(0.0).build().unwrap(),
            &shapes::box_polygon(20.0, 0.5).unwrap(),
        )
        .unwrap();

    let wall = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .position([1.0_f32, 1.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let _wall_shape = world
        .body(wall)
        .unwrap()
        .create_polygon(
            &ShapeDef::builder().density(0.0).build().unwrap(),
            &shapes::box_polygon(0.25, 1.0).unwrap(),
        )
        .unwrap();

    let c1 = Vec2::new(0.0, 0.7);
    let c2 = Vec2::new(0.0, 1.5);
    let radius = 0.25;

    let query = world.query().unwrap();
    let fraction = query
        .cast_mover(
            Position::ZERO,
            c1,
            c2,
            radius,
            [2.0_f32, 0.0],
            QueryFilter::default(),
        )
        .unwrap();
    assert!((0.0..1.0).contains(&fraction));

    let mut plane_results = MoverQueryBuffer::with_capacity(8).unwrap();
    let plane_results_capacity = plane_results.capacity();
    query
        .collide_mover_into(
            Position::ZERO,
            c1,
            c2,
            radius,
            QueryFilter::default(),
            &mut plane_results,
        )
        .unwrap();
    assert_eq!(plane_results.capacity(), plane_results_capacity);
    assert!(!plane_results.is_empty());
    assert!(plane_results.iter().any(|plane| plane.hit));
    assert!(
        plane_results
            .iter()
            .any(|plane| plane.plane.normal().y > 0.5)
    );

    let mut collision_planes: Vec<CollisionPlane> = plane_results
        .iter()
        .copied()
        .filter_map(|result| result.into_rigid_collision_plane().unwrap())
        .collect();
    assert!(!collision_planes.is_empty());

    let solved = solve_planes([0.0_f32, -0.2], &mut collision_planes).unwrap();
    assert!(solved.iteration_count() >= 0);
    assert!(solved.translation().y >= -1.0e-4);
    assert!(collision_planes.iter().any(|plane| plane.push() > 0.0));

    let clipped = clip_vector([0.0_f32, -1.0], &collision_planes).unwrap();
    assert!(clipped.y >= -1.0e-4);
}

#[test]
fn mover_value_types_use_explicit_raw_conversions() {
    let plane = Plane::new([0.0_f32, 1.0], 2.5).unwrap();
    assert_eq!(Plane::from_raw(plane.into_raw()).unwrap(), plane);

    let collision_plane = CollisionPlane::new(plane, 3.0, true).unwrap();
    assert_eq!(
        CollisionPlane::from_raw(collision_plane.into_raw()).unwrap(),
        collision_plane
    );

    let result = PlaneSolverResult::from_raw(boxdd_sys::ffi::b2PlaneSolverResult {
        translation: boxdd_sys::ffi::b2Vec2 { x: 0.5, y: -0.25 },
        iterationCount: 4,
    })
    .unwrap();
    assert_eq!(result.translation(), Vec2::new(0.5, -0.25));
    assert_eq!(result.iteration_count(), 4);

    assert!(matches!(
        PlaneSolverResult::from_raw(boxdd_sys::ffi::b2PlaneSolverResult {
            translation: boxdd_sys::ffi::b2Vec2 {
                x: f32::NAN,
                y: 0.0,
            },
            iterationCount: 0,
        }),
        Err(Error::InvalidArgument {
            operation: "PlaneSolverResult::from_raw",
            argument: "translation",
            ..
        })
    ));
    assert!(matches!(
        PlaneSolverResult::from_raw(boxdd_sys::ffi::b2PlaneSolverResult {
            translation: boxdd_sys::ffi::b2Vec2 { x: 0.0, y: 0.0 },
            iterationCount: -1,
        }),
        Err(Error::InvalidArgument {
            operation: "PlaneSolverResult::from_raw",
            argument: "iteration_count",
            ..
        })
    ));
}

#[test]
fn mover_solver_validation_errors_are_recoverable() {
    initialize_foundation();

    let plane = Plane::new([0.0_f32, 1.0], 0.0).unwrap();
    let mut planes = [CollisionPlane::rigid(plane).unwrap()];

    assert!(planes[0].validate().is_ok());

    let solved = solve_planes([0.0_f32, -0.2], &mut planes).unwrap();
    assert!(solved.translation().is_valid());
    assert!(planes[0].push() >= 0.0);

    let clipped = clip_vector([0.0_f32, -1.0], &planes).unwrap();
    assert!(clipped.is_valid());

    assert_eq!(
        Plane::new([0.0_f32, 2.0], 0.0).unwrap_err(),
        Error::invalid_argument(
            "Plane::new",
            "normal/offset",
            "a finite plane with a unit normal",
        )
    );

    assert_eq!(
        CollisionPlane::new(plane, -1.0, true).unwrap_err(),
        Error::invalid_argument(
            "CollisionPlane::new",
            "planes[].push_limit",
            "a finite value greater than or equal to zero",
        )
    );

    let mut invalid_raw = CollisionPlane::rigid(plane).unwrap().into_raw();
    invalid_raw.push = f32::NAN;
    assert_eq!(
        CollisionPlane::from_raw(invalid_raw).unwrap_err(),
        Error::invalid_argument(
            "CollisionPlane::from_raw",
            "planes[].push",
            "a finite value greater than or equal to zero",
        )
    );
    assert_eq!(
        solve_planes([f32::NAN, 0.0], &mut planes).unwrap_err(),
        Error::invalid_argument("solve_planes", "target_delta", "a finite vector")
    );
}
