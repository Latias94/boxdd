fn main() {
    assert_eq!(boxdd_sys::adapter::ADAPTER_ABI_VERSION, 2);
    assert_eq!(boxdd_sys::UPSTREAM_SHA.len(), 40);
    assert_eq!(boxdd_sys::PROVIDER_ADAPTER, "vendored");
}
