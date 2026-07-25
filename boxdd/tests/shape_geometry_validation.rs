use boxdd::{
    Aabb, ApiError, BodyBuilder, Polygon, Position, QueryFilter, RecordingCapacity, ShapeCastInput,
    ShapeDef, ShapeProxy, SurfaceMaterial, Vec2, World, WorldDef, shapes,
};
use std::cell::Cell;

struct ConversionProbe<'a> {
    conversions: &'a Cell<usize>,
    value: Vec2,
}

impl From<ConversionProbe<'_>> for Vec2 {
    fn from(probe: ConversionProbe<'_>) -> Self {
        probe.conversions.set(probe.conversions.get() + 1);
        probe.value
    }
}

fn assert_cast_output_eq(actual: boxdd::CastOutput, expected: boxdd::CastOutput) {
    assert_eq!(actual.normal, expected.normal);
    assert_eq!(actual.point, expected.point);
    assert_eq!(actual.fraction, expected.fraction);
    assert_eq!(actual.iterations, expected.iterations);
    assert_eq!(actual.hit, expected.hit);
}

#[test]
fn world_try_shape_set_geometry_rejects_invalid_values() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let def = ShapeDef::default();
    let shape_id =
        world.create_circle_shape_for(body_id, &def, &shapes::circle([0.0_f32, 0.0], 0.5));

    assert_eq!(
        world
            .try_shape_set_segment(shape_id, &shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]))
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut raw_polygon = shapes::box_polygon(0.5, 0.5).into_raw();
    raw_polygon.radius = -1.0;
    assert_eq!(
        world
            // SAFETY: this intentionally violates the radius invariant to verify defensive
            // validation rejects the value before native use.
            .try_shape_set_polygon(shape_id, &unsafe { Polygon::from_raw(raw_polygon) })
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn owned_shape_try_set_geometry_rejects_invalid_values() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let def = ShapeDef::default();
    let mut shape =
        world.create_circle_shape_for_owned(body_id, &def, &shapes::circle([0.0_f32, 0.0], 0.5));

    assert_eq!(
        shape
            .try_set_circle(&shapes::circle([f32::NAN, 0.0], 0.5))
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shape
            .try_set_capsule(&shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25))
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn surface_material_validation_rejects_every_invalid_numeric_field() {
    assert!(SurfaceMaterial::default().validate().is_ok());
    assert!(
        SurfaceMaterial::default()
            .with_tangent_speed(-1.0)
            .validate()
            .is_ok()
    );

    for material in [
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
    ] {
        assert_eq!(material.validate(), Err(ApiError::InvalidArgument));
    }
}

#[test]
fn every_shape_material_setter_rejects_invalid_values_without_mutation() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    let shape = world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let baseline = world.shape_surface_material(shape);
    let invalid = SurfaceMaterial::default().with_rolling_resistance(f32::INFINITY);

    assert_eq!(
        world.try_shape_set_surface_material(shape, &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world.shape_set_surface_material(shape, &invalid)
        }))
        .is_err()
    );
    assert_eq!(world.shape_surface_material(shape), baseline);

    {
        let mut scoped = world.shape(shape).unwrap();
        assert_eq!(
            scoped.try_set_surface_material(&invalid),
            Err(ApiError::InvalidArgument)
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scoped.set_surface_material(&invalid)
            }))
            .is_err()
        );
        assert_eq!(scoped.surface_material(), baseline);
    }

    let mut owned = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::default(),
        &shapes::circle([1.0_f32, 0.0], 0.5),
    );
    let owned_baseline = owned.surface_material();
    assert_eq!(
        owned.try_set_surface_material(&invalid),
        Err(ApiError::InvalidArgument)
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owned.set_surface_material(&invalid)
        }))
        .is_err()
    );
    assert_eq!(owned.surface_material(), owned_baseline);

    {
        let mut session = world.start_recording(RecordingCapacity::default());
        assert_eq!(
            session.try_shape_set_surface_material(shape, &invalid),
            Err(ApiError::InvalidArgument)
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                session.shape_set_surface_material(shape, &invalid)
            }))
            .is_err()
        );
    }
    assert_eq!(world.shape_surface_material(shape), baseline);

    world.destroy_shape_id(shape, true);
    assert_eq!(
        world.try_shape_set_surface_material(shape, &invalid),
        Err(ApiError::InvalidArgument)
    );
}

#[test]
fn safe_shape_creation_panics_on_invalid_geometry() {
    let world_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body_id = world.create_body_id(BodyBuilder::new().build());
        world.create_circle_shape_for(
            body_id,
            &ShapeDef::default(),
            &shapes::circle([f32::NAN, 0.0], 0.5),
        );
    }));
    assert!(world_result.is_err());

    let body_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut world = World::new(WorldDef::default()).unwrap();
        let mut body = world.create_body(BodyBuilder::new().build());
        body.create_segment_shape(
            &ShapeDef::default(),
            &shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]),
        );
    }));
    assert!(body_result.is_err());
}

#[test]
fn standalone_geometry_try_helpers_reject_invalid_inputs() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let invalid_world_transform = boxdd::WorldTransform::from_pos_angle([f32::NAN, 0.0], 0.0);
    let invalid_transform = boxdd::Transform::from_pos_angle([f32::NAN, 0.0], 0.0);

    assert_eq!(
        circle.try_mass_data(-1.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle.try_aabb(invalid_world_transform).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle.try_contains_point([f32::NAN, 0.0]).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle
            .try_ray_cast([0.0_f32, 0.0], [f32::NAN, 0.0])
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    assert_eq!(
        shapes::circle([f32::NAN, 0.0], 0.5)
            .try_mass_data(1.0)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let polygon = shapes::box_polygon(1.0, 1.0);
    assert_eq!(
        polygon.try_transformed(invalid_transform).unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut raw_polygon = polygon.into_raw();
    raw_polygon.radius = -1.0;
    // SAFETY: this intentionally violates the radius invariant to exercise fallible validation.
    let invalid_polygon = unsafe { Polygon::from_raw(raw_polygon) };
    assert_eq!(
        invalid_polygon
            .try_aabb(boxdd::WorldTransform::IDENTITY)
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn polygon_validation_rejects_degenerate_and_clockwise_raw_geometry() {
    let mut degenerate_raw = shapes::box_polygon(1.0, 1.0).into_raw();
    degenerate_raw.vertices[0] = Vec2::new(-1.0, 0.0).into_raw();
    degenerate_raw.vertices[1] = Vec2::new(0.0, 0.0).into_raw();
    degenerate_raw.vertices[2] = Vec2::new(1.0, 0.0).into_raw();
    degenerate_raw.vertices[3] = Vec2::new(2.0, 0.0).into_raw();
    // SAFETY: this intentionally violates the convex-polygon invariant for a validation test.
    let degenerate = unsafe { Polygon::from_raw(degenerate_raw) };
    assert!(!degenerate.is_valid());
    assert_eq!(
        degenerate.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        degenerate.try_mass_data(1.0).unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut clockwise_raw = shapes::box_polygon(1.0, 1.0).into_raw();
    clockwise_raw.vertices[..4].reverse();
    clockwise_raw.normals[..4].reverse();
    // SAFETY: this intentionally violates the counter-clockwise invariant for a validation test.
    let clockwise = unsafe { Polygon::from_raw(clockwise_raw) };
    assert!(!clockwise.is_valid());
    assert_eq!(clockwise.validate().unwrap_err(), ApiError::InvalidArgument);
    assert_eq!(
        clockwise.try_mass_data(1.0).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn native_polygon_helpers_satisfy_complete_semantic_validation() {
    let polygons = [
        shapes::box_polygon(50.0, 10.0),
        shapes::box_polygon(1.25, 0.75),
        shapes::offset_box_polygon(
            1.5,
            0.625,
            boxdd::Transform::from_pos_angle([1_000.25_f32, -750.5], 0.37),
        ),
        shapes::rounded_box_polygon(1.0, 0.5, 0.2),
        shapes::polygon_from_points(
            [
                Vec2::new(-1.25, -0.5),
                Vec2::new(0.75, -1.0),
                Vec2::new(1.5, 0.25),
                Vec2::new(0.25, 1.25),
                Vec2::new(-1.0, 0.75),
            ],
            0.125,
        )
        .expect("valid native polygon hull"),
    ];

    for polygon in polygons {
        assert!(polygon.is_valid());
        polygon.validate().expect("native polygon must validate");
    }

    let mut wrong_centroid = shapes::box_polygon(50.0, 10.0).into_raw();
    wrong_centroid.centroid.x += 0.25;
    // SAFETY: this intentionally violates the centroid invariant for a validation test.
    let wrong_centroid = unsafe { Polygon::from_raw(wrong_centroid) };
    assert!(!wrong_centroid.is_valid());
    assert_eq!(
        wrong_centroid.validate().unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn polygon_hull_validation_has_a_recoverable_native_path() {
    let valid = [
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, -1.0),
    ];
    assert!(shapes::try_polygon_hull_is_valid(valid).unwrap());

    let collinear = [
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
    ];
    assert!(!shapes::try_polygon_hull_is_valid(collinear).unwrap());

    assert_eq!(
        shapes::try_polygon_hull_is_valid([
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
        ])
        .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn definition_validation_checks_pure_fields_before_callback_lease() {
    let invalid_world = WorldDef::builder().gravity([f32::NAN, 0.0]).build();
    let invalid_body = BodyBuilder::new().linear_damping(f32::NAN).build();
    let invalid_shape = ShapeDef::builder().density(f32::NAN).build();
    let valid_world = WorldDef::default();
    let valid_body = BodyBuilder::new().build();
    let valid_shape = ShapeDef::default();

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let mut observed = None;
    let completed = world.visit_overlap_aabb(
        Position::ZERO,
        Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]),
        QueryFilter::default(),
        |_| {
            observed = Some((
                invalid_world.validate(),
                invalid_body.validate(),
                invalid_shape.validate(),
                valid_world.validate(),
                valid_body.validate(),
                valid_shape.validate(),
            ));
            false
        },
    );

    assert!(!completed);
    assert_eq!(
        observed,
        Some((
            Err(ApiError::InvalidArgument),
            Err(ApiError::InvalidArgument),
            Err(ApiError::InvalidArgument),
            Err(ApiError::InCallback),
            Err(ApiError::InCallback),
            Err(ApiError::InCallback),
        ))
    );
}

#[test]
fn standalone_shape_specific_shape_casts_match_try_variants() {
    let proxy = ShapeProxy::new([[0.0_f32, -3.0]], 0.05).expect("valid cast proxy");
    let input = ShapeCastInput::new(proxy, [0.0_f32, 6.0]);
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let segment = shapes::segment([-1.0_f32, 0.0], [1.0_f32, 0.0]);
    let polygon = shapes::box_polygon(0.5, 0.5);

    for output in [
        circle.shape_cast(input),
        circle.try_shape_cast(input).unwrap(),
        capsule.shape_cast(input),
        capsule.try_shape_cast(input).unwrap(),
        segment.shape_cast(input),
        segment.try_shape_cast(input).unwrap(),
        polygon.shape_cast(input),
        polygon.try_shape_cast(input).unwrap(),
    ] {
        assert!(output.hit);
        assert!(output.fraction >= 0.0 && output.fraction <= 1.0);
    }

    assert_cast_output_eq(
        circle.shape_cast(input),
        circle.try_shape_cast(input).unwrap(),
    );
    assert_cast_output_eq(
        capsule.shape_cast(input),
        capsule.try_shape_cast(input).unwrap(),
    );
    assert_cast_output_eq(
        segment.shape_cast(input),
        segment.try_shape_cast(input).unwrap(),
    );
    assert_cast_output_eq(
        polygon.shape_cast(input),
        polygon.try_shape_cast(input).unwrap(),
    );
}

#[test]
fn standalone_shape_specific_shape_casts_reject_invalid_inputs() {
    let proxy = ShapeProxy::new([[0.0_f32, -3.0]], 0.05).expect("valid cast proxy");
    let input = ShapeCastInput::new(proxy, [f32::NAN, 0.0]);
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);

    assert_eq!(
        circle.try_shape_cast(input).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn safe_standalone_geometry_helpers_panic_on_invalid_inputs() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let invalid_world_transform = boxdd::WorldTransform::from_pos_angle([f32::NAN, 0.0], 0.0);
    let invalid_transform = boxdd::Transform::from_pos_angle([f32::NAN, 0.0], 0.0);

    let mass_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.mass_data(-1.0);
    }));
    assert!(mass_result.is_err());

    let aabb_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.aabb(invalid_world_transform);
    }));
    assert!(aabb_result.is_err());

    let point_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.contains_point([f32::NAN, 0.0]);
    }));
    assert!(point_result.is_err());

    let ray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.ray_cast([0.0_f32, 0.0], [f32::NAN, 0.0]);
    }));
    assert!(ray_result.is_err());

    let proxy = ShapeProxy::new([[0.0_f32, -3.0]], 0.05).expect("valid cast proxy");
    let invalid_shape_cast = ShapeCastInput::new(proxy, [f32::NAN, 0.0]);
    let shape_cast_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.shape_cast(invalid_shape_cast);
    }));
    assert!(shape_cast_result.is_err());

    let polygon = shapes::box_polygon(1.0, 1.0);
    let transform_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        polygon.transformed(invalid_transform);
    }));
    assert!(transform_result.is_err());
}

#[test]
fn degenerate_segment_and_capsule_helpers_remain_usable() {
    let segment = shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]);
    assert_eq!(segment.validate().unwrap_err(), ApiError::InvalidArgument);
    assert_eq!(
        segment.try_aabb(boxdd::WorldTransform::IDENTITY).unwrap(),
        segment.aabb(boxdd::WorldTransform::IDENTITY)
    );
    assert_cast_output_eq(
        segment
            .try_ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0], false)
            .unwrap(),
        segment.ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0], false),
    );

    let capsule = shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.5);
    assert_eq!(capsule.validate().unwrap_err(), ApiError::InvalidArgument);
    assert_eq!(capsule.try_mass_data(1.0).unwrap(), capsule.mass_data(1.0));
    assert_eq!(
        capsule.try_aabb(boxdd::WorldTransform::IDENTITY).unwrap(),
        capsule.aabb(boxdd::WorldTransform::IDENTITY)
    );
    assert_eq!(
        capsule.try_contains_point([0.0_f32, 0.0]).unwrap(),
        capsule.contains_point([0.0_f32, 0.0])
    );
    assert_cast_output_eq(
        capsule
            .try_ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0])
            .unwrap(),
        capsule.ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0]),
    );
}

#[test]
fn standalone_geometry_keeps_validation_callback_safe_and_gates_native_calls() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().build());
    world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let segment = shapes::segment([-1.0_f32, 0.0], [1.0_f32, 0.0]);
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let chain_segment = shapes::chain_segment(
        [-2.0_f32, 0.0],
        [-1.0_f32, 0.0],
        [1.0_f32, 0.0],
        [2.0_f32, 0.0],
    );
    let polygon = shapes::box_polygon(0.5, 0.5);
    let conversions = Cell::new(0);
    let mut callback_results = None;
    let completed = world.visit_overlap_aabb(
        Position::ZERO,
        Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]),
        QueryFilter::default(),
        |_| {
            let point_error = circle
                .try_contains_point(ConversionProbe {
                    conversions: &conversions,
                    value: Vec2::ZERO,
                })
                .unwrap_err();
            let polygon_error = shapes::try_polygon_from_points(
                [
                    ConversionProbe {
                        conversions: &conversions,
                        value: Vec2::new(-1.0, 0.0),
                    },
                    ConversionProbe {
                        conversions: &conversions,
                        value: Vec2::new(1.0, 0.0),
                    },
                    ConversionProbe {
                        conversions: &conversions,
                        value: Vec2::new(0.0, 1.0),
                    },
                ],
                0.0,
            )
            .unwrap_err();
            let hull_error = shapes::try_polygon_hull_is_valid([
                ConversionProbe {
                    conversions: &conversions,
                    value: Vec2::new(-1.0, 0.0),
                },
                ConversionProbe {
                    conversions: &conversions,
                    value: Vec2::new(1.0, 0.0),
                },
                ConversionProbe {
                    conversions: &conversions,
                    value: Vec2::new(0.0, 1.0),
                },
            ])
            .unwrap_err();
            let invalid_hull_error = shapes::try_polygon_hull_is_valid([
                Vec2::new(f32::NAN, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
            ])
            .unwrap_err();
            callback_results = Some((
                point_error,
                polygon_error,
                hull_error,
                invalid_hull_error,
                circle.is_valid()
                    && segment.is_valid()
                    && capsule.is_valid()
                    && chain_segment.is_valid()
                    && polygon.is_valid(),
                circle.validate().is_ok()
                    && segment.validate().is_ok()
                    && capsule.validate().is_ok()
                    && chain_segment.validate().is_ok()
                    && polygon.validate().is_ok(),
                shapes::circle([f32::NAN, 0.0], 0.5).validate(),
            ));
            false
        },
    );

    assert!(!completed);
    assert_eq!(conversions.get(), 7);
    assert_eq!(
        callback_results,
        Some((
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InvalidArgument,
            true,
            true,
            Err(ApiError::InvalidArgument),
        ))
    );

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.visit_overlap_aabb(
            Position::ZERO,
            Aabb::new([-1.0_f32, -1.0], [1.0, 1.0]),
            QueryFilter::default(),
            |_| {
                circle.mass_data(1.0);
                true
            },
        );
    }));
    assert!(panic.is_err());
}
