use boxdd::World;

fn require_send_sync<T: Send + Sync>() {}

fn main() {
    require_send_sync::<World>();
}
