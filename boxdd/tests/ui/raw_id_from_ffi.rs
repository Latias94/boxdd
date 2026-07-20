use boxdd::RawBodyId;
use boxdd_sys::ffi;

fn main() {
    let raw = ffi::b2BodyId {
        index1: 1,
        world0: 0,
        generation: 1,
    };
    let world = ffi::b2WorldId {
        index1: 1,
        generation: 1,
    };
    let _ = RawBodyId::from_ffi(raw, world);
}
