#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    struct Drawer;
    impl boxdd::DebugDraw for Drawer {}

    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let _ = world.debug_draw(&mut Drawer, boxdd::DebugDrawOptions::default());
    let _ = world.debug_draw_collect(boxdd::DebugDrawOptions::default());
}
