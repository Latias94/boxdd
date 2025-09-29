#![cfg(feature = "serialize")]

use boxdd::{BodyBuilder, ShapeDef, Vec2, World, WorldDef, shapes};

#[test]
fn scene_roundtrip_basic() {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def).expect("create world");

    let a = world.create_body_id(BodyBuilder::new().position([-1.0, 2.0]).build());
    let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());

    // Add shapes
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(a, &sdef, &poly);
    let _ = world.create_polygon_shape_for(b, &sdef, &poly);

    // Optionally add joints here; some environments may assert in upstream when constraints are ill-posed.

    world.step(1.0 / 60.0, 4);

    let scene = boxdd::serialize::SceneSnapshot::take(&world);
    let json = serde_json::to_string(&scene).expect("serialize scene");
    let back: boxdd::serialize::SceneSnapshot =
        serde_json::from_str(&json).expect("deserialize scene");
    let world2 = back.rebuild();
    let round = boxdd::serialize::SceneSnapshot::take(&world2);

    assert_eq!(scene.bodies.len(), round.bodies.len(), "bodies len");
    assert_eq!(scene.joints.len(), round.joints.len(), "joints len");
    // Chains may not be present if not created; skip chain checks here.
}

#[test]
fn shape_flags_snapshot_recorded() {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def).expect("create world");
    let a = world.create_body_id(BodyBuilder::new().position([0.0, 1.0]).build());

    // Enable various flags on the ShapeDef and create via ID-style API so flags are recorded.
    let sdef = ShapeDef::builder()
        .density(1.0)
        .sensor(true)
        .enable_custom_filtering(true)
        .enable_sensor_events(true)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .enable_pre_solve_events(true)
        .invoke_contact_creation(true)
        .build();
    let circle = boxdd_sys::ffi::b2Circle {
        center: Vec2::new(0.0, 0.0).into(),
        radius: 0.25,
    };
    let _sid = world.create_circle_shape_for(a, &sdef, &circle);

    let scene = boxdd::serialize::SceneSnapshot::take(&world);
    // Find a circle shape and inspect serialized def flags.
    let mut found = false;
    for b in &scene.bodies {
        for sh in &b.shapes {
            if let boxdd::serialize::ShapeGeom::Circle { .. } = sh.geom {
                let val = serde_json::to_value(&sh.def).expect("serde shape def to value");
                assert_eq!(val["is_sensor"], true);
                assert_eq!(val["enable_custom_filtering"], true);
                assert_eq!(val["enable_sensor_events"], true);
                assert_eq!(val["enable_contact_events"], true);
                assert_eq!(val["enable_hit_events"], true);
                assert_eq!(val["enable_pre_solve_events"], true);
                assert_eq!(val["invoke_contact_creation"], true);
                found = true;
                break;
            }
        }
    }
    assert!(found, "did not find circle shape with expected flags");
}
