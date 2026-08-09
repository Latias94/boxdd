#[test]
fn empty_world_steps_and_stays_empty() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .build()
                .unwrap(),
        )
        .unwrap();

    for _ in 0..60 {
        drop(world.step(1.0 / 60.0, 1).unwrap());
    }

    let c = world.counters().unwrap();
    assert_eq!(c.body_count, 0);
    assert_eq!(world.awake_body_count().unwrap(), 0);
}
