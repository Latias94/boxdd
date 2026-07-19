use boxdd::prelude::*;

#[test]
fn try_step_invalid_values_return_err() {
    let mut world = World::new(WorldDef::default()).unwrap();

    assert_eq!(
        world.try_step(f32::NAN, 4).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world.try_step(1.0 / 60.0, 0).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn try_query_invalid_values_return_err() {
    let world = World::new(WorldDef::default()).unwrap();
    let triangle = [[0.0_f32, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let mut hits = Vec::new();
    let mut planes = Vec::new();

    assert_eq!(
        world
            .try_overlap_aabb(
                Position::ZERO,
                Aabb::new([1.0_f32, 1.0], [-1.0, -1.0]),
                QueryFilter::default()
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_cast_ray_closest(
                Position::new(WorldScalar::NAN, 0.0),
                [1.0, 0.0],
                QueryFilter::default(),
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_overlap_polygon_points(
                Position::ZERO,
                [[0.0_f32, 0.0], [f32::NAN, 1.0], [1.0, 0.0]],
                0.0,
                QueryFilter::default(),
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_cast_shape_points(
                Position::ZERO,
                triangle,
                -1.0,
                [0.0_f32, 1.0],
                QueryFilter::default(),
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_overlap_polygon_points_with_offset(
                Position::ZERO,
                triangle,
                0.0,
                [0.0_f32, 0.0],
                f32::NAN,
                QueryFilter::default(),
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_cast_shape_points_with_offset_into(
                Position::ZERO,
                triangle,
                0.0,
                [0.0_f32, 0.0],
                0.0_f32,
                [f32::NAN, 0.0],
                QueryFilter::default(),
                &mut hits,
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_cast_mover(
                Position::ZERO,
                [0.0_f32, 0.0],
                [0.0, 1.0],
                0.0,
                [1.0_f32, 0.0],
                QueryFilter::default(),
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_collide_mover_into(
                Position::ZERO,
                [f32::NAN, 0.0],
                [0.0, 1.0],
                0.25,
                QueryFilter::default(),
                &mut planes,
            )
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}
