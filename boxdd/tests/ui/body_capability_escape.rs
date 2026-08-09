use boxdd::{Body, BodyId, World};

fn escape(world: &mut World, body_id: BodyId) -> Body<'static> {
    world.body(body_id).unwrap()
}

fn main() {
    let _ = escape;
}
