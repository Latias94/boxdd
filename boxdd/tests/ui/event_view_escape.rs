use boxdd::{ContactEventsView, Foundation, World};

fn escape(world: &mut World) -> ContactEventsView<'_> {
    let completed = world.step(0.0, 1).unwrap();
    completed.contact_events().unwrap()
}

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation.create_world(foundation.world_def()).unwrap();
    let _ = escape(&mut world);
}
