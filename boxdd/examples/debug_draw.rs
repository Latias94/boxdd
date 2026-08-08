use boxdd::{
    BodyBuilder, DebugDraw, DebugDrawOptions, HexColor, Position, ShapeDef, Vec2, WorldTransform,
    shapes,
};

struct Printer;

impl DebugDraw for Printer {
    fn draw_polygon(&mut self, transform: WorldTransform, vertices: &[Vec2], color: HexColor) {
        println!(
            "polygon {} verts at ({:.2},{:.2}) color={:#x}",
            vertices.len(),
            transform.position().x,
            transform.position().y,
            color.rgb_u32()
        );
    }
    fn draw_segment(&mut self, p1: Position, p2: Position, _color: HexColor) {
        println!(
            "segment ({:.2},{:.2})->({:.2},{:.2})",
            p1.x, p1.y, p2.x, p2.y
        );
    }
    fn draw_string(&mut self, p: Position, s: &str, _color: HexColor) {
        println!("label at ({:.2},{:.2}): {}", p.x, p.y, s);
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let def = boxdd::WorldBuilder::from(foundation.world_def())
        .gravity(Vec2::new(0.0, -9.8))
        .build()?;
    let mut world = foundation.create_world(def)?;
    // ground (ID-style, no RAII wrappers)
    let ground_def = BodyBuilder::from(foundation.body_def()).build()?;
    let ground_id = world.create_body(ground_def)?;
    let sdef = ShapeDef::builder().density(0.0).build()?;
    let ground_poly = shapes::box_polygon(10.0, 0.5).expect("valid polygon geometry");
    let _gs = world.body(ground_id)?.create_polygon(&sdef, &ground_poly)?;

    // dynamic box
    let body_def = BodyBuilder::from(foundation.body_def())
        .position(Vec2::new(0.0, 4.0))
        .build()?;
    let body_id = world.create_body(body_def)?;
    let sdef_dyn = ShapeDef::builder().density(1.0).build()?;
    let dyn_poly = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
    let _bs = world.body(body_id)?.create_polygon(&sdef_dyn, &dyn_poly)?;

    let mut drawer = Printer;
    let opts = DebugDrawOptions::default();
    for _ in 0..3 {
        drop(world.step(1.0 / 60.0, 4)?);
        world.debug_draw(&mut drawer, opts)?;
    }
    Ok(())
}
