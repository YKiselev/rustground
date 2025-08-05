use std::{sync::Arc, time::Instant};

use rg_common::{App, Plugin};
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
    max_fps: f32,
    frame_time: Instant,
}

impl ClientState {
    pub(super) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        Ok(Self {
            app: Arc::clone(&app),
            net: ClientNetwork::new(app)?,
            window: ClientWindow::new(app)?,
            max_fps: 60.0,
            frame_time: Instant::now(),
        })
    }

    fn run_frame(&mut self) {
        self.net.frame_start(&self.app);

        self.net.update(&self.app);

        self.net.frame_end(&self.app);
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
            WindowEvent::RedrawRequested => if !event_loop.exiting() {
                self.run_frame();
            }
            _ => (),
        }

        self.window.window_event(event_loop, window_id, event);
    }
}
