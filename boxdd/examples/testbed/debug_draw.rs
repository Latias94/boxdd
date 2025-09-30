use boxdd as bd;
use dear_imgui_rs as imgui;

pub struct ImguiDebugDraw<'a> {
    pub ui: &'a imgui::Ui,
    pub pixels_per_meter: f32,
}

impl bd::DebugDraw for ImguiDebugDraw<'_> {
    fn draw_segment(&mut self, p1: bd::Vec2, p2: bd::Vec2, color: i32) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        dl.add_line(w2s(p1), w2s(p2), col).build();
    }
    fn draw_polygon(&mut self, vertices: &[bd::Vec2], color: i32) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        for i in 0..vertices.len() {
            let a = w2s(vertices[i]);
            let b = w2s(vertices[(i + 1) % vertices.len()]);
            dl.add_line(a, b, col).build();
        }
    }
    fn draw_circle(&mut self, center: bd::Vec2, radius: f32, color: i32) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        dl.add_circle(w2s(center), radius * s, col).thickness(1.0).build();
    }
    fn draw_solid_polygon(
        &mut self,
        xf: bd::Transform,
        vertices: &[bd::Vec2],
        _radius: f32,
        color: i32,
    ) {
        if vertices.is_empty() {
            return;
        }
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let transform = |v: bd::Vec2| xf.transform_point(v);
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let pts: Vec<[f32; 2]> = vertices.iter().map(|&v| w2s(transform(v))).collect();
        let fill = 0x4000_0000u32 | ((color as u32) & 0x00ff_ffff);
        dl.add_concave_poly_filled(&pts, fill);
        // outline
        let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        for i in 0..pts.len() {
            dl.add_line(pts[i], pts[(i + 1) % pts.len()], col).build();
        }
    }
    fn draw_solid_circle(&mut self, xf: bd::Transform, radius: f32, color: i32) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let center = xf.position();
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let fill = 0x4000_0000u32 | ((color as u32) & 0x00ff_ffff);
        let outline = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        // Approximate filled circle with polygon
        let steps = 28;
        let mut pts: Vec<[f32; 2]> = Vec::with_capacity(steps);
        for i in 0..steps {
            let ang = (i as f32) * (std::f32::consts::TAU / steps as f32);
            let v = bd::Vec2::new(center.x + radius * ang.cos(), center.y + radius * ang.sin());
            pts.push(w2s(v));
        }
        dl.add_concave_poly_filled(&pts, fill);
        // Outline
        for i in 0..steps {
            dl.add_line(pts[i], pts[(i + 1) % steps], outline).build();
        }
    }
    fn draw_solid_capsule(&mut self, p1: bd::Vec2, p2: bd::Vec2, radius: f32, color: i32) {
        // Approximate: thick line + end circles
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let outline = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        let fill = 0x4000_0000u32 | ((color as u32) & 0x00ff_ffff);
        dl.add_line(w2s(p1), w2s(p2), fill)
            .thickness(radius * 2.0 * s)
            .build();
        dl.add_circle(w2s(p1), radius * s, fill).thickness(1.0).build();
        dl.add_circle(w2s(p2), radius * s, fill).thickness(1.0).build();
        dl.add_line(w2s(p1), w2s(p2), outline).thickness(1.0).build();
    }
    fn draw_transform(&mut self, xf: bd::Transform) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let len = 0.5;
        let rot = xf.rotation();
        let x_axis = rot.rotate_vec(bd::Vec2::new(len, 0.0));
        let y_axis = rot.rotate_vec(bd::Vec2::new(0.0, len));
        let p = xf.position();
        dl.add_line(w2s(p), w2s(bd::Vec2::new(p.x + x_axis.x, p.y + x_axis.y)), 0xffff0000)
            .build();
        dl.add_line(w2s(p), w2s(bd::Vec2::new(p.x + y_axis.x, p.y + y_axis.y)), 0xff00ff00)
            .build();
    }
    fn draw_point(&mut self, p: bd::Vec2, size: f32, color: i32) {
        let dl = self.ui.get_foreground_draw_list();
        let ds = self.ui.io().display_size();
        let origin = [ds[0] * 0.5, ds[1] * 0.5];
        let s = self.pixels_per_meter;
        let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
        let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
        // Small dot as tiny polygon (triangle approximation)
        let r = (size.max(2.0)) * 0.5;
        let c = w2s(p);
        let pts = [[c[0] - r, c[1]], [c[0] + r, c[1]], [c[0], c[1] + r]];
        dl.add_concave_poly_filled(&pts, col);
    }
}
