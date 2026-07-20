use boxdd::{World, WorldDef};

fn main() {
    let world = World::new(WorldDef::default()).unwrap();
    let escaped = world.with_body_events_view(|mut events| events.next());
    drop(world);
    let _ = escaped.map(|event| event.body_id());
}
