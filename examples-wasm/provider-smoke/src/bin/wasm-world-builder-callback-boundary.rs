#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    let _ = unsafe {
        boxdd::WorldDef::builder()
            .task_system_raw(1, None, None, core::ptr::null_mut())
            .build()
    };
}
