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

    let mut world = boxdd::World::new(boxdd::WorldDef::default()).unwrap();
    world.set_custom_filter(filter);
    let _ = world.try_set_custom_filter(filter);
    world.clear_custom_filter();
    let _ = world.try_clear_custom_filter();
    world.set_custom_filter_callback(Some(filter));
    let _ = world.try_set_custom_filter_callback(Some(filter));

    world.set_pre_solve(pre_solve);
    let _ = world.try_set_pre_solve(pre_solve);
    world.clear_pre_solve();
    let _ = world.try_clear_pre_solve();
    world.set_pre_solve_callback(Some(pre_solve));
    let _ = world.try_set_pre_solve_callback(Some(pre_solve));

    let mix = |left: boxdd::MaterialMixInput, right: boxdd::MaterialMixInput| {
        left.coefficient.max(right.coefficient)
    };
    world.set_friction_callback(mix);
    let _ = world.try_set_friction_callback(mix);
    world.clear_friction_callback();
    let _ = world.try_clear_friction_callback();
    world.set_restitution_callback(mix);
    let _ = world.try_set_restitution_callback(mix);
    world.clear_restitution_callback();
    let _ = world.try_clear_restitution_callback();

    let mut definition = boxdd::WorldDef::default();
    unsafe {
        definition.set_task_system_raw(1, None, None, core::ptr::null_mut());
    }
}
