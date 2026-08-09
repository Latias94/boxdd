use bevy_boxdd::BoxddPhysicsContext;

fn mutate_native_world(context: &mut BoxddPhysicsContext) {
    let _world = context.world_mut();
}

fn main() {}
