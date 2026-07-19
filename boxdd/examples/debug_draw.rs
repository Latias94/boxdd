use boxdd::{
    BodyBuilder, DebugDraw, DebugDrawOptions, HexColor, Position, ShapeDef, Vec2, World, WorldDef,
    WorldTransform, shapes,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def)?;
    // ground (ID-style, no RAII wrappers)
    let ground_def = BodyBuilder::new().build();
    let ground_id = world.create_body_id(ground_def);
    let sdef = ShapeDef::builder().density(0.0).build();
    let ground_poly = shapes::box_polygon(10.0, 0.5);
    let _gs = world.create_polygon_shape_for(ground_id, &sdef, &ground_poly);

    // dynamic box
    let body_def = BodyBuilder::new().position(Vec2::new(0.0, 4.0)).build();
    let body_id = world.create_body_id(body_def);
    let sdef_dyn = ShapeDef::builder().density(1.0).build();
    let dyn_poly = shapes::box_polygon(0.5, 0.5);
    let _bs = world.create_polygon_shape_for(body_id, &sdef_dyn, &dyn_poly);

    let mut drawer = Printer;
    let opts = DebugDrawOptions::default();
    for _ in 0..3 {
        world.step(1.0 / 60.0, 4);
        world.debug_draw(&mut drawer, opts);
    }
    Ok(())
}
