use boxdd::{Foundation, RecordingLimits, ReplayConfig, ReplayPlayer};
fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let recording = world
        .start_recording(RecordingLimits::default())
        .unwrap()
        .finish()
        .unwrap();
    drop(world);

    let player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &recording,
        ReplayConfig::default(),
    )
    .unwrap();
    let escaped = player.with_view(|view| view.body(0)).unwrap();
    drop(player);
    let _ = escaped.map(|body| body.position());
}
