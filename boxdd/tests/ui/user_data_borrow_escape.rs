use boxdd::Foundation;
fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let world = foundation.create_world(foundation.world_def()).unwrap();
    let escaped = world.with_user_data::<String, _>(|value| value).unwrap();
    drop(world);
    let _ = escaped;
}
