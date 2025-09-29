// Demonstrate taking and rebuilding a scene snapshot (requires --features serialize)

use boxdd::{BodyBuilder, ShapeDef, Vec2, World, WorldDef, shapes};

fn main() {
    // Build a tiny world with two bodies and a revolute joint
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def).expect("create world");

    let a = world.create_body_id(BodyBuilder::new().position([-1.0, 2.0]).build());
    let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());

    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(a, &sdef, &poly);
    let _ = world.create_polygon_shape_for(b, &sdef, &poly);

    // Build a revolute joint at world origin
    let _jid = world.create_revolute_joint_world_id(a, b, [0.0, 2.0]);

    // Add a simple chain (polyline) to body A via ID-style API so it is captured by snapshot
    let chain_def = boxdd::shapes::chain::ChainDef::builder()
        .points([[-0.5, 0.0], [0.0, 0.5], [0.5, 0.0]].into_iter())
        .is_loop(false)
        .build();
    let _chain = world.create_chain_for_id(a, &chain_def);

    // Timestep once
    world.step(1.0 / 60.0, 4);

    // Take scene snapshot and serialize to JSON
    let scene = boxdd::serialize::SceneSnapshot::take(&world);
    let json = serde_json::to_string_pretty(&scene).expect("serialize scene");
    println!("scene json chars: {}", json.len());

    // Rebuild world from snapshot
    let world2 = scene.rebuild();

    // Validate body counts match
    let n1 = world.body_ids().len();
    let n2 = world2.body_ids().len();
    assert_eq!(n1, n2, "body count mismatch after rebuild");

    println!("ok: {} bodies round-tripped", n2);
}
