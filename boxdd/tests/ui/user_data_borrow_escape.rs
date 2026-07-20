use boxdd::{World, WorldDef};

fn main() {
    let world = World::new(WorldDef::default()).unwrap();
    let escaped: Option<&String> = world.with_user_data::<String, _>(|value| value);
    drop(world);
    let _ = escaped;
}
