#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let options = boxdd::DebugDrawOptions::default();
    let mut commands = Vec::new();
    let mut drawer = ();

    let _ = world.debug_draw_collect(options);
    let _ = world.debug_draw_collect_into(&mut commands, options);
    let _ = world.debug_draw(&mut drawer, options);
}
