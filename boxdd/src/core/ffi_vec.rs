use crate::error::{ApiError, ApiResult};
use boxdd_sys::ffi;

pub(crate) const FFI_OUTPUT_EXPECT: &str = "Box2D returned an invalid FFI output contract";

mod sealed {
    pub trait Sealed {}
}

/// A plain C value that Box2D may initialize through an output pointer.
///
/// # Safety
///
/// Implementors must be `Copy` C-layout values with no drop glue that Box2D uses as complete output
/// elements. For every element included in the returned count, Box2D must write a value that is
/// valid for the corresponding Rust FFI type; elements outside that prefix may stay uninitialized.
/// The trait is sealed so safe wrapper types cannot accidentally become direct FFI output buffers.
pub(crate) unsafe trait FfiOutput: sealed::Sealed + Copy {}

macro_rules! impl_ffi_output {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed for $ty {}

            // SAFETY: bindgen declares these official Box2D output element types as `Copy`
            // `repr(C)` records with no drop glue.
            unsafe impl FfiOutput for $ty {}
        )+
    };
}

impl_ffi_output!(ffi::b2ContactData, ffi::b2JointId, ffi::b2ShapeId);

/// Fill `out` from a native output-pointer API and commit only its validated initialized prefix.
///
/// `out` is cleared before any fallible work, so allocation failures, invalid native counts, and
/// unwinding from `fill` all leave its visible length at zero.
///
/// # Safety
///
/// When called, `fill` may initialize at most `requested` consecutive values at the supplied
/// pointer. It must return the exact number of initialized values and must not read from the output
/// memory before initializing it.
pub(crate) unsafe fn fill_from_ffi<T: FfiOutput>(
    out: &mut Vec<T>,
    requested: i32,
    fill: impl FnOnce(*mut T, i32) -> i32,
) -> ApiResult<()> {
    out.clear();

    let requested_usize =
        usize::try_from(requested).map_err(|_| ApiError::NegativeFfiOutputCapacity {
            capacity: requested,
        })?;
    if requested_usize == 0 {
        return Ok(());
    }

    out.try_reserve(requested_usize)
        .map_err(|_| ApiError::FfiOutputAllocationFailed)?;
    let output = out.spare_capacity_mut();
    debug_assert!(output.len() >= requested_usize);

    let initialized = fill(output.as_mut_ptr().cast::<T>(), requested);
    let initialized_usize = usize::try_from(initialized)
        .map_err(|_| ApiError::NegativeFfiOutputCount { count: initialized })?;
    if initialized_usize > requested_usize {
        return Err(ApiError::FfiOutputCountExceedsCapacity {
            count: initialized,
            capacity: requested,
        });
    }

    // SAFETY: `fill` initialized the prefix required by its contract, and both count bounds were
    // validated before making those elements visible to Rust.
    unsafe { out.set_len(initialized_usize) };
    Ok(())
}

/// Allocate and fill a raw FFI output buffer.
///
/// # Safety
///
/// `fill` must uphold the same initialization contract as [`fill_from_ffi`].
pub(crate) unsafe fn read_from_ffi<T: FfiOutput>(
    requested: i32,
    fill: impl FnOnce(*mut T, i32) -> i32,
) -> ApiResult<Vec<T>> {
    let mut out = Vec::new();
    // SAFETY: the caller supplies the initialization contract for `fill`.
    unsafe { fill_from_ffi(&mut out, requested, fill)? };
    Ok(out)
}

struct ClearOnDrop<'a, T> {
    out: &'a mut Vec<T>,
    armed: bool,
}

impl<T> Drop for ClearOnDrop<'_, T> {
    fn drop(&mut self) {
        if self.armed {
            self.out.clear();
        }
    }
}

/// Fill a raw FFI buffer, then explicitly convert its validated prefix into safe values.
///
/// # Safety
///
/// `fill` must uphold the same initialization contract as [`fill_from_ffi`].
pub(crate) unsafe fn fill_mapped_from_ffi<Raw: FfiOutput, Safe>(
    out: &mut Vec<Safe>,
    requested: i32,
    fill: impl FnOnce(*mut Raw, i32) -> i32,
    mut map: impl FnMut(Raw) -> Safe,
) -> ApiResult<()> {
    out.clear();

    assert_eq!(
        core::mem::size_of::<Raw>(),
        core::mem::size_of::<Safe>(),
        "mapped FFI output types must have identical sizes",
    );
    assert_eq!(
        core::mem::align_of::<Raw>(),
        core::mem::align_of::<Safe>(),
        "mapped FFI output types must have identical alignments",
    );
    assert_ne!(
        core::mem::size_of::<Raw>(),
        0,
        "mapped FFI output types must not be zero-sized",
    );

    let requested_usize =
        usize::try_from(requested).map_err(|_| ApiError::NegativeFfiOutputCapacity {
            capacity: requested,
        })?;
    if requested_usize == 0 {
        return Ok(());
    }

    out.try_reserve(requested_usize)
        .map_err(|_| ApiError::FfiOutputAllocationFailed)?;
    let output = out.spare_capacity_mut();
    debug_assert!(output.len() >= requested_usize);

    let initialized = fill(output.as_mut_ptr().cast::<Raw>(), requested);
    let initialized_usize = usize::try_from(initialized)
        .map_err(|_| ApiError::NegativeFfiOutputCount { count: initialized })?;
    if initialized_usize > requested_usize {
        return Err(ApiError::FfiOutputCountExceedsCapacity {
            count: initialized,
            capacity: requested,
        });
    }

    let mut guard = ClearOnDrop { out, armed: true };
    for index in 0..initialized_usize {
        // SAFETY: the native callback initialized this raw element, the layout checks above make
        // each raw slot coincide exactly with one safe slot, and the visible vector length excludes
        // the slot until `map` has returned and the safe value has been written.
        let raw = unsafe { guard.out.as_mut_ptr().cast::<Raw>().add(index).read() };
        let safe = map(raw);
        // SAFETY: capacity covers the full requested range. Every preceding slot already contains
        // a valid `Safe`, and this write initializes the next slot before extending the visible
        // prefix. `ClearOnDrop` drops that prefix if a later conversion unwinds.
        unsafe {
            guard.out.as_mut_ptr().add(index).write(safe);
            guard.out.set_len(index + 1);
        }
    }
    guard.armed = false;
    Ok(())
}

/// Allocate, fill, and explicitly convert an FFI output buffer into safe values.
///
/// # Safety
///
/// `fill` must uphold the same initialization contract as [`fill_from_ffi`].
pub(crate) unsafe fn read_mapped_from_ffi<Raw: FfiOutput, Safe>(
    requested: i32,
    fill: impl FnOnce(*mut Raw, i32) -> i32,
    map: impl FnMut(Raw) -> Safe,
) -> ApiResult<Vec<Safe>> {
    let mut out = Vec::new();
    // SAFETY: the caller supplies the initialization contract for `fill`.
    unsafe { fill_mapped_from_ffi(&mut out, requested, fill, map)? };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{fill_from_ffi, fill_mapped_from_ffi};
    use crate::{ApiError, ShapeId};
    use boxdd_sys::ffi;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn raw_shape(index: i32) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1: index,
            world0: 2,
            generation: 3,
        }
    }

    #[test]
    fn grows_from_existing_capacity_to_the_full_request() {
        let mut out = Vec::<ffi::b2ShapeId>::with_capacity(8);

        unsafe {
            fill_from_ffi(&mut out, 10, |ptr, capacity| {
                for index in 0..capacity {
                    ptr.add(index as usize).write(raw_shape(index));
                }
                capacity
            })
            .unwrap();
        }

        assert!(out.capacity() >= 10);
        assert_eq!(out.len(), 10);
        assert_eq!(out[9].index1, 9);
    }

    #[test]
    fn zero_request_clears_without_calling_native_fill() {
        let mut out = vec![raw_shape(7)];

        unsafe {
            fill_from_ffi(&mut out, 0, |_ptr, _capacity| {
                panic!("zero-capacity fill must not run")
            })
            .unwrap();
        }

        assert!(out.is_empty());
    }

    #[test]
    fn exact_and_excess_capacity_expose_only_initialized_values() {
        for existing_capacity in [2, 8] {
            let mut out = Vec::<ffi::b2ShapeId>::with_capacity(existing_capacity);
            let original_ptr = out.as_ptr();

            unsafe {
                fill_from_ffi(&mut out, 2, |ptr, _capacity| {
                    ptr.write(raw_shape(11));
                    ptr.add(1).write(raw_shape(12));
                    2
                })
                .unwrap();
            }

            assert_eq!(out.len(), 2);
            assert_eq!(out[0].index1, 11);
            assert_eq!(out[1].index1, 12);
            assert_eq!(out.as_ptr(), original_ptr);
        }
    }

    #[test]
    fn repeated_fill_reuses_the_allocation() {
        let mut out = Vec::<ffi::b2ShapeId>::with_capacity(4);

        unsafe {
            fill_from_ffi(&mut out, 4, |ptr, _capacity| {
                ptr.write(raw_shape(1));
                1
            })
            .unwrap();
        }
        let original_ptr = out.as_ptr();

        unsafe {
            fill_from_ffi(&mut out, 4, |ptr, _capacity| {
                ptr.write(raw_shape(2));
                1
            })
            .unwrap();
        }

        assert_eq!(out.as_ptr(), original_ptr);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].index1, 2);
    }

    #[test]
    fn negative_request_is_rejected_after_clearing() {
        let mut out = vec![raw_shape(7)];

        let error = unsafe { fill_from_ffi(&mut out, -1, |_ptr, _capacity| 0) }.unwrap_err();

        assert_eq!(error, ApiError::NegativeFfiOutputCapacity { capacity: -1 });
        assert!(out.is_empty());
    }

    #[test]
    fn negative_native_count_is_rejected_without_committing_length() {
        let mut out = vec![raw_shape(7)];

        let error = unsafe { fill_from_ffi(&mut out, 2, |_ptr, _capacity| -1) }.unwrap_err();

        assert_eq!(error, ApiError::NegativeFfiOutputCount { count: -1 });
        assert!(out.is_empty());
    }

    #[test]
    fn excessive_native_count_is_rejected_without_committing_length() {
        let mut out = vec![raw_shape(7)];

        let error = unsafe { fill_from_ffi(&mut out, 2, |_ptr, _capacity| 3) }.unwrap_err();

        assert_eq!(
            error,
            ApiError::FfiOutputCountExceedsCapacity {
                count: 3,
                capacity: 2,
            }
        );
        assert!(out.is_empty());
    }

    #[test]
    fn fill_panic_leaves_the_visible_length_at_zero() {
        let mut out = vec![raw_shape(7)];

        let panic = catch_unwind(AssertUnwindSafe(|| unsafe {
            let _ = fill_from_ffi(&mut out, 2, |ptr, _capacity| {
                ptr.write(raw_shape(1));
                panic!("synthetic native fill panic");
            });
        }));

        assert!(panic.is_err());
        assert!(out.is_empty());
    }

    #[test]
    fn mapped_fill_panic_clears_partially_converted_safe_values() {
        let mut out = vec![ShapeId::from_raw(raw_shape(99))];

        let panic = catch_unwind(AssertUnwindSafe(|| unsafe {
            let _ = fill_mapped_from_ffi(
                &mut out,
                2,
                |ptr: *mut ffi::b2ShapeId, _capacity| {
                    ptr.write(raw_shape(1));
                    ptr.add(1).write(raw_shape(2));
                    2
                },
                |raw| {
                    if raw.index1 == 2 {
                        panic!("synthetic conversion panic");
                    }
                    ShapeId::from_raw(raw)
                },
            );
        }));

        assert!(panic.is_err());
        assert!(out.is_empty());
    }

    #[test]
    fn mapped_fill_reuses_the_safe_output_allocation_as_raw_storage() {
        let mut out = Vec::<ShapeId>::with_capacity(2);
        let expected_raw_ptr = out.as_mut_ptr().cast::<ffi::b2ShapeId>();

        unsafe {
            fill_mapped_from_ffi(
                &mut out,
                2,
                |ptr: *mut ffi::b2ShapeId, capacity| {
                    assert_eq!(ptr, expected_raw_ptr);
                    for index in 0..capacity {
                        ptr.add(index as usize).write(raw_shape(index));
                    }
                    capacity
                },
                ShapeId::from_raw,
            )
            .unwrap();
        }

        assert_eq!(out.as_mut_ptr().cast::<ffi::b2ShapeId>(), expected_raw_ptr);
        assert_eq!(out.iter().map(|id| id.index1).collect::<Vec<_>>(), [0, 1]);
    }
}
