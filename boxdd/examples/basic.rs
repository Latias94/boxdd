use boxdd::{Vec2, World, WorldDef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def = WorldDef::builder()
        .gravity(Vec2::new(0.0, -9.8))
        .worker_count(0)
        .build();
    let mut world = World::new(def)?;
    println!("gravity before: {:?}", world.gravity());
    world.step(1.0 / 60.0, 4);
    Ok(())
}
