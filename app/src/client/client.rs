use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use glam::Vec3;
use rg_common::{
    App,
    gfx::world_renderer::{WorldRenderer, WorldRendererContext},
    world::HyperCube,
    wrap_var_bag,
};
use rg_vulkan::renderer::VulkanRenderer;
use tracing::{error, info};
use winit::event_loop::ActiveEventLoop;

use crate::{
    app_logger::AppLoggerBuffer,
    application::async_runtime::ClientChannel,
    client::{
        Client, WindowState, cl_actions::ClientActions, cl_commands::init_client_commands,
        cl_config::ClientConfig, cl_console::Console, cl_context::ClientContext,
        cl_fps::FrameStats, cl_game_actions::GameActions, cl_game_overlay::GameOverlay,
        cl_menu::Menu, cl_net::ClientNetwork, cl_ui_layer::UiLayer,
    },
    error::AppError,
};

impl Client {
    pub(crate) fn new(
        app: Arc<App>,
        channel: ClientChannel,
        app_log_buffer: AppLoggerBuffer,
    ) -> Result<Self, AppError> {
        info!("Starting client...");

        let cfg = wrap_var_bag(ClientConfig::new());
        app.vars.add("client", &cfg)?;

        let net = ClientNetwork::new(&app, channel)?;
        let in_game_overlay = GameOverlay::new();
        let menu = Menu::new();
        let console = Console::new(Arc::clone(&app), app_log_buffer);
        let guard = cfg.read()?;

        let mut actions = ClientActions::default();
        actions.load(&guard.bindings);
        std::mem::drop(guard);

        let game_actions = Arc::new(GameActions::default());

        let _commands = init_client_commands(Arc::clone(&app))?;

        let client = Self {
            app,
            config: cfg,
            net,
            renderer: None,
            renderer_failed: false,
            max_fps: 200.0,
            frame_stats: FrameStats::default(),
            window_state: WindowState::default(),
            hyper_cube: HyperCube::solid(),
            game_overlay: in_game_overlay,
            menu,
            console,
            actions,
            game_actions,
            _commands,
        };

        Ok(client)
    }

    pub(super) fn update(&mut self, event_loop: &ActiveEventLoop, _ctx: &ClientContext) {
        let frame_start = Instant::now();
        self.ensure_cursor();
        self.ensure_renderer(event_loop);
        self.frame_stats.add_sample();

        // Start frame
        self.net.frame_start(&self.app);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.begin_frame();
        }

        // Update
        self.net.update(&self.app);
        self.console.update();
        self.menu.update();
        self.game_overlay.update();
        self.update_renderer();

        // End frame
        self.net.frame_end(&self.app);
        let mut render_failed = false;
        if let Some(renderer) = self.renderer.as_mut() {
            render_failed = !renderer.end_frame();
        }

        if render_failed {
            self.renderer.take();
            self.renderer_failed = false;
        }
        self.cap_fps(frame_start);
    }

    fn cap_fps(&self, frame_start: Instant) {
        let target_frame_time = if self.max_fps > 0.0 {
            Duration::from_micros((1000_000.0 / self.max_fps).round() as u64)
        } else {
            Duration::ZERO
        };
        if !target_frame_time.is_zero() {
            while frame_start.elapsed() < target_frame_time {
                let time_left = target_frame_time.saturating_sub(frame_start.elapsed());
                if time_left > Duration::from_millis(2_000) {
                    std::thread::sleep(time_left - Duration::from_millis(1_500));
                } else {
                    std::hint::spin_loop();
                }
            }
        }
    }

    fn ensure_renderer(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer_failed {
            return;
        }
        if self.renderer.is_none() {
            info!("Initializing renderer...");
            self.renderer = match VulkanRenderer::new(&self.app, event_loop) {
                Ok(renderer) => Some(renderer),
                Err(e) => {
                    error!("Renderer initialization failed: {}", e);
                    self.renderer_failed = true;
                    None
                }
            }
        }
    }

    fn ensure_cursor(&mut self) {
        let should_be_captured =
            !self.console.is_visible() && !self.menu.is_visible() && self.window_state.focused;
        if self.window_state.cursor_captured != should_be_captured {
            if let Some(ref renderer) = self.renderer {
                if should_be_captured {
                    renderer.capture_mouse();
                } else {
                    renderer.release_mouse();
                }
                self.window_state.cursor_captured = should_be_captured;
            }
        }
    }

    fn update_renderer(&mut self) {
        if let Some(renderer) = self.renderer.as_mut() {
            // Draw world
            renderer.draw_world(|ctx| {
                let hc = &mut self.hyper_cube;
                hc.origin = Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };
                ctx.draw_hyper_cube(hc);

                hc.origin = Vec3 {
                    x: 20.0,
                    y: 0.0,
                    z: -2.0,
                };
                ctx.draw_hyper_cube(hc);
            });

            // Draw UI
            renderer.draw_ui(|canvas| {
                if self.game_overlay.is_visible() {
                    self.game_overlay.draw(canvas);
                }
                if self.console.is_visible() {
                    self.console.draw(canvas);
                }
                if self.menu.is_visible() {
                    self.menu.draw(canvas);
                }
            });
        }
    }

    fn toggle_menu(&mut self) {
        self.menu.toggle();
    }

    fn toggle_console(&mut self) {
        self.console.toggle();
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.renderer.take();
    }
}
