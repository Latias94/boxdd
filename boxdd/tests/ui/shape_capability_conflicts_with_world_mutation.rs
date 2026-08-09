use boxdd::{Foundation, ShapeDef};

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let body_id = world
        .create_body(world.body_builder().build().unwrap())
        .unwrap();
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let shape = world.shape(shape_id).unwrap();
    let _next = world.create_body(world.body_builder().build().unwrap());
    drop(shape);
}
