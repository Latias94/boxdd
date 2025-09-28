use boxdd::prelude::*;

#[test]
fn empty_world_steps_and_stays_empty() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    for _ in 0..60 {
        world.step(1.0 / 60.0, 1);
    }

    let c = world.counters();
    assert_eq!(c.body_count, 0);
    assert_eq!(world.awake_body_count(), 0);
}
