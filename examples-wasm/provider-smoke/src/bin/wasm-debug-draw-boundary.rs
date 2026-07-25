#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    struct Drawer;
    impl boxdd::DebugDraw for Drawer {}

    let mut world = boxdd::World::new(boxdd::WorldDef::default()).unwrap();
    world.debug_draw(&mut Drawer, boxdd::DebugDrawOptions::default());
    let _ = world.debug_draw_collect(boxdd::DebugDrawOptions::default());
}
