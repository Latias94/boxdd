use super::*;

impl World {
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_capacity_impl(shape)
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        check_shape_target(&self.core, shape)?;
        Ok(crate::shapes::shape_sensor_capacity_impl(shape))
    }

    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_in_impl(self.core.brand(), shape)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn shape_sensor_overlaps_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_into_in_impl(self.core.brand(), shape, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_in_impl(self.core.brand(), shape)
    }

    pub fn try_shape_sensor_overlaps_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_into_in_impl(self.core.brand(), shape, out)
    }

    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_valid_in_impl(self.core.brand(), shape)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    pub fn try_shape_sensor_overlaps_valid(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_valid_in_impl(self.core.brand(), shape)
    }

    pub fn shape_sensor_overlaps_valid_into(&self, shape: ShapeId, out: &mut Vec<ShapeId>) {
        assert_shape_target(&self.core, shape);
        crate::shapes::shape_sensor_overlaps_valid_into_in_impl(self.core.brand(), shape, out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    pub fn try_shape_sensor_overlaps_valid_into(
        &self,
        shape: ShapeId,
        out: &mut Vec<ShapeId>,
    ) -> crate::error::ApiResult<()> {
        check_shape_target(&self.core, shape)?;
        crate::shapes::shape_sensor_overlaps_valid_into_in_impl(self.core.brand(), shape, out)
    }
}
