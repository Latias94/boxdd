use boxdd::TreeProxyId;

fn export_raw(proxy: TreeProxyId) {
    let _ = proxy.into_raw();
}

fn main() {
    let _ = TreeProxyId::from_raw(0);
}
