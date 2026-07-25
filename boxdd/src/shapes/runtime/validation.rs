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
pub(crate) fn assert_shape_def_valid(def: &ShapeDef) {
    assert_non_negative_finite_shape_scalar("density", def.density());
    assert_surface_material_valid(&def.material());
    let _lease = crate::core::foundation::assert_transient_native_lease();
    assert!(
        def.0.internalValue == unsafe { ffi::b2DefaultShapeDef() }.internalValue,
        "invalid ShapeDef: not initialized from b2DefaultShapeDef"
    );
}

#[inline]
pub(crate) fn check_shape_def_valid(def: &ShapeDef) -> ApiResult<()> {
    check_non_negative_finite_shape_scalar(def.density())?;
    check_surface_material_valid(&def.material())?;
    let _lease = crate::core::foundation::transient_native_lease()?;
    if def.0.internalValue != unsafe { ffi::b2DefaultShapeDef() }.internalValue {
        return Err(ApiError::InvalidArgument);
    }
    Ok(())
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
pub(crate) fn assert_chain_segment_geometry_valid(segment: &ChainSegment) {
    assert_shape_geometry_valid("chain segment", segment.is_valid());
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
pub(crate) fn check_chain_segment_geometry_valid(segment: &ChainSegment) -> ApiResult<()> {
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

#[track_caller]
pub(crate) fn assert_shape_world_point_in_local_range(name: &str, id: ShapeId, value: Position) {
    crate::body::assert_body_world_point_in_local_range(
        name,
        value,
        crate::body::body_position_impl(shape_body_id_impl(id)),
    );
}

#[inline]
pub(crate) fn check_shape_world_point_in_local_range(
    id: ShapeId,
    value: Position,
) -> ApiResult<()> {
    crate::body::check_body_world_point_in_local_range(
        value,
        crate::body::body_position_impl(shape_body_id_impl(id)),
    )?;
    Ok(())
}

#[track_caller]
pub(crate) fn assert_shape_vec2_valid(name: &str, value: Vec2) {
    assert!(value.is_valid(), "{name} must be a valid local vector");
}

#[inline]
pub(crate) fn check_shape_vec2_valid(value: Vec2) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn check_shape_wind_parameters_valid(wind: Vec2, drag: f32, lift: f32) -> ApiResult<()> {
    check_shape_vec2_valid(wind)?;
    check_non_negative_finite_shape_scalar(drag)?;
    if lift.is_finite() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

pub(crate) fn shape_closest_point_checked_impl(id: ShapeId, target: Position) -> Position {
    assert_shape_world_point_in_local_range("target", id, target);
    shape_closest_point_impl(id, target)
}

pub(crate) fn try_shape_closest_point_checked_impl(
    id: ShapeId,
    target: Position,
) -> ApiResult<Position> {
    check_shape_world_point_in_local_range(id, target)?;
    Ok(shape_closest_point_impl(id, target))
}

pub(crate) fn shape_test_point_checked_impl(id: ShapeId, point: Position) -> bool {
    assert_shape_world_point_in_local_range("point", id, point);
    shape_test_point_impl(id, point)
}

pub(crate) fn try_shape_test_point_checked_impl(id: ShapeId, point: Position) -> ApiResult<bool> {
    check_shape_world_point_in_local_range(id, point)?;
    Ok(shape_test_point_impl(id, point))
}

pub(crate) fn shape_ray_cast_checked_impl(
    id: ShapeId,
    origin: Position,
    translation: Vec2,
) -> WorldCastOutput {
    assert_shape_world_point_in_local_range("origin", id, origin);
    shape_ray_cast_impl(id, origin, translation)
}

pub(crate) fn try_shape_ray_cast_checked_impl(
    id: ShapeId,
    origin: Position,
    translation: Vec2,
) -> ApiResult<WorldCastOutput> {
    check_shape_world_point_in_local_range(id, origin)?;
    Ok(shape_ray_cast_impl(id, origin, translation))
}
