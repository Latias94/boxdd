use super::*;

#[inline]
pub(crate) fn check_native_shape_non_negative_scalar(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> Result<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite non-negative value",
        })
    }
}

#[inline]
pub(crate) fn check_native_shape_count(
    operation: &'static str,
    output: &'static str,
    value: i32,
) -> Result<i32> {
    if value >= 0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a non-negative native int",
        })
    }
}

#[inline]
pub(crate) fn check_native_shape_sensor_capacity(
    operation: &'static str,
    capacity: i32,
    is_sensor: bool,
) -> Result<i32> {
    if capacity >= 0 && (is_sensor || capacity == 0) {
        Ok(capacity)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "sensor_capacity",
            constraint: "a non-negative count that is zero for a non-sensor shape",
        })
    }
}

pub(crate) fn check_non_negative_finite_shape_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value greater than or equal to zero",
        ))
    }
}

#[inline]
pub(crate) fn check_surface_material_valid(
    operation: &'static str,
    material: &SurfaceMaterial,
) -> Result<()> {
    check_non_negative_finite_shape_scalar(operation, "friction", material.friction())?;
    check_non_negative_finite_shape_scalar(operation, "restitution", material.restitution())?;
    check_non_negative_finite_shape_scalar(
        operation,
        "rolling_resistance",
        material.rolling_resistance(),
    )?;
    if !material.custom_color_is_valid() {
        return Err(Error::invalid_argument(
            operation,
            "custom_color",
            "an RGB value in the inclusive range 0x000000..=0xFFFFFF",
        ));
    }
    if material.tangent_speed().is_finite() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            "tangent_speed",
            "a finite value",
        ))
    }
}

#[inline]
pub(crate) fn check_shape_def_valid(def: &ShapeDef) -> Result<()> {
    check_non_negative_finite_shape_scalar("ShapeDef::validate", "density", def.density())?;
    check_surface_material_valid("ShapeDef::validate", &def.material())?;
    Ok(())
}

#[inline]
pub(crate) fn check_circle_geometry_valid(circle: &Circle) -> Result<()> {
    circle.validate()
}

#[inline]
pub(crate) fn check_segment_geometry_valid(segment: &Segment) -> Result<()> {
    segment.validate()
}

#[inline]
pub(crate) fn check_chain_segment_geometry_valid(segment: &ChainSegment) -> Result<()> {
    segment.validate()
}

#[inline]
pub(crate) fn check_capsule_geometry_valid(capsule: &Capsule) -> Result<()> {
    capsule.validate()
}

#[inline]
pub(crate) fn check_polygon_geometry_valid(polygon: &Polygon) -> Result<()> {
    polygon.validate()
}

#[inline]
pub(crate) fn check_shape_world_point_in_local_range(
    operation: &'static str,
    argument: &'static str,
    shape: crate::world::ShapeCall<'_>,
    value: Position,
) -> Result<()> {
    let raw_body = unsafe { ffi::b2Shape_GetBody(shape.id().into_raw()) };
    let body = shape.with_output_identity_resolver(|resolver| resolver.active_body(raw_body))?;
    let body_position = crate::body::check_valid_native_body_position(
        operation,
        "body_position",
        crate::body::body_position_impl(body),
    )?;
    crate::body::check_body_world_point_in_local_range(operation, argument, value, body_position)?;
    Ok(())
}

#[inline]
pub(crate) fn check_shape_vec2_valid(
    operation: &'static str,
    argument: &'static str,
    value: Vec2,
) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

#[inline]
pub(crate) fn check_shape_wind_parameters_valid(wind: Vec2, drag: f32, lift: f32) -> Result<()> {
    check_shape_vec2_valid("Shape::apply_wind", "wind", wind)?;
    check_non_negative_finite_shape_scalar("Shape::apply_wind", "drag", drag)?;
    if lift.is_finite() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            "Shape::apply_wind",
            "lift",
            "a finite value",
        ))
    }
}

#[cfg(test)]
mod native_output_tests {
    use super::*;

    #[test]
    fn native_shape_scalar_and_count_checks_fail_closed() {
        assert_eq!(
            check_native_shape_non_negative_scalar("Shape::density", "density", f32::NAN),
            Err(Error::InvalidNativeOutput {
                operation: "Shape::density",
                output: "density",
                constraint: "a finite non-negative value",
            })
        );
        assert_eq!(
            check_native_shape_count("Shape::contact_data", "contact_capacity", -1),
            Err(Error::InvalidNativeOutput {
                operation: "Shape::contact_data",
                output: "contact_capacity",
                constraint: "a non-negative native int",
            })
        );
        assert_eq!(
            check_native_shape_sensor_capacity("Shape::sensor_capacity", 1, false),
            Err(Error::InvalidNativeOutput {
                operation: "Shape::sensor_capacity",
                output: "sensor_capacity",
                constraint: "a non-negative count that is zero for a non-sensor shape",
            })
        );

        assert_eq!(
            check_native_shape_non_negative_scalar("Shape::restitution", "restitution", 2.0),
            Ok(2.0)
        );
        assert_eq!(
            check_native_shape_sensor_capacity("Shape::sensor_capacity", 0, false),
            Ok(0)
        );
        assert_eq!(
            check_native_shape_sensor_capacity("Shape::sensor_capacity", 3, true),
            Ok(3)
        );
    }
}
