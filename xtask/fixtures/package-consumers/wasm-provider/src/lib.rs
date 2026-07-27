const fn has_nonzero_byte(bytes: &[u8]) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0 {
            return true;
        }
        index += 1;
    }
    false
}

const fn strings_equal(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const _: () = {
    assert!(boxdd_sys::SNAPSHOT_LAYOUT_HASH != 0);
    assert!(has_nonzero_byte(&boxdd_sys::PRIVATE_ABI_HASH));
    assert!(strings_equal(boxdd_sys::PROVIDER_ADAPTER, "wasm-provider"));
};

#[cfg(feature = "double-precision")]
const _: () = assert!(boxdd_sys::IS_DOUBLE_PRECISION);

#[cfg(not(feature = "double-precision"))]
const _: () = assert!(!boxdd_sys::IS_DOUBLE_PRECISION);

pub fn provider_identity() -> (&'static str, usize) {
    (boxdd_sys::PROVIDER_ADAPTER, boxdd_sys::UPSTREAM_SHA.len())
}
