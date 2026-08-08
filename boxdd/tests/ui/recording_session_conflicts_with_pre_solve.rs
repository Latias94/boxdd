use boxdd::{Foundation, RecordingLimits};

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let session = world
        .start_recording(RecordingLimits::default())
        .unwrap();
    world
        .set_pre_solve(|_, _, _, _| true)
        .unwrap();
    drop(session);
}
