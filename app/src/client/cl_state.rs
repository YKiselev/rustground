use std::{sync::Arc, time::Instant};

use log::{error, info};
use rg_common::{App, Plugin};
use rg_vulkan::renderer::VulkanRenderer;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    client::{cl_net::ClientNetwork, cl_window::ClientWindow},
    error::AppError,
};

#[derive(Debug)]
pub(super) struct ClientState {
    app: Arc<App>,
    net: ClientNetwork,
    window: ClientWindow,
    renderer: Option<VulkanRenderer>,
    renderer_failed: bool,
    max_fps: f32,
    frame_time: Instant,
}

impl ClientState {
    pub(super) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        let net = ClientNetwork::new(app)?;
        let window = ClientWindow::new(app)?;
        Ok(Self {
            app: Arc::clone(&app),
            net,
            window,
            renderer: None,
            renderer_failed: false,
            max_fps: 60.0,
            frame_time: Instant::now(),
        })
    }

    fn run_frame(&mut self) {
        self.net.frame_start(&self.app);

        self.net.update(&self.app);

        if let (Some(renderer), Some(window)) =
            (self.renderer.as_mut(), self.window.window.as_ref())
        {
            renderer.render(window);
        }

        self.net.frame_end(&self.app);
    }

    fn ensure_renderer(&mut self) {
        if self.renderer_failed {
            return;
        }
        if self.renderer.is_none() {
            if let Some(window) = self.window.window.as_ref() {
                info!("Initializing renderer...");
                match VulkanRenderer::new(window) {
                    Ok(renderer) => self.renderer = Some(renderer),
                    Err(e) => {
                        error!("Renderer initialization failed: {}", e);
                        self.renderer_failed = true;
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for ClientState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window.resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => {
                if let (Some(renderer)) = self.renderer.as_mut() {
                    renderer.mark_resized();
                }
            }
            WindowEvent::RedrawRequested => {
                if !event_loop.exiting() {
                    self.ensure_renderer();
                    if self.renderer.is_some() {
                        self.run_frame();
                    }
                }
            }
            _ => (),
        }

        self.window.window_event(event_loop, window_id, event);
    }
}
