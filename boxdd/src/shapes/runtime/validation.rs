use super::*;

#[track_caller]
pub(crate) fn assert_non_negative_finite_shape_scalar(name: &str, value: f32) {
    assert!(
        value.is_finite() && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

pub(crate) fn check_non_negative_finite_shape_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn assert_surface_material_valid(material: &SurfaceMaterial) {
    assert_non_negative_finite_shape_scalar("friction", material.friction());
    assert_non_negative_finite_shape_scalar("restitution", material.restitution());
    assert_non_negative_finite_shape_scalar("rolling_resistance", material.rolling_resistance());
    assert!(
        material.tangent_speed().is_finite(),
        "tangent_speed must be finite, got {}",
        material.tangent_speed()
    );
}

#[inline]
pub(crate) fn check_surface_material_valid(material: &SurfaceMaterial) -> ApiResult<()> {
    check_non_negative_finite_shape_scalar(material.friction())?;
    check_non_negative_finite_shape_scalar(material.restitution())?;
    check_non_negative_finite_shape_scalar(material.rolling_resistance())?;
    if material.tangent_speed().is_finite() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn shape_def_cookie_is_valid(def: &ShapeDef) -> bool {
    def.0.internalValue == unsafe { ffi::b2DefaultShapeDef() }.internalValue
}

#[inline]
pub(crate) fn assert_shape_def_valid(def: &ShapeDef) {
    assert!(
        shape_def_cookie_is_valid(def),
        "invalid ShapeDef: not initialized from b2DefaultShapeDef"
    );
    assert_non_negative_finite_shape_scalar("density", def.density());
    assert_surface_material_valid(&def.material());
}

#[inline]
pub(crate) fn check_shape_def_valid(def: &ShapeDef) -> ApiResult<()> {
    if !shape_def_cookie_is_valid(def) {
        return Err(ApiError::InvalidArgument);
    }
    check_non_negative_finite_shape_scalar(def.density())?;
    check_surface_material_valid(&def.material())
}

#[track_caller]
pub(crate) fn assert_shape_geometry_valid(name: &str, valid: bool) {
    assert!(valid, "{name} must contain valid Box2D geometry");
}

#[inline]
pub(crate) fn assert_circle_geometry_valid(circle: &Circle) {
    assert_shape_geometry_valid("circle", circle.is_valid());
}

#[inline]
pub(crate) fn assert_segment_geometry_valid(segment: &Segment) {
    assert_shape_geometry_valid("segment", segment.is_valid());
}

#[inline]
pub(crate) fn assert_capsule_geometry_valid(capsule: &Capsule) {
    assert_shape_geometry_valid("capsule", capsule.is_valid());
}

#[inline]
pub(crate) fn assert_polygon_geometry_valid(polygon: &Polygon) {
    assert_shape_geometry_valid("polygon", polygon.is_valid());
}

#[inline]
pub(crate) fn check_circle_geometry_valid(circle: &Circle) -> ApiResult<()> {
    circle.validate()
}

#[inline]
pub(crate) fn check_segment_geometry_valid(segment: &Segment) -> ApiResult<()> {
    segment.validate()
}

#[inline]
pub(crate) fn check_capsule_geometry_valid(capsule: &Capsule) -> ApiResult<()> {
    capsule.validate()
}

#[inline]
pub(crate) fn check_polygon_geometry_valid(polygon: &Polygon) -> ApiResult<()> {
    polygon.validate()
}

pub(crate) fn shape_set_density_checked_impl(id: ShapeId, density: f32, update_body_mass: bool) {
    crate::core::debug_checks::assert_shape_valid(id);
    assert_non_negative_finite_shape_scalar("density", density);
    shape_set_density_impl(id, density, update_body_mass);
}

pub(crate) fn try_shape_set_density_checked_impl(
    id: ShapeId,
    density: f32,
    update_body_mass: bool,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    check_non_negative_finite_shape_scalar(density)?;
    shape_set_density_impl(id, density, update_body_mass);
    Ok(())
}

pub(crate) fn shape_set_friction_checked_impl(id: ShapeId, friction: f32) {
    crate::core::debug_checks::assert_shape_valid(id);
    assert_non_negative_finite_shape_scalar("friction", friction);
    shape_set_friction_impl(id, friction);
}

pub(crate) fn try_shape_set_friction_checked_impl(id: ShapeId, friction: f32) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    check_non_negative_finite_shape_scalar(friction)?;
    shape_set_friction_impl(id, friction);
    Ok(())
}

pub(crate) fn shape_set_restitution_checked_impl(id: ShapeId, restitution: f32) {
    crate::core::debug_checks::assert_shape_valid(id);
    assert_non_negative_finite_shape_scalar("restitution", restitution);
    shape_set_restitution_impl(id, restitution);
}

pub(crate) fn try_shape_set_restitution_checked_impl(
    id: ShapeId,
    restitution: f32,
) -> ApiResult<()> {
    crate::core::debug_checks::check_shape_valid(id)?;
    check_non_negative_finite_shape_scalar(restitution)?;
    shape_set_restitution_impl(id, restitution);
    Ok(())
}
