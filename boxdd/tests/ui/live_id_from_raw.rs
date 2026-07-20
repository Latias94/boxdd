use boxdd::BodyId;
use boxdd_sys::ffi;

fn main() {
    let raw = ffi::b2BodyId {
        index1: 1,
        world0: 0,
        generation: 1,
    };
    let _ = BodyId::from_raw(raw);
}
