use boxdd::{prelude::*, shapes};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn material_mix_callbacks_receive_material_ids_and_can_override_restitution() {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .restitution_threshold(0.0)
            .build(),
    )
    .unwrap();

    let ground = world.create_body_id(BodyBuilder::new().build());
    let ground_def = ShapeDef::builder()
        .density(0.0)
        .material(
            SurfaceMaterial::default()
                .friction(0.9)
                .restitution(0.0)
                .user_material_id(11),
        )
        .build();
    let _ground_shape =
        world.create_polygon_shape_for(ground, &ground_def, &shapes::box_polygon(20.0, 0.5));

    let ball = world.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 5.0])
            .build(),
    );
    let ball_def = ShapeDef::builder()
        .density(1.0)
        .material(
            SurfaceMaterial::default()
                .friction(0.8)
                .restitution(0.0)
                .user_material_id(22),
        )
        .build();
    let _ball_shape =
        world.create_circle_shape_for(ball.id(), &ball_def, &shapes::circle([0.0_f32, 0.0], 0.5));

    let friction_seen = Arc::new(AtomicBool::new(false));
    world.set_friction_callback({
        let friction_seen = Arc::clone(&friction_seen);
        move |a: MaterialMixInput, b: MaterialMixInput| {
            let saw_expected_pair = (a.user_material_id == 11 && b.user_material_id == 22)
                || (a.user_material_id == 22 && b.user_material_id == 11);
            if saw_expected_pair {
                friction_seen.store(true, Ordering::SeqCst);
            }
            0.0
        }
    });

    let restitution_seen = Arc::new(AtomicBool::new(false));
    world.set_restitution_callback({
        let restitution_seen = Arc::clone(&restitution_seen);
        move |a: MaterialMixInput, b: MaterialMixInput| {
            let saw_expected_pair = (a.user_material_id == 11 && b.user_material_id == 22)
                || (a.user_material_id == 22 && b.user_material_id == 11);
            if saw_expected_pair {
                restitution_seen.store(true, Ordering::SeqCst);
            }
            1.0
        }
    });

    let mut bounced = false;
    for _ in 0..240 {
        world.step(1.0 / 120.0, 8);
        if ball.linear_velocity().y > 0.1 {
            bounced = true;
            break;
        }
    }

    assert!(
        friction_seen.load(Ordering::SeqCst),
        "expected friction callback to receive shape material ids"
    );
    assert!(
        restitution_seen.load(Ordering::SeqCst),
        "expected restitution callback to receive shape material ids"
    );
    assert!(
        bounced,
        "expected restitution callback to override zero restitution and produce a bounce"
    );
}

#[test]
fn material_mix_callback_panic_is_caught_and_resumed_after_step() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    world.set_friction_callback(|_, _| -> f32 {
        panic!("boom in friction mix");
    });

    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder()
        .density(1.0)
        .material(
            SurfaceMaterial::default()
                .friction(0.5)
                .restitution(0.0)
                .user_material_id(1),
        )
        .build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(a, &sdef, &poly);
    let _ = world.create_polygon_shape_for(b, &sdef, &poly);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.step(1.0 / 60.0, 1);
    }));
    assert!(r.is_err());
}
