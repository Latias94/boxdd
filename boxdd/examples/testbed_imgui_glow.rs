//! Dear ImGui + Winit 0.30 + Glow testbed (dear-imgui 0.2 APIs)
//! Enable with: `cargo run -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed`

#[cfg(feature = "imgui-glow-testbed")]
fn main() {
    app::run();
}

#[cfg(not(feature = "imgui-glow-testbed"))]
fn main() {
    println!("Enable with --features imgui-glow-testbed");
}

#[cfg(feature = "imgui-glow-testbed")]
mod app {
    use std::{num::NonZeroU32, sync::Arc, time::Instant};

    use bd::prelude::*;
    use boxdd as bd;
    use dear_imgui as imgui;
    use dear_imgui_glow::GlowRenderer;
    use dear_imgui_winit::WinitPlatform;
    use glow::HasContext;
    use glutin::{
        config::ConfigTemplateBuilder,
        context::{ContextAttributesBuilder, PossiblyCurrentContext},
        display::{GetGlDisplay, GlDisplay},
        surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
    };
    use winit::raw_window_handle::HasWindowHandle;
    use winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{Window, WindowId},
    };

    pub fn run() {
        env_logger::init();
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let mut app = App::default();
        event_loop.run_app(&mut app).unwrap();
    }

    struct ImguiState {
        context: imgui::Context,
        platform: WinitPlatform,
        renderer: GlowRenderer,
        last_frame: Instant,
    }

    struct TestbedWindow {
        window: Arc<Window>,
        surface: Surface<WindowSurface>,
        context: PossiblyCurrentContext,
        imgui: ImguiState,
        physics: PhysicsApp,
    }

    #[derive(Default)]
    struct App {
        window: Option<TestbedWindow>,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_none() {
                match TestbedWindow::new(el) {
                    Ok(w) => {
                        w.window.request_redraw();
                        self.window = Some(w);
                    }
                    Err(e) => {
                        eprintln!("Failed to create window: {e}");
                        el.exit();
                    }
                }
            }
        }

        fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            let Some(w) = self.window.as_mut() else {
                return;
            };
            if id != w.window.id() {
                return;
            }
            let full: winit::event::Event<()> = winit::event::Event::WindowEvent {
                window_id: id,
                event: event.clone(),
            };
            w.imgui
                .platform
                .handle_event(&mut w.imgui.context, &w.window, &full);
            match event {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::RedrawRequested => {
                    if let Err(e) = w.render() {
                        eprintln!("Render error: {e}");
                        el.exit();
                    }
                }
                WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        w.surface.resize(
                            &w.context,
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        );
                    }
                    w.window.request_redraw();
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
            if let Some(w) = self.window.as_ref() {
                w.window.request_redraw();
            }
        }
    }

    impl TestbedWindow {
        fn new(event_loop: &ActiveEventLoop) -> Result<Self, Box<dyn std::error::Error>> {
            let window_attrs = winit::window::Window::default_attributes()
                .with_title("boxdd testbed")
                .with_inner_size(LogicalSize::new(1024.0, 720.0));

            let (window, cfg) = glutin_winit::DisplayBuilder::new()
                .with_window_attributes(Some(window_attrs))
                .build(event_loop, ConfigTemplateBuilder::new(), |mut configs| {
                    configs.next().unwrap()
                })?;
            let window = Arc::new(window.unwrap());

            let raw = window.window_handle()?.as_raw();
            let ctx_attribs = ContextAttributesBuilder::new().build(Some(raw));
            let not_current = unsafe { cfg.display().create_context(&cfg, &ctx_attribs)? };

            let surf_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
                .with_srgb(Some(true))
                .build(
                    raw,
                    NonZeroU32::new(1024).unwrap(),
                    NonZeroU32::new(720).unwrap(),
                );
            let surface = unsafe { cfg.display().create_window_surface(&cfg, &surf_attribs)? };
            use glutin::context::NotCurrentGlContext;
            let context = not_current.make_current(&surface)?;

            // Dear ImGui
            let mut imgui_ctx = imgui::Context::create();
            imgui_ctx.set_ini_filename(None::<String>).unwrap();
            let mut platform = WinitPlatform::new(&mut imgui_ctx);
            platform.attach_window(
                &window,
                dear_imgui_winit::HiDpiMode::Default,
                &mut imgui_ctx,
            );

            // Glow + renderer
            let gl = unsafe {
                glow::Context::from_loader_function_cstr(|s| {
                    context.display().get_proc_address(s).cast()
                })
            };
            let mut renderer = GlowRenderer::new(gl, &mut imgui_ctx)?;
            renderer.set_framebuffer_srgb_enabled(true);
            renderer.new_frame()?;
            let imgui = ImguiState {
                context: imgui_ctx,
                platform,
                renderer,
                last_frame: Instant::now(),
            };

            let physics = PhysicsApp::new()?;
            Ok(Self {
                window,
                surface,
                context,
                imgui,
                physics,
            })
        }

        fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            // Delta time + physics step
            let now = Instant::now();
            let dt = now - self.imgui.last_frame;
            self.imgui.context.io_mut().set_delta_time(dt.as_secs_f32());
            self.imgui.last_frame = now;
            self.physics.update();

            // UI
            self.imgui
                .platform
                .prepare_frame(&self.window, &mut self.imgui.context);
            let ui = self.imgui.context.frame();
            self.physics.ui(&ui);

            // Debug draw
            let mut dd = ImguiDebugDraw {
                ui: &ui,
                pixels_per_meter: 30.0,
            };
            self.physics
                .world
                .debug_draw(&mut dd, bd::DebugDrawOptions::default());

            // Clear + render
            let gl = self.imgui.renderer.gl_context().unwrap();
            unsafe {
                gl.enable(glow::FRAMEBUFFER_SRGB);
                gl.clear_color(0.06, 0.07, 0.09, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                gl.disable(glow::FRAMEBUFFER_SRGB);
            }
            self.imgui
                .platform
                .prepare_render_with_ui(&ui, &self.window);
            let draw_data = self.imgui.context.render();
            self.imgui.renderer.new_frame()?;
            self.imgui.renderer.render(&draw_data)?;
            self.surface.swap_buffers(&self.context)?;
            Ok(())
        }
    }

    // ImGui debug draw adapter
    struct ImguiDebugDraw<'a> {
        ui: &'a imgui::Ui,
        pixels_per_meter: f32,
    }
    impl bd::DebugDraw for ImguiDebugDraw<'_> {
        fn draw_segment(&mut self, p1: bd::Vec2, p2: bd::Vec2, color: i32) {
            let dl = self.ui.get_foreground_draw_list();
            let ds = self.ui.io().display_size();
            let origin = [ds[0] * 0.5, ds[1] * 0.5];
            let s = self.pixels_per_meter;
            let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
            let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
            dl.add_line(w2s(p1), w2s(p2), col).thickness(1.5).build();
        }
        fn draw_polygon(&mut self, vertices: &[bd::Vec2], color: i32) {
            if vertices.len() < 2 {
                return;
            }
            let dl = self.ui.get_foreground_draw_list();
            let ds = self.ui.io().display_size();
            let origin = [ds[0] * 0.5, ds[1] * 0.5];
            let s = self.pixels_per_meter;
            let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
            let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
            for i in 0..vertices.len() {
                let a = vertices[i];
                let b = vertices[(i + 1) % vertices.len()];
                dl.add_line(w2s(a), w2s(b), col).build();
            }
        }
        fn draw_circle(&mut self, center: bd::Vec2, radius: f32, color: i32) {
            let dl = self.ui.get_foreground_draw_list();
            let ds = self.ui.io().display_size();
            let origin = [ds[0] * 0.5, ds[1] * 0.5];
            let s = self.pixels_per_meter;
            let w2s = |v: bd::Vec2| [origin[0] + v.x * s, ds[1] - (origin[1] + v.y * s)];
            let col = 0xff00_0000u32 | ((color as u32) & 0x00ff_ffff);
            dl.add_circle(w2s(center), radius * s, col)
                .thickness(1.5)
                .build();
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
            let mut pts: Vec<[f32; 2]> = vertices.iter().map(|&v| w2s(transform(v))).collect();
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
            dl.add_circle(w2s(p1), radius * s, fill)
                .thickness(1.0)
                .build();
            dl.add_circle(w2s(p2), radius * s, fill)
                .thickness(1.0)
                .build();
            dl.add_line(w2s(p1), w2s(p2), outline)
                .thickness(1.0)
                .build();
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
            dl.add_line(
                w2s(p),
                w2s(bd::Vec2::new(p.x + x_axis.x, p.y + x_axis.y)),
                0xffff0000,
            )
            .build();
            dl.add_line(
                w2s(p),
                w2s(bd::Vec2::new(p.x + y_axis.x, p.y + y_axis.y)),
                0xff00ff00,
            )
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

    // ---------------- Physics Testbed (scenes + UI) ----------------

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    enum Scene {
        Pyramid,
        Bridge,
        Car,
        RevoluteMotor,
        PrismaticElevator,
        ChainWalkway,
        Sensors,
        Contacts,
        Bullet,
        Stacking,
        Shapes,
        Robustness,
        Events,
        Benchmark,
        Determinism,
    }

    struct PhysicsApp {
        world: bd::World,
        scene: Scene,
        gravity_y: f32,
        sub_steps: i32,
        running: bool,
        // Stats
        created_bodies: usize,
        created_shapes: usize,
        created_joints: usize,
        // Scene params
        pyramid_rows: i32,
        pyramid_cols: i32,
        bridge_planks: i32,
        car_motor_speed: f32,
        car_motor_torque: f32,
        car_hz: f32,
        car_dr: f32,
        revolute_lower_deg: f32,
        revolute_upper_deg: f32,
        revolute_speed: f32,
        revolute_torque: f32,
        prism_lower: f32,
        prism_upper: f32,
        prism_speed: f32,
        prism_force: f32,
        chain_boxes: i32,
        chain_amp: f32,
        chain_freq: f32,
        // Sensors
        sensor_band_y: f32,
        sensor_half_thickness: f32,
        sensor_mover_start_y: f32,
        sensor_radius: f32,
        // Contacts
        contact_box_half: f32,
        contact_speed: f32,
        contact_gap: f32,
        // Bullet
        bullet_dt: f32,
        bullet_substeps: i32,
        bullet_speed: f32,
        bullet_radius: f32,
        bullet_threshold: f32,
    }

    impl PhysicsApp {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let gravity_y = -9.8;
            let scene = Scene::Pyramid;
            let mut app = Self {
                world: bd::World::new(bd::WorldDef::builder().gravity([0.0, gravity_y]).build())?,
                scene,
                gravity_y,
                sub_steps: 4,
                running: true,
                created_bodies: 0,
                created_shapes: 0,
                created_joints: 0,
                pyramid_rows: 10,
                pyramid_cols: 10,
                bridge_planks: 20,
                car_motor_speed: 15.0,
                car_motor_torque: 40.0,
                car_hz: 4.0,
                car_dr: 0.7,
                revolute_lower_deg: -45.0,
                revolute_upper_deg: 45.0,
                revolute_speed: 2.0,
                revolute_torque: 50.0,
                prism_lower: 0.0,
                prism_upper: 4.0,
                prism_speed: 2.0,
                prism_force: 100.0,
                chain_boxes: 10,
                chain_amp: 0.4,
                chain_freq: 0.6,
                sensor_band_y: 1.5,
                sensor_half_thickness: 0.3,
                sensor_mover_start_y: 3.0,
                sensor_radius: 0.25,
                contact_box_half: 0.5,
                contact_speed: 2.0,
                contact_gap: 1.5,
                bullet_dt: 1.0 / 240.0,
                bullet_substeps: 16,
                bullet_speed: 60.0,
                bullet_radius: 0.3,
                bullet_threshold: 0.001,
            };
            app.build_scene();
            Ok(app)
        }
        fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.world = bd::World::new(
                bd::WorldDef::builder()
                    .gravity([0.0, self.gravity_y])
                    .build(),
            )?;
            self.build_scene();
            Ok(())
        }
        fn update(&mut self) {
            if self.running {
                match self.scene {
                    Scene::Bullet => self.world.step(self.bullet_dt, self.bullet_substeps),
                    _ => self.world.step(1.0 / 60.0, self.sub_steps),
                }
            }
        }
        fn ui(&mut self, ui: &imgui::Ui) {
            ui.window("BoxDD Testbed").build(|| {
                // Simulation controls
                if ui.button(if self.running { "Pause" } else { "Play" }) {
                    self.running = !self.running;
                }
                ui.same_line();
                if ui.button("Step") {
                    self.step_once();
                }
                ui.same_line();
                if ui.button("Reset") {
                    let _ = self.reset();
                }
                ui.separator();

                let names = [
                    "Pyramid",
                    "Bridge",
                    "Car",
                    "Revolute Motor",
                    "Prismatic Elevator",
                    "Chain Walkway",
                    "Sensors",
                    "Contacts",
                    "Bullet (Continuous)",
                    "Stacking",
                    "Shapes Variety",
                    "Robustness",
                    "Events Summary",
                    "Benchmark",
                    "Determinism",
                ];
                let mut idx = self.scene_index();
                if let Some(_c) = ui.begin_combo("Scene", names[idx]) {
                    for (i, &name) in names.iter().enumerate() {
                        let selected = i == idx;
                        if ui.selectable_config(name).selected(selected).build() {
                            idx = i;
                            self.scene = self.scene_from_index(idx);
                            let _ = self.reset();
                        }
                    }
                }
                ui.separator();
                let mut g = self.gravity_y;
                if ui.slider("Gravity Y", -30.0, 10.0, &mut g) {
                    self.gravity_y = g;
                    let _ = self.reset();
                }
                let mut ss = self.sub_steps;
                if ui.slider("Substeps", 1, 32, &mut ss) {
                    self.sub_steps = ss;
                }
                let mut run = self.running;
                if ui.checkbox("Running", &mut run) {
                    self.running = run;
                }
                let c = self.world.counters();
                ui.text(format!(
                    "Counters: bodies={} shapes={} contacts={} joints={}",
                    c.body_count, c.shape_count, c.contact_count, c.joint_count
                ));
                ui.separator();
                ui.text("Scene Params");
                match self.scene {
                    Scene::Pyramid => {
                        let mut r = self.pyramid_rows;
                        let mut c = self.pyramid_cols;
                        if ui.slider("Rows", 1, 30, &mut r) {
                            self.pyramid_rows = r;
                            let _ = self.reset();
                        }
                        if ui.slider("Cols", 1, 30, &mut c) {
                            self.pyramid_cols = c;
                            let _ = self.reset();
                        }
                    }
                    Scene::Bridge => {
                        let mut n = self.bridge_planks;
                        if ui.slider("Planks", 4, 60, &mut n) {
                            self.bridge_planks = n;
                            let _ = self.reset();
                        }
                    }
                    Scene::Car => {
                        let mut hz = self.car_hz;
                        let mut dr = self.car_dr;
                        let mut sp = self.car_motor_speed;
                        let mut tq = self.car_motor_torque;
                        if ui.slider("Spring Hz", 0.5, 20.0, &mut hz) {
                            self.car_hz = hz;
                            let _ = self.reset();
                        }
                        if ui.slider("Spring DR", 0.0, 2.0, &mut dr) {
                            self.car_dr = dr;
                            let _ = self.reset();
                        }
                        if ui.slider("Motor Speed", 0.0, 30.0, &mut sp) {
                            self.car_motor_speed = sp;
                            let _ = self.reset();
                        }
                        if ui.slider("Motor Torque", 0.0, 200.0, &mut tq) {
                            self.car_motor_torque = tq;
                            let _ = self.reset();
                        }
                    }
                    Scene::RevoluteMotor => {
                        let mut lo = self.revolute_lower_deg;
                        let mut hi = self.revolute_upper_deg;
                        let mut sp = self.revolute_speed;
                        let mut tq = self.revolute_torque;
                        if ui.slider("Lower (deg)", -180.0, 0.0, &mut lo) {
                            self.revolute_lower_deg = lo;
                            let _ = self.reset();
                        }
                        if ui.slider("Upper (deg)", 0.0, 180.0, &mut hi) {
                            self.revolute_upper_deg = hi;
                            let _ = self.reset();
                        }
                        if ui.slider("Motor Speed (rad/s)", 0.0, 10.0, &mut sp) {
                            self.revolute_speed = sp;
                            let _ = self.reset();
                        }
                        if ui.slider("Max Torque", 0.0, 200.0, &mut tq) {
                            self.revolute_torque = tq;
                            let _ = self.reset();
                        }
                    }
                    Scene::PrismaticElevator => {
                        let mut lo = self.prism_lower;
                        let mut hi = self.prism_upper;
                        let mut sp = self.prism_speed;
                        let mut f = self.prism_force;
                        if ui.slider("Lower", -5.0, 5.0, &mut lo) {
                            self.prism_lower = lo;
                            let _ = self.reset();
                        }
                        if ui.slider("Upper", 0.0, 10.0, &mut hi) {
                            self.prism_upper = hi;
                            let _ = self.reset();
                        }
                        if ui.slider("Speed (m/s)", 0.0, 10.0, &mut sp) {
                            self.prism_speed = sp;
                            let _ = self.reset();
                        }
                        if ui.slider("Max Force", 0.0, 500.0, &mut f) {
                            self.prism_force = f;
                            let _ = self.reset();
                        }
                    }
                    Scene::ChainWalkway => {
                        let mut nb = self.chain_boxes;
                        let mut amp = self.chain_amp;
                        let mut fr = self.chain_freq;
                        if ui.slider("Boxes", 0, 50, &mut nb) {
                            self.chain_boxes = nb;
                            let _ = self.reset();
                        }
                        if ui.slider("Amplitude", 0.0, 2.0, &mut amp) {
                            self.chain_amp = amp;
                            let _ = self.reset();
                        }
                        if ui.slider("Frequency", 0.1, 3.0, &mut fr) {
                            self.chain_freq = fr;
                            let _ = self.reset();
                        }
                    }
                    Scene::Sensors => {
                        let mut y = self.sensor_band_y;
                        let mut h = self.sensor_half_thickness;
                        let mut sy = self.sensor_mover_start_y;
                        let mut r = self.sensor_radius;
                        if ui.slider("Band Y", -5.0, 5.0, &mut y) {
                            self.sensor_band_y = y;
                            let _ = self.reset();
                        }
                        if ui.slider("Band Half-Height", 0.05, 1.0, &mut h) {
                            self.sensor_half_thickness = h;
                            let _ = self.reset();
                        }
                        if ui.slider("Mover Start Y", -1.0, 6.0, &mut sy) {
                            self.sensor_mover_start_y = sy;
                            let _ = self.reset();
                        }
                        if ui.slider("Mover Radius", 0.05, 1.0, &mut r) {
                            self.sensor_radius = r;
                            let _ = self.reset();
                        }
                    }
                    Scene::Contacts => {
                        let mut half = self.contact_box_half;
                        let mut sp = self.contact_speed;
                        let mut gap = self.contact_gap;
                        if ui.slider("Box Half", 0.1, 2.0, &mut half) {
                            self.contact_box_half = half;
                            let _ = self.reset();
                        }
                        if ui.slider("Speed", 0.1, 10.0, &mut sp) {
                            self.contact_speed = sp;
                            let _ = self.reset();
                        }
                        if ui.slider("Gap", 0.5, 4.0, &mut gap) {
                            self.contact_gap = gap;
                            let _ = self.reset();
                        }
                    }
                    Scene::Bullet => {
                        let mut dt = self.bullet_dt;
                        let mut sub = self.bullet_substeps;
                        let mut sp = self.bullet_speed;
                        let mut rad = self.bullet_radius;
                        let mut th = self.bullet_threshold;
                        if ui.slider("dt", 1.0 / 1000.0, 1.0 / 30.0, &mut dt) {
                            self.bullet_dt = dt.max(1e-5);
                        }
                        if ui.slider("Substeps", 1, 64, &mut sub) {
                            self.bullet_substeps = sub.max(1);
                        }
                        if ui.slider("Speed", 1.0, 120.0, &mut sp) {
                            self.bullet_speed = sp;
                            let _ = self.reset();
                        }
                        if ui.slider("Radius", 0.05, 1.0, &mut rad) {
                            self.bullet_radius = rad;
                            let _ = self.reset();
                        }
                        if ui.slider("Hit Threshold", 0.0, 2.0, &mut th) {
                            self.bullet_threshold = th;
                            let _ = self.reset();
                        }
                    }
                    _ => {}
                }
            });
        }
        fn build_scene(&mut self) {
            self.created_bodies = 0;
            self.created_shapes = 0;
            self.created_joints = 0;
            let ground = self.world.create_body_id(bd::BodyBuilder::new().build());
            self.created_bodies += 1;
            let _g = self.world.create_polygon_shape_for(
                ground,
                &bd::ShapeDef::builder().density(0.0).build(),
                &bd::shapes::box_polygon(50.0, 1.0),
            );
            self.created_shapes += 1;
            match self.scene {
                Scene::Pyramid => {
                    let columns = self.pyramid_cols.max(1) as usize;
                    let rows = self.pyramid_rows.max(1) as usize;
                    let box_poly = bd::shapes::box_polygon(0.5, 0.5);
                    let sdef = bd::ShapeDef::builder().density(1.0).build();
                    for i in 0..rows {
                        for j in 0..(columns - i) {
                            let x = (j as f32) * 1.1 - ((columns - i) as f32) * 0.55;
                            let y = 0.5 + (i as f32) * 1.05 + 2.0;
                            let b = self.world.create_body_id(
                                bd::BodyBuilder::new()
                                    .body_type(bd::BodyType::Dynamic)
                                    .position([x, y])
                                    .build(),
                            );
                            self.created_bodies += 1;
                            let _s = self.world.create_polygon_shape_for(b, &sdef, &box_poly);
                            self.created_shapes += 1;
                        }
                    }
                }
                Scene::Stacking => {
                    let cols = self.pyramid_cols.max(1) as usize;
                    let rows = self.pyramid_rows.max(1) as usize;
                    let box_poly = bd::shapes::box_polygon(0.5, 0.5);
                    let sdef = bd::ShapeDef::builder().density(1.0).build();
                    for i in 0..rows {
                        for j in 0..cols {
                            let x = -((cols as f32) * 0.55) + (j as f32) * 1.1;
                            let y = 0.5 + (i as f32) * 1.05 + 2.0;
                            let b = self.world.create_body_id(
                                bd::BodyBuilder::new()
                                    .body_type(bd::BodyType::Dynamic)
                                    .position([x, y])
                                    .build(),
                            );
                            self.created_bodies += 1;
                            let _s = self.world.create_polygon_shape_for(b, &sdef, &box_poly);
                            self.created_shapes += 1;
                        }
                    }
                }
                Scene::Bridge => {
                    let plank_count = self.bridge_planks.max(2) as usize;
                    let plank_half = bd::Vec2::new(0.5, 0.1);
                    let plank_poly = bd::shapes::box_polygon(plank_half.x, plank_half.y);
                    let sdef = bd::ShapeDef::builder().density(1.0).build();
                    let mut planks = Vec::with_capacity(plank_count);
                    let start_x = -(plank_count as f32) * plank_half.x;
                    for i in 0..plank_count {
                        let x = start_x + i as f32 * (plank_half.x * 2.2);
                        let b = self.world.create_body_id(
                            bd::BodyBuilder::new()
                                .body_type(bd::BodyType::Dynamic)
                                .position([x, 4.0])
                                .build(),
                        );
                        self.created_bodies += 1;
                        let _ = self.world.create_polygon_shape_for(b, &sdef, &plank_poly);
                        self.created_shapes += 1;
                        planks.push(b);
                    }
                    let left_anchor = bd::Vec2::new(start_x - plank_half.x, 4.0);
                    let right_anchor = bd::Vec2::new(-start_x + plank_half.x, 4.0);
                    let _ =
                        self.world
                            .create_revolute_joint_world_id(ground, planks[0], left_anchor);
                    self.created_joints += 1;
                    let _ = self.world.create_revolute_joint_world_id(
                        ground,
                        planks[plank_count - 1],
                        right_anchor,
                    );
                    self.created_joints += 1;
                    for i in 0..(plank_count - 1) {
                        let a = planks[i];
                        let b = planks[i + 1];
                        let anchor = self.world.body_position(a);
                        let base = self.world.joint_base_from_world_points(
                            a,
                            b,
                            [anchor.x + plank_half.x, anchor.y],
                            [anchor.x - plank_half.x, anchor.y],
                        );
                        let rdef = bd::RevoluteJointDef::new(base);
                        let _ = self.world.create_revolute_joint_id(&rdef);
                        self.created_joints += 1;
                    }
                }
                Scene::Car => {
                    let chassis = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, 2.0])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let sdef = bd::ShapeDef::builder().density(1.0).build();
                    let _ = self.world.create_polygon_shape_for(
                        chassis,
                        &sdef,
                        &bd::shapes::box_polygon(1.25, 0.25),
                    );
                    self.created_shapes += 1;
                    let wheel_radius = 0.4;
                    let offx = 0.8;
                    let offy = -0.3;
                    let w1 = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([-offx, 2.0 + offy])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let w2 = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([offx, 2.0 + offy])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let circle = bd::shapes::circle([0.0_f32, 0.0], wheel_radius);
                    let _ = self.world.create_circle_shape_for(w1, &sdef, &circle);
                    self.created_shapes += 1;
                    let _ = self.world.create_circle_shape_for(w2, &sdef, &circle);
                    self.created_shapes += 1;
                    let axis = [0.0_f32, 1.0];
                    let base1 = self.world.joint_base_from_world_with_axis(
                        chassis,
                        w1,
                        [-offx, 2.0 + offy],
                        [-offx, 2.0 + offy],
                        axis,
                    );
                    let wdef1 = bd::WheelJointDef::new(base1)
                        .enable_spring(true)
                        .hertz(self.car_hz)
                        .damping_ratio(self.car_dr)
                        .enable_motor(true)
                        .max_motor_torque(self.car_motor_torque * 0.5)
                        .motor_speed(0.0);
                    let _ = self.world.create_wheel_joint_id(&wdef1);
                    self.created_joints += 1;
                    let base2 = self.world.joint_base_from_world_with_axis(
                        chassis,
                        w2,
                        [offx, 2.0 + offy],
                        [offx, 2.0 + offy],
                        axis,
                    );
                    let wdef2 = bd::WheelJointDef::new(base2)
                        .enable_spring(true)
                        .hertz(self.car_hz)
                        .damping_ratio(self.car_dr)
                        .enable_motor(true)
                        .max_motor_torque(self.car_motor_torque)
                        .motor_speed(self.car_motor_speed);
                    let _ = self.world.create_wheel_joint_id(&wdef2);
                    self.created_joints += 1;
                }
                Scene::RevoluteMotor => {
                    let rotor = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, 2.0])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let _ = self.world.create_polygon_shape_for(
                        rotor,
                        &bd::ShapeDef::builder().density(1.0).build(),
                        &bd::shapes::box_polygon(1.0, 0.1),
                    );
                    self.created_shapes += 1;
                    let base = self.world.joint_base_from_world_points(
                        ground,
                        rotor,
                        [0.0_f32, 2.0],
                        [0.0_f32, 2.0],
                    );
                    let rdef = bd::RevoluteJointDef::new(base)
                        .limit_deg(self.revolute_lower_deg, self.revolute_upper_deg)
                        .enable_motor(true)
                        .max_motor_torque(self.revolute_torque)
                        .motor_speed(self.revolute_speed);
                    let _ = self.world.create_revolute_joint_id(&rdef);
                    self.created_joints += 1;
                }
                Scene::PrismaticElevator => {
                    let platform = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, 1.0])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let _ = self.world.create_polygon_shape_for(
                        platform,
                        &bd::ShapeDef::builder().density(1.0).build(),
                        &bd::shapes::box_polygon(1.0, 0.2),
                    );
                    self.created_shapes += 1;
                    let axis = [0.0_f32, 1.0];
                    let anchor = [0.0_f32, 1.0];
                    let base = self
                        .world
                        .joint_base_from_world_with_axis(ground, platform, anchor, anchor, axis);
                    let pdef = bd::PrismaticJointDef::new(base)
                        .enable_limit(true)
                        .lower_translation(self.prism_lower)
                        .upper_translation(self.prism_upper)
                        .enable_motor(true)
                        .max_motor_force(self.prism_force)
                        .motor_speed(self.prism_speed);
                    let _ = self.world.create_prismatic_joint_id(&pdef);
                    self.created_joints += 1;
                }
                Scene::ChainWalkway => {
                    let mut pts: Vec<bd::Vec2> = Vec::new();
                    for i in -20..=20 {
                        let x = i as f32 * 0.5;
                        let y = (x * self.chain_freq).sin() * self.chain_amp;
                        pts.push(bd::Vec2::new(x, y));
                    }
                    let cdef = bd::shapes::chain::ChainDef::builder()
                        .points(pts.iter().copied())
                        .is_loop(false)
                        .single_material(&bd::SurfaceMaterial::default())
                        .build();
                    let _ = self.world.create_chain_for_id(ground, &cdef);
                    let sdef = bd::ShapeDef::builder().density(1.0).build();
                    let poly = bd::shapes::box_polygon(0.2, 0.2);
                    for i in 0..self.chain_boxes.max(0) {
                        let x = -4.0 + i as f32 * 0.8;
                        let b = self.world.create_body_id(
                            bd::BodyBuilder::new()
                                .body_type(bd::BodyType::Dynamic)
                                .position([x, 3.0_f32])
                                .build(),
                        );
                        self.created_bodies += 1;
                        let _ = self.world.create_polygon_shape_for(b, &sdef, &poly);
                        self.created_shapes += 1;
                    }
                }
                Scene::Sensors => {
                    let sensor_body = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .position([0.0_f32, self.sensor_band_y])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let sensor_def = bd::ShapeDef::builder()
                        .density(0.0)
                        .sensor(true)
                        .enable_sensor_events(true)
                        .build();
                    let _ = self.world.create_polygon_shape_for(
                        sensor_body,
                        &sensor_def,
                        &bd::shapes::box_polygon(4.0, self.sensor_half_thickness),
                    );
                    self.created_shapes += 1;
                    let mover = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, self.sensor_mover_start_y])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let _ = self.world.create_circle_shape_for(
                        mover,
                        &bd::ShapeDef::builder()
                            .density(1.0)
                            .enable_sensor_events(true)
                            .build(),
                        &bd::shapes::circle([0.0_f32, 0.0], self.sensor_radius),
                    );
                    self.created_shapes += 1;
                }
                Scene::Contacts => {
                    let y1 = 2.0_f32;
                    let y2 = y1 + self.contact_gap;
                    let b1 = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, y1])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let b2 = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, y2])
                            .build(),
                    );
                    self.created_bodies += 1;
                    let sdef = bd::ShapeDef::builder()
                        .density(1.0)
                        .enable_contact_events(true)
                        .enable_hit_events(true)
                        .build();
                    let _ = self.world.create_polygon_shape_for(
                        b1,
                        &sdef,
                        &bd::shapes::box_polygon(self.contact_box_half, self.contact_box_half),
                    );
                    self.created_shapes += 1;
                    let _ = self.world.create_polygon_shape_for(
                        b2,
                        &sdef,
                        &bd::shapes::box_polygon(self.contact_box_half, self.contact_box_half),
                    );
                    self.created_shapes += 1;
                    self.world
                        .set_body_linear_velocity(b1, [0.0_f32, self.contact_speed]);
                    self.world
                        .set_body_linear_velocity(b2, [0.0_f32, -self.contact_speed]);
                }
                Scene::Bullet => {
                    self.world.enable_continuous(true);
                    self.world.set_hit_event_threshold(self.bullet_threshold);
                    let wall = self
                        .world
                        .create_body_id(bd::BodyBuilder::new().position([5.0_f32, 0.0]).build());
                    self.created_bodies += 1;
                    let _ = self.world.create_polygon_shape_for(
                        wall,
                        &bd::ShapeDef::builder()
                            .density(0.0)
                            .enable_contact_events(true)
                            .enable_hit_events(true)
                            .build(),
                        &bd::shapes::box_polygon(0.5, 3.0),
                    );
                    self.created_shapes += 1;
                    let bullet = self.world.create_body_id(
                        bd::BodyBuilder::new()
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0_f32, 0.0])
                            .bullet(true)
                            .build(),
                    );
                    self.created_bodies += 1;
                    let _ = self.world.create_circle_shape_for(
                        bullet,
                        &bd::ShapeDef::builder()
                            .density(1.0)
                            .enable_contact_events(true)
                            .enable_hit_events(true)
                            .build(),
                        &bd::shapes::circle([0.0_f32, 0.0], self.bullet_radius),
                    );
                    self.created_shapes += 1;
                    self.world
                        .set_body_linear_velocity(bullet, [self.bullet_speed, 0.0_f32]);
                }
                _ => {}
            }
        }
        fn step_once(&mut self) {
            match self.scene {
                Scene::Bullet => self.world.step(self.bullet_dt, self.bullet_substeps),
                _ => self.world.step(1.0 / 60.0, self.sub_steps),
            }
        }
        fn scene_index(&self) -> usize {
            match self.scene {
                Scene::Pyramid => 0,
                Scene::Bridge => 1,
                Scene::Car => 2,
                Scene::RevoluteMotor => 3,
                Scene::PrismaticElevator => 4,
                Scene::ChainWalkway => 5,
                Scene::Sensors => 6,
                Scene::Contacts => 7,
                Scene::Bullet => 8,
                Scene::Stacking => 9,
                Scene::Shapes => 10,
                Scene::Robustness => 11,
                Scene::Events => 12,
                Scene::Benchmark => 13,
                Scene::Determinism => 14,
            }
        }
        fn scene_from_index(&self, i: usize) -> Scene {
            match i {
                0 => Scene::Pyramid,
                1 => Scene::Bridge,
                2 => Scene::Car,
                3 => Scene::RevoluteMotor,
                4 => Scene::PrismaticElevator,
                5 => Scene::ChainWalkway,
                6 => Scene::Sensors,
                7 => Scene::Contacts,
                8 => Scene::Bullet,
                9 => Scene::Stacking,
                10 => Scene::Shapes,
                11 => Scene::Robustness,
                12 => Scene::Events,
                13 => Scene::Benchmark,
                _ => Scene::Determinism,
            }
        }
    }
}
