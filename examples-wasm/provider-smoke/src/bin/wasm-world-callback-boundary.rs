#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    fn filter(_: boxdd::ShapeId, _: boxdd::ShapeId) -> bool {
        true
    }

    fn pre_solve(_: boxdd::ShapeId, _: boxdd::ShapeId, _: boxdd::Position, _: boxdd::Vec2) -> bool {
        true
    }

    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let _ = world.set_custom_filter(filter);
    let _ = world.clear_custom_filter();

    let _ = world.set_pre_solve(pre_solve);
    let _ = world.clear_pre_solve();

    let mix = |left: boxdd::MaterialMixInput, right: boxdd::MaterialMixInput| {
        left.coefficient.max(right.coefficient)
    };
    let mixer_id = boxdd::MixerId::from_bytes([0xB1; 32]);
    let _ = world.set_friction_callback(mixer_id, mix);
    let _ = world.clear_friction_callback();
    let _ = world.set_restitution_callback(mixer_id, mix);
    let _ = world.clear_restitution_callback();
}
