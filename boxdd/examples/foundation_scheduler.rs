use boxdd::{Foundation, FoundationConfig, WorkerCount, WorldBuilder, WorldCapacity};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Foundation configuration freezes before the first safe native call.
    let foundation = Foundation::initialize(FoundationConfig::new(1.0))?;

    // Current WASM providers are single-worker. Native targets use Box2D's built-in scheduler.
    let requested_workers = if cfg!(target_family = "wasm") { 1 } else { 2 };
    let workers = WorkerCount::new(requested_workers)?;
    let initial_capacity = WorldCapacity::new(128, 512, 16, 128, 1024)?;
    let mut world = foundation.create_world(
        WorldBuilder::from(foundation.world_def())
            .worker_count(workers)
            .capacity(initial_capacity)
            .build()?,
    )?;

    println!(
        "foundation: length_units_per_meter={} worlds={} workers={}",
        foundation.config().length_units_per_meter(),
        foundation.activity().ordinary_worlds,
        world.worker_count()?.get()
    );
    println!(
        "world: bounds={:?} maximum_capacity={:?}",
        world.bounds()?,
        world.maximum_capacity()?
    );

    // Runtime changes are owner-thread step-boundary operations. They do not make World sendable.
    world.set_worker_count(WorkerCount::default())?;
    drop(world.step(0.0, 1)?);
    drop(world);

    assert_eq!(foundation.activity().ordinary_worlds, 0);
    Ok(())
}
