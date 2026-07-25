#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use std::sync::Arc;

    let _ = boxdd::FoundationConfig::default().with_assert_hook(Arc::new(|_, _, _| {}));
    let _ = boxdd::FoundationConfig::default().with_log_hook(Arc::new(|_| {}));
}
