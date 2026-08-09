use boxdd::{BodyBuilder, BodyDef, BodyId, Foundation, JointBase, World, WorldDef};

fn deleted_context_free_apis(
    foundation: &'static Foundation,
    body_a: BodyId,
    body_b: BodyId,
) {
    let _ = WorldDef::default();
    let _ = WorldDef::builder();
    let _ = BodyDef::default();
    let _ = BodyDef::builder();
    let _ = BodyBuilder::new();
    let _: BodyBuilder = Default::default();
    let _ = JointBase::new(body_a, body_b);
    let _ = foundation.joint_base(body_a, body_b);
    let _ = World::new(foundation.world_def());
}

fn main() {
    let _ = deleted_context_free_apis;
}
