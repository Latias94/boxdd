//! Abort-only subprocess probe for a real native Box2D callback.

use boxdd::{Aabb, BodyBuilder, BodyType, Position, QueryFilter, ShapeDef, Vec2, World, WorldDef};

const CALLBACK_MARKER: &str = "boxdd-panic-abort-probe: callback-entered";
const AFTER_MARKER: &str = "boxdd-panic-abort-probe: after-query";

fn main() {
    // This binary is intentionally useful only with the dedicated abort profile. Keep the
    // runtime guard as well as the test assertion so an accidental profile regression is loud.
    if !cfg!(panic = "abort") {
        eprintln!(
            "boxdd-panic-abort-probe: expected panic=abort, got {}",
            if cfg!(panic = "unwind") {
                "panic=unwind"
            } else {
                "an unknown panic strategy"
            }
        );
        std::process::exit(91);
    }

    let mut world = World::new(WorldDef::default()).expect("probe world creation");
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([0.0_f32, 0.0])
            .build(),
    );
    world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &boxdd::shapes::circle([0.0_f32, 0.0], 0.5),
    );

    eprintln!("boxdd-panic-abort-probe: before-query");
    world.visit_overlap_aabb(
        Position::ZERO,
        Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)),
        QueryFilter::default(),
        |_| {
            eprintln!("{CALLBACK_MARKER}");
            panic!("intentional panic=abort callback probe");
        },
    );

    // Reaching this line would mean native traversal continued after a Rust callback panic.
    eprintln!("{AFTER_MARKER}");
}
