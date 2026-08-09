use crate::error::{Error, Result};
use boxdd_sys::ffi;

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
) -> Result<()> {
    out.clear();

    let requested_usize =
        usize::try_from(requested).map_err(|_| Error::NegativeFfiOutputCapacity {
            capacity: requested,
        })?;
    if requested_usize == 0 {
        return Ok(());
    }

    out.try_reserve(requested_usize)
        .map_err(|_| Error::FfiOutputAllocationFailed)?;
    let output = out.spare_capacity_mut();
    debug_assert!(output.len() >= requested_usize);

    let initialized = fill(output.as_mut_ptr().cast::<T>(), requested);
    let initialized_usize = usize::try_from(initialized)
        .map_err(|_| Error::NegativeFfiOutputCount { count: initialized })?;
    if initialized_usize > requested_usize {
        return Err(Error::FfiOutputCountExceedsCapacity {
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
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    // SAFETY: the caller supplies the initialization contract for `fill`.
    unsafe { fill_from_ffi(&mut out, requested, fill)? };
    Ok(out)
}

/// Allocate, fill, and fallibly convert an FFI output buffer into safe values.
///
/// # Safety
///
/// `fill` must uphold the same initialization contract as [`fill_from_ffi`].
pub(crate) unsafe fn try_read_mapped_from_ffi<Raw: FfiOutput, Safe>(
    requested: i32,
    fill: impl FnOnce(*mut Raw, i32) -> i32,
    mut map: impl FnMut(Raw) -> Result<Safe>,
) -> Result<Vec<Safe>> {
    // Safe IDs include a Rust-only owner token, so native code first writes into its exact raw
    // layout. The owned result is then built directly from that validated raw prefix.
    let raw = unsafe { read_from_ffi(requested, fill)? };
    let mut out = Vec::new();
    out.try_reserve(raw.len())
        .map_err(|_| Error::FfiOutputAllocationFailed)?;
    for raw in raw {
        out.push(map(raw)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{fill_from_ffi, try_read_mapped_from_ffi};
    use crate::Error;
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

        assert_eq!(error, Error::NegativeFfiOutputCapacity { capacity: -1 });
        assert!(out.is_empty());
    }

    #[test]
    fn negative_native_count_is_rejected_without_committing_length() {
        let mut out = vec![raw_shape(7)];

        let error = unsafe { fill_from_ffi(&mut out, 2, |_ptr, _capacity| -1) }.unwrap_err();

        assert_eq!(error, Error::NegativeFfiOutputCount { count: -1 });
        assert!(out.is_empty());
    }

    #[test]
    fn excessive_native_count_is_rejected_without_committing_length() {
        let mut out = vec![raw_shape(7)];

        let error = unsafe { fill_from_ffi(&mut out, 2, |_ptr, _capacity| 3) }.unwrap_err();

        assert_eq!(
            error,
            Error::FfiOutputCountExceedsCapacity {
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
    fn mapped_read_discards_partial_output_on_error() {
        let error = unsafe {
            try_read_mapped_from_ffi(
                2,
                |ptr: *mut ffi::b2ShapeId, _capacity| {
                    ptr.write(raw_shape(1));
                    ptr.add(1).write(raw_shape(2));
                    2
                },
                |raw| {
                    if raw.index1 == 2 {
                        Err(Error::WrongWorld)
                    } else {
                        Ok(i64::from(raw.index1))
                    }
                },
            )
        }
        .unwrap_err();

        assert_eq!(error, Error::WrongWorld);
    }

    #[test]
    fn mapped_read_supports_a_different_safe_layout() {
        let out = unsafe {
            try_read_mapped_from_ffi(
                2,
                |ptr: *mut ffi::b2ShapeId, capacity| {
                    for index in 0..capacity {
                        ptr.add(index as usize).write(raw_shape(index));
                    }
                    capacity
                },
                |raw| Ok([i64::from(raw.index1), i64::from(raw.world0), 17]),
            )
            .unwrap()
        };

        assert_eq!(out, [[0, 2, 17], [1, 2, 17]]);
    }
}
