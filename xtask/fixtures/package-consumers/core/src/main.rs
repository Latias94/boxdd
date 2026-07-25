use boxdd::{Vec2, World, WorldDef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity(Vec2::new(0.0, -9.8))
            .build(),
    )?;
    world.step(1.0 / 60.0, 4);
    assert_eq!(world.gravity(), Vec2::new(0.0, -9.8));
    drop(world);
    Ok(())
}
