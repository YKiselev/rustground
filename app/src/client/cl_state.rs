use std::{
    rc::Rc,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{
    app_logger::AppLoggerBuffer,
    application::async_runtime::ClientChannel,
    client::{
        cl_config::ClientConfig, cl_console::Console, cl_fps::FrameStats,
        cl_game_actions::GameActions, cl_game_overlay::GameOverlay, cl_menu::Menu,
        cl_net::ClientNetwork, cl_ui_layer::UiLayer,
    },
    error::AppError,
};
use bitflags::bitflags;
use glam::Vec3;
use rg_common::{
    App, Plugin,
    gfx::world_renderer::{WorldRenderer, WorldRendererContext},
    ui::{
        canvas::{Canvas, WrapMode},
        color::Color,
    },
    world::HyperCube,
};
use rg_vulkan::renderer::VulkanRenderer;
use tracing::{error, info};
use winit::{
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::WindowId,
};

#[derive(Debug, Default)]
struct WindowState {
    modifiers: ModifiersState,
    focused: bool,
    cursor_captured: bool,
}

pub(super) struct ClientState {
    pub app: Arc<App>,
    config: Arc<RwLock<ClientConfig>>,
    net: ClientNetwork,
    renderer: Option<VulkanRenderer>,
    renderer_failed: bool,
    max_fps: f32,
    frame_stats: FrameStats,
    window_state: WindowState,
    hyper_cube: HyperCube,
    game_overlay: GameOverlay,
    menu: Menu,
    console: Console,
    game_actions: GameActions,
}

impl ClientState {
    pub(super) fn new(
        app: &Arc<App>,
        config: &Arc<RwLock<ClientConfig>>,
        channel: ClientChannel,
        app_log_buffer: AppLoggerBuffer,
    ) -> Result<Self, AppError> {
        let net = ClientNetwork::new(app, channel)?;
        let in_game_overlay = GameOverlay::new();
        let menu = Menu::new();
        let console = Console::new(app_log_buffer);
        let guard = config.read()?;
        let game_actions = GameActions::new(&guard.bindings);
        Ok(Self {
            app: Arc::clone(&app),
            config: Arc::clone(&config),
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
            game_actions,
        })
    }

    pub fn destroy(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            std::mem::drop(renderer);
        }
    }

    fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
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

    fn cap_fps(&self, frame_start: Instant) {
        let target_frame_time = if self.max_fps > 0.0 {
            Duration::from_micros((1000_000.0 / self.max_fps).round() as u64)
        } else {
            Duration::ZERO
        };
        if !target_frame_time.is_zero() {
            while frame_start.elapsed() < target_frame_time {
                let time_left = target_frame_time.saturating_sub(frame_start.elapsed());
                if time_left > Duration::from_micros(2_000_000) {
                    std::thread::sleep(time_left - Duration::from_micros(1_500));
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

    pub(super) fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.renderer.take();
        match VulkanRenderer::new(&self.app, event_loop) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => error!("Unable to create Vulkan renderer: {:?}", e),
        }
        event_loop.listen_device_events(winit::event_loop::DeviceEvents::Always);
    }

    pub(super) fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.mark_resized();
                }
            }
            WindowEvent::Focused(focused) => {
                self.window_state.focused = focused;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                info!("Window={window_id:?} changed scale to {scale_factor}");
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.window_state.modifiers = modifiers.state();
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    info!("Mouse wheel Line Delta: ({x},{y})");
                }
                MouseScrollDelta::PixelDelta(px) => {
                    info!("Mouse wheel Pixel Delta: ({},{})", px.x, px.y);
                }
            },
            WindowEvent::RedrawRequested => {
                if !event_loop.exiting() {
                    self.run_frame(event_loop);
                }
            }
            WindowEvent::KeyboardInput { ref event, .. } => {
                self.dispatch_key_event(event);
            }
            _ => (),
        }
    }

    pub(super) fn device_event(&mut self, event: DeviceEvent, event_loop: &ActiveEventLoop) {
        if self.menu.device_event(event_loop, &event) {
            return;
        }

        if self.console.device_event(event_loop, &event) {
            return;
        }

        if self.game_overlay.device_event(event_loop, &event) {
            return;
        }

        self.update_game_actions(&event);

        self.device_event_fallback(event, event_loop);
    }

    fn device_event_fallback(&mut self, event: DeviceEvent, _: &ActiveEventLoop) {
        match event {
            DeviceEvent::Key(raw_key_event) => {
                // Process only key release actions
                if raw_key_event.state == ElementState::Released {
                    match raw_key_event.physical_key {
                        PhysicalKey::Code(key_code) => match key_code {
                            KeyCode::Escape => self.toggle_menu(),
                            KeyCode::Backquote => self.toggle_console(),
                            _ => {}
                        },
                        PhysicalKey::Unidentified(_) => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn dispatch_key_event(&mut self, event: &KeyEvent) {
        if self.menu.keyboard_input(event) {
            return;
        }

        if self.console.keyboard_input(event) {
            return;
        }

        match event.physical_key {
            PhysicalKey::Code(ref key_code) => {
                if *key_code == KeyCode::Space {
                    info!("fps: {:.2}", self.frame_stats.calc_fps());
                }
            }
            _ => {}
        };
    }

    fn toggle_menu(&mut self) {
        self.menu.toggle();
    }

    fn toggle_console(&mut self) {
        self.console.toggle();
    }

    fn update_game_actions(&mut self, event: &DeviceEvent) {
        match event {
            DeviceEvent::Key(raw_key_event) => {
                info!("{:?}", raw_key_event);
                self.game_actions
                    .update_from_key(raw_key_event.physical_key, raw_key_event.state)
            }
            DeviceEvent::Button { button, state } => {
                self.game_actions.update_from_button(*button, *state)
            }
            _ => {}
        }
    }
}
