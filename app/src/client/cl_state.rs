use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use glam::Vec3;
use log::{debug, error, info};
use rg_common::{
    App, Plugin,
    gfx::world_renderer::{WorldRenderer, WorldRendererContext},
    world::HyperCube,
};
use rg_vulkan::renderer::VulkanRenderer;
use winit::{
    event::{Event, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::WindowId,
};

use crate::{
    application::async_runtime::ClientChannel, client::{cl_config::ClientConfig, cl_fps::FrameStats, cl_net::ClientNetwork}, error::AppError,
};

pub(super) struct ClientState {
    pub app: Arc<App>,
    config: Arc<RwLock<ClientConfig>>,
    net: ClientNetwork,
    renderer: Option<VulkanRenderer>,
    renderer_failed: bool,
    max_fps: f32,
    frame_stats: FrameStats,
    modifiers: ModifiersState,
    hyper_cube: HyperCube,
}

impl ClientState {
    pub(super) fn new(
        app: &Arc<App>,
        config: &Arc<RwLock<ClientConfig>>,
        channel: ClientChannel
    ) -> Result<Self, AppError> {
        let net = ClientNetwork::new(app, channel)?;
        Ok(Self {
            app: Arc::clone(&app),
            config: Arc::clone(&config),
            net,
            renderer: None,
            renderer_failed: false,
            max_fps: 200.0,
            frame_stats: FrameStats::default(),
            modifiers: ModifiersState::default(),
            hyper_cube: HyperCube::solid(),
        })
    }

    pub fn destroy(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            std::mem::drop(renderer);
        }
    }

    fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let frame_start = Instant::now();
        self.ensure_renderer(event_loop);
        self.frame_stats.add_sample();

        // Start frame
        self.net.frame_start(&self.app);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.begin_frame();
        }

        // Update
        self.net.update(&self.app);
        if let Some(renderer) = self.renderer.as_mut() {
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
            renderer.render();
        }

        // End frame
        self.net.frame_end(&self.app);
        let mut render_failed = false;
        if let Some(renderer) = self.renderer.as_mut() {
            render_failed = !renderer.end_frame();
        }

        if render_failed {
            self.renderer.take();
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

    pub(super) fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.renderer.take();
        match VulkanRenderer::new(&self.app, event_loop) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => error!("Unable to create Vulkan renderer: {:?}", e),
        }
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
                if focused {
                    info!("Window={window_id:?} focused");
                } else {
                    info!("Window={window_id:?} unfocused");
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                info!("Window={window_id:?} changed scale to {scale_factor}");
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
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
            WindowEvent::KeyboardInput {
                ref event,
                is_synthetic: false,
                ..
            } => match event.physical_key {
                PhysicalKey::Code(ref key_code) => {
                    if *key_code == KeyCode::Space {
                        info!("fps: {:.2}", self.frame_stats.calc_fps());
                    }
                }
                PhysicalKey::Unidentified(_) => {}
            },
            _ => (),
        }
    }
}
