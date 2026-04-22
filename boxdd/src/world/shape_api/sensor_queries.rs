use super::*;

impl World {
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_capacity_impl(shape)
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_capacity_impl(shape))
    }

    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_impl(shape)
    }

    pub fn shape_sensor_overlaps_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_impl(shape))
    }

    pub fn try_shape_sensor_overlaps_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_into_impl(shape, out);
        Ok(())
    }

    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_impl(shape)
    }

    pub fn try_shape_sensor_overlaps_valid(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(crate::shapes::shape_sensor_overlaps_valid_impl(shape))
    }

    pub fn shape_sensor_overlaps_valid_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        crate::core::debug_checks::assert_shape_valid(shape);
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
    }

    pub fn try_shape_sensor_overlaps_valid_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        crate::shapes::shape_sensor_overlaps_valid_into_impl(shape, out);
        Ok(())
    }
}
