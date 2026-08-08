use boxdd::Foundation;

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let body_id = world
        .create_body(world.body_builder().build().unwrap())
        .unwrap();
    let body = world.body(body_id).unwrap();
    let _next = world.create_body(world.body_builder().build().unwrap());
    drop(body);
}
