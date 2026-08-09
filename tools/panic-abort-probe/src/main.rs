//! Abort-only subprocess probe for a real native Box2D callback.

use boxdd::{Aabb, BodyBuilder, BodyType, Position, QueryFilter, ShapeDef, Vec2, shapes};

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

    let foundation = boxdd::Foundation::initialize_default().expect("probe foundation");
    let mut world = foundation
        .create_world(foundation.world_def())
        .expect("probe world creation");
    let body = world
        .create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Static)
                .position([0.0_f32, 0.0])
                .build()
                .expect("valid probe definition"),
        )
        .expect("probe body creation");
    world
        .body(body)
        .expect("probe body capability")
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5).expect("probe circle geometry"),
        )
        .expect("probe shape creation");

    eprintln!("boxdd-panic-abort-probe: before-query");
    world
        .query()
        .expect("probe query capability")
        .visit_overlap_aabb(
            Position::ZERO,
            Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0))
                .expect("probe query bounds"),
            QueryFilter::default(),
            |_| {
                eprintln!("{CALLBACK_MARKER}");
                panic!("intentional panic=abort callback probe");
            },
        )
        .expect("the callback must abort before returning a result");

    // Reaching this line would mean native traversal continued after a Rust callback panic.
    eprintln!("{AFTER_MARKER}");
}
