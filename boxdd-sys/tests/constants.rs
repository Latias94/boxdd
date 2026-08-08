use boxdd_sys::{adapter, ffi};

const _: u64 = ffi::B2_DEFAULT_MASK_BITS;
const _: u32 = ffi::B2_ENABLE_VALIDATION;

#[test]
fn stable_constants_match_the_rust_build_contract() {
    assert_eq!(ffi::B2_DEFAULT_MASK_BITS, u64::MAX);
    assert_eq!(
        ffi::B2_ENABLE_VALIDATION,
        u32::from(cfg!(feature = "validate"))
    );

    let identity = adapter::verify_runtime_identity().expect("linked adapter identity");
    assert_eq!(identity.validation_enabled, ffi::B2_ENABLE_VALIDATION as u8);
}
