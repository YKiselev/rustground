use std::sync::Arc;

use log::{error, info};
use rg_common::{App, Plugin};
use rg_vulkan::renderer::VulkanRenderer;
use winit::{
    application::ApplicationHandler,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    client::{cl_net::ClientNetwork, cl_window::create_window_attributes},
    error::AppError,
    fps::FrameStats,
};

pub(super) struct ClientState {
    pub app: Arc<App>,
    net: ClientNetwork,
    window: Option<Window>,
    renderer: Option<VulkanRenderer>,
    renderer_failed: bool,
    max_fps: f32,
    frame_stats: FrameStats,
    modifiers: ModifiersState,
}

impl ClientState {
    pub(super) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        let net = ClientNetwork::new(app)?;
        Ok(Self {
            app: Arc::clone(&app),
            net,
            window: None,
            renderer: None,
            renderer_failed: false,
            max_fps: 60.0,
            frame_stats: FrameStats::default(),
            modifiers: ModifiersState::default(),
        })
    }

    pub fn destroy(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            std::mem::drop(renderer);
        }
    }

    fn run_frame(&mut self) {
        self.ensure_renderer();

        self.frame_stats.add_sample();
        self.net.frame_start(&self.app);

        self.net.update(&self.app);

        if let (Some(renderer), Some(window)) = (self.renderer.as_mut(), self.window.as_ref()) {
            renderer.render(window);
            window.request_redraw();
        }

        self.net.frame_end(&self.app);
    }

    fn ensure_renderer(&mut self) {
        if self.renderer_failed || self.window.is_none() {
            return;
        }
        if self.renderer.is_none() {
            if let Some(window) = self.window.as_ref() {
                info!("Initializing renderer...");
                self.renderer = match VulkanRenderer::new(&self.app, window) {
                    Ok(renderer) => Some(renderer),
                    Err(e) => {
                        error!("Renderer initialization failed: {}", e);
                        self.renderer_failed = true;
                        None
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for ClientState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = create_window_attributes(event_loop);
        self.window = match event_loop.create_window(attrs) {
            Ok(wnd) => Some(wnd),
            Err(e) => {
                error!("Unable to create window: {e:?}");
                None
            }
        };
    }

    fn window_event(
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
            WindowEvent::ActivationTokenDone { token: _token, .. } => {
                #[cfg(any(x11_platform, wayland_platform))]
                {
                    startup_notify::set_activation_token_env(_token);
                    if let Err(err) = self.create_window(event_loop, None) {
                        error!("Error creating new window: {err}");
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if !event_loop.exiting() {
                    self.run_frame();
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
