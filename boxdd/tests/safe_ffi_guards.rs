use boxdd::{Error, Foundation, FoundationConfig, FoundationInitError, Polygon, Transform, shapes};

#[cfg(feature = "double-precision")]
const BELOW_SAFE_RAY_SCALE: f32 = 0.5e-9;
#[cfg(not(feature = "double-precision"))]
const BELOW_SAFE_RAY_SCALE: f32 = 0.5e-5;

fn initialize_foundation() {
    Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn foundation_rejects_scales_that_break_safe_native_calculations() {
    for scale in [
        f32::from_bits(1),
        BELOW_SAFE_RAY_SCALE,
        1.0e13_f32,
        1.0e22_f32,
        f32::MAX,
    ] {
        assert!(matches!(
            Foundation::initialize(FoundationConfig::new(scale)),
            Err(Error::FoundationInitialization(
                FoundationInitError::InvalidLengthUnitsPerMeter
            ))
        ));
    }
}

#[test]
fn polygon_mass_data_rejects_density_that_overflows_native_mass_properties() {
    initialize_foundation();
    let polygon = Polygon::box_polygon(1.0, 1.0).expect("box polygon should be valid");

    assert!(matches!(
        polygon.mass_data(f32::MAX),
        Err(Error::InvalidArgument {
            operation: "Polygon::mass_data",
            argument: "polygon/density",
            ..
        })
    ));
}

#[test]
fn offset_polygon_rejects_a_finite_transform_that_collapses_f32_geometry() {
    initialize_foundation();
    let transform =
        Transform::from_pos_angle([f32::MAX, f32::MAX], 0.0).expect("transform should be finite");

    assert!(matches!(
        Polygon::offset_from_points([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], 0.0, transform),
        Err(Error::InvalidArgument {
            operation: "Polygon::offset_from_points",
            argument: "transform",
            ..
        })
    ));
}

#[test]
fn capsule_rejects_finite_endpoints_whose_f32_separation_overflows() {
    initialize_foundation();

    assert!(matches!(
        shapes::capsule([-f32::MAX, 0.0], [f32::MAX, 0.0], 0.25),
        Err(Error::InvalidArgument {
            operation: "Capsule::new",
            argument: "capsule",
            ..
        })
    ));
}
