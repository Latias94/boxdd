use super::*;

unsafe fn try_read_sensor_output(
    resolver: &crate::core::identity_registry::OutputIdentityResolver<'_>,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> Result<Vec<ShapeId>> {
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(requested, fill, |raw| resolver.shape(raw))
    }
}

pub(crate) fn shape_sensor_overlaps_in_impl(
    shape: crate::world::ShapeCall<'_>,
) -> Result<Vec<ShapeId>> {
    let id = raw_shape_id(shape.id());
    let cap = shape_sensor_capacity_impl("Shape::sensor_overlaps", shape.id())?;
    shape.with_output_identity_resolver(|resolver| unsafe {
        try_read_sensor_output(resolver, cap, |ptr, cap| {
            ffi::b2Shape_GetSensorData(id, ptr, cap)
        })
    })
}

#[inline]
pub(crate) fn shape_sensor_capacity_impl(operation: &'static str, id: ShapeId) -> Result<i32> {
    check_native_shape_sensor_capacity(
        operation,
        unsafe { ffi::b2Shape_GetSensorCapacity(raw_shape_id(id)) },
        shape_is_sensor_impl(id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> (
        crate::id::IdBrand,
        std::sync::Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) {
        let brand = crate::id::IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 7,
            },
            crate::id::WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let registry = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        let body = registry
            .register_body(ffi::b2BodyId {
                index1: 1,
                world0: brand.world0(),
                generation: 1,
            })
            .unwrap();
        for index1 in [2, 9] {
            registry
                .register_shape(raw_shape(index1, brand.world0()), body)
                .unwrap();
        }
        (brand, registry)
    }

    fn raw_shape(index1: i32, world0: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1,
            world0,
            generation: 1,
        }
    }

    #[test]
    fn sensor_output_rejects_invalid_ids() {
        let (brand, registry) = test_registry();
        let error = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_sensor_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(1, brand.world0().wrapping_add(1)));
                    1
                })
            })
            .unwrap_err();
        assert_eq!(error, Error::WrongWorld);

        let error = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_sensor_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(0, brand.world0()));
                    1
                })
            })
            .unwrap_err();
        assert_eq!(error, Error::InvalidShapeId);

        let out = registry
            .with_output_resolver(|resolver| unsafe {
                try_read_sensor_output(resolver, 1, |ptr, _capacity| {
                    ptr.write(raw_shape(2, brand.world0()));
                    1
                })
            })
            .unwrap();
        let raw = out[0].into_raw();
        assert_eq!(raw.index1, 2);
        assert_eq!(raw.world0, brand.world0());
        assert_eq!(raw.generation, 1);
    }
}
