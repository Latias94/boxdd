use boxdd as bd;
use dear_imgui_rs as imgui;

pub struct ImguiDebugDraw<'a> {
    pub ui: &'a imgui::Ui,
    pub pixels_per_meter: f32,
}

fn imgui_color(color: bd::HexColor, alpha: u8) -> u32 {
    color.with_alpha(alpha)
}

impl ImguiDebugDraw<'_> {
    fn world_to_screen(&self, position: bd::Position) -> [f32; 2] {
        let display_size = self.ui.io().display_size();
        let origin = [display_size[0] * 0.5, display_size[1] * 0.5];
        let x = position.x as f32;
        let y = position.y as f32;
        [
            origin[0] + x * self.pixels_per_meter,
            display_size[1] - (origin[1] + y * self.pixels_per_meter),
        ]
    }
}

impl bd::DebugDraw for ImguiDebugDraw<'_> {
    fn draw_segment(&mut self, p1: bd::Position, p2: bd::Position, color: bd::HexColor) {
        let dl = self.ui.get_foreground_draw_list();
        let col = imgui_color(color, 0xff);
        dl.add_line(self.world_to_screen(p1), self.world_to_screen(p2), col)
            .build();
    }
    fn draw_polygon(
        &mut self,
        transform: bd::WorldTransform,
        vertices: &[bd::Vec2],
        color: bd::HexColor,
    ) {
        let dl = self.ui.get_foreground_draw_list();
        let col = imgui_color(color, 0xff);
        for i in 0..vertices.len() {
            let a = self.world_to_screen(transform.transform_point(vertices[i]));
            let b =
                self.world_to_screen(transform.transform_point(vertices[(i + 1) % vertices.len()]));
            dl.add_line(a, b, col).build();
        }
    }
    fn draw_circle(&mut self, center: bd::Position, radius: f32, color: bd::HexColor) {
        let dl = self.ui.get_foreground_draw_list();
        let col = imgui_color(color, 0xff);
        dl.add_circle(
            self.world_to_screen(center),
            radius * self.pixels_per_meter,
            col,
        )
        .thickness(1.0)
        .build();
    }
    fn draw_solid_polygon(
        &mut self,
        transform: bd::WorldTransform,
        vertices: &[bd::Vec2],
        _radius: f32,
        color: bd::HexColor,
    ) {
        if vertices.is_empty() {
            return;
        }
        let dl = self.ui.get_foreground_draw_list();
        let pts: Vec<[f32; 2]> = vertices
            .iter()
            .map(|&vertex| self.world_to_screen(transform.transform_point(vertex)))
            .collect();
        let fill = imgui_color(color, 0x40);
        dl.add_concave_poly_filled(&pts, fill);
        // outline
        let col = imgui_color(color, 0xff);
        for i in 0..pts.len() {
            dl.add_line(pts[i], pts[(i + 1) % pts.len()], col).build();
        }
    }
    fn draw_solid_circle(
        &mut self,
        transform: bd::WorldTransform,
        center: bd::Vec2,
        radius: f32,
        color: bd::HexColor,
    ) {
        let dl = self.ui.get_foreground_draw_list();
        let center = transform.transform_point(center);
        let fill = imgui_color(color, 0x40);
        let outline = imgui_color(color, 0xff);
        // Approximate filled circle with polygon
        let steps = 28;
        let mut pts: Vec<[f32; 2]> = Vec::with_capacity(steps);
        for i in 0..steps {
            let ang = (i as f32) * (std::f32::consts::TAU / steps as f32);
            let offset = bd::Vec2::new(radius * ang.cos(), radius * ang.sin());
            pts.push(self.world_to_screen(center.offset(offset)));
        }
        dl.add_concave_poly_filled(&pts, fill);
        // Outline
        for i in 0..steps {
            dl.add_line(pts[i], pts[(i + 1) % steps], outline).build();
        }
    }
    fn draw_solid_capsule(
        &mut self,
        p1: bd::Position,
        p2: bd::Position,
        radius: f32,
        color: bd::HexColor,
    ) {
        // Approximate: thick line + end circles
        let dl = self.ui.get_foreground_draw_list();
        let s = self.pixels_per_meter;
        let outline = imgui_color(color, 0xff);
        let fill = imgui_color(color, 0x40);
        let screen_p1 = self.world_to_screen(p1);
        let screen_p2 = self.world_to_screen(p2);
        dl.add_line(screen_p1, screen_p2, fill)
            .thickness(radius * 2.0 * s)
            .build();
        dl.add_circle(screen_p1, radius * s, fill)
            .thickness(1.0)
            .build();
        dl.add_circle(screen_p2, radius * s, fill)
            .thickness(1.0)
            .build();
        dl.add_line(screen_p1, screen_p2, outline)
            .thickness(1.0)
            .build();
    }
    fn draw_transform(&mut self, transform: bd::WorldTransform) {
        let dl = self.ui.get_foreground_draw_list();
        let len = 0.5;
        let rot = transform.rotation();
        let x_axis = rot.rotate_vec(bd::Vec2::new(len, 0.0));
        let y_axis = rot.rotate_vec(bd::Vec2::new(0.0, len));
        let p = transform.position();
        let screen_p = self.world_to_screen(p);
        dl.add_line(screen_p, self.world_to_screen(p.offset(x_axis)), 0xffff0000)
            .build();
        dl.add_line(screen_p, self.world_to_screen(p.offset(y_axis)), 0xff00ff00)
            .build();
    }
    fn draw_point(&mut self, p: bd::Position, size: f32, color: bd::HexColor) {
        let dl = self.ui.get_foreground_draw_list();
        let col = imgui_color(color, 0xff);
        // Small dot as tiny polygon (triangle approximation)
        let r = (size.max(2.0)) * 0.5;
        let c = self.world_to_screen(p);
        let pts = [[c[0] - r, c[1]], [c[0] + r, c[1]], [c[0], c[1] + r]];
        dl.add_concave_poly_filled(&pts, col);
    }

    fn draw_bounds(&mut self, bounds: bd::Aabb, color: bd::HexColor) {
        let dl = self.ui.get_foreground_draw_list();
        let lower = self.world_to_screen(bounds.lower().into());
        let upper = self.world_to_screen(bounds.upper().into());
        let min = [lower[0].min(upper[0]), lower[1].min(upper[1])];
        let max = [lower[0].max(upper[0]), lower[1].max(upper[1])];
        dl.add_rect(min, max, imgui_color(color, 0xff)).build();
    }
}
