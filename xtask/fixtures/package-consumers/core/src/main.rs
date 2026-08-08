use boxdd::{Aabb, Foundation, Position, QueryFilter, Vec2, WorldBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "double-precision")]
    assert_eq!(std::mem::size_of::<boxdd::WorldScalar>(), 8);
    #[cfg(not(feature = "double-precision"))]
    assert_eq!(std::mem::size_of::<boxdd::WorldScalar>(), 4);

    let foundation = Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        WorldBuilder::from(foundation.world_def())
            .gravity(Vec2::new(0.0, -9.8))
            .build()?,
    )?;
    drop(world.step(1.0 / 60.0, 4)?);
    assert_eq!(world.gravity()?, Vec2::new(0.0, -9.8));
    let query = world.query()?;
    let hits = query.overlap_aabb(
        Position::ZERO,
        Aabb::new([-1.0, -1.0], [1.0, 1.0])?,
        QueryFilter::default(),
    )?;
    assert!(hits.is_empty());
    drop(query);
    drop(world);
    Ok(())
}
