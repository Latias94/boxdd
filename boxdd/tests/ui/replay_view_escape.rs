use boxdd::{RecordingCapacity, ReplayConfig, ReplayPlayer, World, WorldDef};

fn main() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let recording = world.start_recording(RecordingCapacity::default()).finish();
    drop(world);

    let player = ReplayPlayer::open_recording(&recording, ReplayConfig::default()).unwrap();
    let escaped = player.with_view(|view| view.body(0)).unwrap();
    drop(player);
    let _ = escaped.map(|body| body.position());
}
