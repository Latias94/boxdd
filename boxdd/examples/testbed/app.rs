use boxdd as bd;
use dear_imgui as imgui;
use dear_imgui_glow::GlowRenderer;
use dear_imgui_winit::WinitPlatform;
use glow::HasContext as _;
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlSurface;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, NotCurrentGlContext},
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use std::{num::NonZeroU32, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowId},
};

mod debug_draw {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/testbed/debug_draw.rs"
    ));
}
mod scenes {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/testbed/scenes/mod.rs"
    ));
}
use debug_draw::ImguiDebugDraw;
use scenes::PhysicsApp;

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
    surface: glutin::surface::Surface<WindowSurface>,
    context: glutin::context::PossiblyCurrentContext,
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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::Space) => {
                    w.physics.running = !w.physics.running;
                }
                PhysicalKey::Code(KeyCode::KeyN) => {
                    w.physics.step_once();
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    let _ = w.physics.reset();
                }
                _ => {}
            },
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
        let not_current = unsafe {
            cfg.display()
                .create_context(&cfg, &ctx_attribs)
                .expect("context")
        };
        let surface = unsafe {
            let size = window.inner_size();
            let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
                window.window_handle()?.as_raw(),
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            );
            cfg.display()
                .create_window_surface(&cfg, &attrs)
                .expect("surface")
        };
        let context = not_current.make_current(&surface)?;

        // Dear ImGui setup
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
            pixels_per_meter: self.physics.pixels_per_meter,
        };
        let opts = self.physics.debug_draw_options();
        self.physics.world.debug_draw(&mut dd, opts);

        // Scene-specific overlays (drawn after debug draw so they stay on top)
        self.physics.debug_overlay(&ui);

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
