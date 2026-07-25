#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    let mut world = boxdd::World::new(boxdd::WorldDef::default()).unwrap();
    let options = boxdd::DebugDrawOptions::default();
    let mut commands = Vec::new();
    let mut drawer = ();

    let _ = world.debug_draw_collect(options);
    let _ = world.try_debug_draw_collect(options);
    world.debug_draw_collect_into(&mut commands, options);
    let _ = world.try_debug_draw_collect_into(&mut commands, options);
    world.debug_draw(&mut drawer, options);
    let _ = world.try_debug_draw(&mut drawer, options);
}
