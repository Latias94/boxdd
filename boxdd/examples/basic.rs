use boxdd::Vec2;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let def = boxdd::WorldBuilder::from(foundation.world_def())
        .gravity(Vec2::new(0.0, -9.8))
        .build()?;
    let mut world = foundation.create_world(def)?;
    println!("gravity before: {:?}", world.gravity()?);
    drop(world.step(1.0 / 60.0, 4)?);
    Ok(())
}
