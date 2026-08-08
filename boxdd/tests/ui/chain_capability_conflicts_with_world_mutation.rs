use boxdd::{ChainDef, Foundation, Vec2};

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let body_id = world
        .create_body(world.body_builder().build().unwrap())
        .unwrap();
    let definition = ChainDef::builder()
        .points([
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build()
        .unwrap();
    let chain_id = world
        .body(body_id)
        .unwrap()
        .create_chain(&definition)
        .unwrap();
    let chain = world.chain(chain_id).unwrap();
    let _next = world.create_body(world.body_builder().build().unwrap());
    drop(chain);
}
