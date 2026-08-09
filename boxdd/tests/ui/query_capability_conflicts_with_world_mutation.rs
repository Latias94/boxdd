use boxdd::Foundation;

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let query = world.query().unwrap();
    let _next = world.create_body(world.body_builder().build().unwrap());
    drop(query);
}
