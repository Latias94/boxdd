use boxdd::prelude::*;

#[test]
fn step_invalid_values_return_err() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    assert_eq!(
        world.step(f32::NAN, 4).unwrap_err(),
        Error::invalid_argument(
            "World::step",
            "time_step",
            "a finite value greater than or equal to zero",
        )
    );
    assert_eq!(
        world.step(1.0 / 60.0, 0).unwrap_err(),
        Error::invalid_argument("World::step", "sub_steps", "an integer greater than zero",)
    );
}

#[test]
fn query_invalid_values_return_err_and_clear_reusable_output() {
    let world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let query = world.query().unwrap();
    let triangle = [[0.0_f32, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let proxy = ShapeProxy::new(triangle, 0.0).unwrap();
    let mut hits = RayQueryBuffer::new();
    let mut planes = MoverQueryBuffer::new();

    assert_eq!(
        Aabb::new([1.0_f32, 1.0], [-1.0, -1.0]).unwrap_err(),
        Error::invalid_argument("Aabb::new", "aabb", "finite ordered lower and upper bounds",)
    );
    assert_eq!(
        query
            .cast_ray_closest(
                Position::new(WorldScalar::NAN, 0.0),
                [1.0, 0.0],
                QueryFilter::default(),
            )
            .unwrap_err(),
        Error::invalid_argument(
            "Query::cast_ray_closest",
            "origin",
            "a finite world position",
        )
    );
    assert_eq!(
        ShapeProxy::new([[0.0_f32, 0.0], [f32::NAN, 1.0], [1.0, 0.0]], 0.0,).unwrap_err(),
        Error::invalid_argument("ShapeProxy::new", "points", "a finite vector",)
    );
    assert_eq!(
        ShapeProxy::new(triangle, -1.0).unwrap_err(),
        Error::invalid_argument(
            "ShapeProxy::new",
            "radius",
            "a finite value greater than or equal to zero",
        )
    );
    assert_eq!(
        Transform::from_pos_angle([0.0_f32, 0.0], f32::NAN).unwrap_err(),
        Error::invalid_argument("Rot::from_radians", "rad", "a finite angle",)
    );
    assert_eq!(
        query
            .cast_shape_into(
                Position::ZERO,
                proxy,
                [f32::NAN, 0.0],
                QueryFilter::default(),
                &mut hits,
            )
            .unwrap_err(),
        Error::invalid_argument("Query::cast_shape", "translation", "a finite vector",)
    );
    assert_eq!(
        query
            .cast_mover(
                Position::ZERO,
                [0.0_f32, 0.0],
                [0.0, 1.0],
                0.0,
                [1.0_f32, 0.0],
                QueryFilter::default(),
            )
            .unwrap_err(),
        Error::invalid_argument(
            "Query::cast_mover",
            "radius",
            "a finite value greater than the configured minimum mover radius",
        )
    );
    assert_eq!(
        query
            .collide_mover_into(
                Position::ZERO,
                [f32::NAN, 0.0],
                [0.0, 1.0],
                0.25,
                QueryFilter::default(),
                &mut planes,
            )
            .unwrap_err(),
        Error::invalid_argument("Query::collide_mover", "c1", "a finite vector")
    );
    assert!(hits.is_empty());
    assert!(planes.is_empty());
}
