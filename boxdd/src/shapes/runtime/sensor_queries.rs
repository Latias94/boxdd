use super::*;

fn retain_valid_shape_ids(ids: &mut Vec<ShapeId>) {
    ids.retain(|sid| unsafe { ffi::b2Shape_IsValid(raw_shape_id(*sid)) });
}

unsafe fn try_fill_sensor_output(
    out: &mut Vec<ShapeId>,
    brand: crate::id::IdBrand,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> ApiResult<()> {
    unsafe {
        crate::core::ffi_vec::try_fill_mapped_from_ffi(out, requested, fill, |raw| {
            brand.try_shape(raw)
        })
    }
}

unsafe fn try_read_sensor_output(
    brand: crate::id::IdBrand,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> ApiResult<Vec<ShapeId>> {
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(requested, fill, |raw| brand.try_shape(raw))
    }
}

pub(crate) fn shape_sensor_overlaps_into_in_impl(
    brand: crate::id::IdBrand,
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) };
    unsafe {
        try_fill_sensor_output(out, brand, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr, cap)
        })
    }
}

pub(crate) fn shape_sensor_overlaps_in_impl(
    brand: crate::id::IdBrand,
    id: ShapeId,
) -> ApiResult<Vec<ShapeId>> {
    let id = raw_shape_id(id);
    let cap = unsafe { ffi::b2Shape_GetSensorCapacity(id) };
    unsafe {
        try_read_sensor_output(brand, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr, cap)
        })
    }
}

pub(crate) fn shape_sensor_overlaps_valid_into_in_impl(
    brand: crate::id::IdBrand,
    id: ShapeId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    shape_sensor_overlaps_into_in_impl(brand, id, out)?;
    retain_valid_shape_ids(out);
    Ok(())
}

pub(crate) fn shape_sensor_overlaps_valid_in_impl(
    brand: crate::id::IdBrand,
    id: ShapeId,
) -> ApiResult<Vec<ShapeId>> {
    let mut ids = shape_sensor_overlaps_in_impl(brand, id)?;
    retain_valid_shape_ids(&mut ids);
    Ok(ids)
}

#[inline]
pub(crate) fn shape_sensor_capacity_impl(id: ShapeId) -> i32 {
    unsafe { ffi::b2Shape_GetSensorCapacity(raw_shape_id(id)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_brand() -> crate::id::IdBrand {
        crate::id::IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 7,
            },
            crate::id::WorldToken::allocate().unwrap(),
        )
        .unwrap()
    }

    fn raw_shape(index1: i32, world0: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1,
            world0,
            generation: 1,
        }
    }

    #[test]
    fn sensor_output_rejects_invalid_ids_and_reuses_safe_output_allocation() {
        let brand = test_brand();
        let mut out = Vec::<ShapeId>::with_capacity(4);
        out.push(brand.shape(raw_shape(9, brand.world0())));
        let expected_ptr = out.as_ptr();
        let expected_capacity = out.capacity();

        let error = unsafe {
            try_fill_sensor_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(1, brand.world0().wrapping_add(1)));
                1
            })
        }
        .unwrap_err();
        assert_eq!(error, ApiError::WrongWorld);
        assert!(out.is_empty());
        assert_eq!(out.as_ptr(), expected_ptr);
        assert_eq!(out.capacity(), expected_capacity);

        let error = unsafe {
            try_fill_sensor_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(0, brand.world0()));
                1
            })
        }
        .unwrap_err();
        assert_eq!(error, ApiError::InvalidShapeId);
        assert!(out.is_empty());
        assert_eq!(out.as_ptr(), expected_ptr);

        unsafe {
            try_fill_sensor_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(2, brand.world0()));
                1
            })
            .unwrap();
        }
        assert_eq!(out.as_ptr(), expected_ptr);
        assert_eq!(out.capacity(), expected_capacity);
        let raw = out[0].into_raw();
        assert_eq!(raw.index1, 2);
        assert_eq!(raw.world0, brand.world0());
        assert_eq!(raw.generation, 1);
    }
}
