use boxdd::{Foundation, RecordingLimits};

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let query = world.query().unwrap();
    let _session = world
        .start_recording(RecordingLimits::default())
        .unwrap();
    drop(query);
}
