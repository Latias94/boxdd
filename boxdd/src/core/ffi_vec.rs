pub(crate) unsafe fn fill_from_ffi<T>(
    out: &mut Vec<T>,
    capacity: usize,
    fill: impl FnOnce(*mut T, i32) -> i32,
) {
    out.clear();
    if capacity == 0 {
        return;
    }
    if out.capacity() < capacity {
        out.reserve(capacity - out.capacity());
    }
    let cap = i32::try_from(capacity).expect("ffi capacity exceeds i32::MAX");
    let wrote = fill(out.as_mut_ptr(), cap).max(0) as usize;
    unsafe { out.set_len(wrote.min(capacity)) };
}

pub(crate) unsafe fn read_from_ffi<T>(
    capacity: usize,
    fill: impl FnOnce(*mut T, i32) -> i32,
) -> Vec<T> {
    let mut out = Vec::new();
    unsafe { fill_from_ffi(&mut out, capacity, fill) };
    out
}
