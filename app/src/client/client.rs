use std::sync::{atomic::Ordering, Arc};

use log::info;
use rg_common::{App, Plugin};
use winit::{application::ApplicationHandler, event::WindowEvent};

use crate::{
    client::{cl_net::ClientNetwork, cl_window::ClientWindow},
    error::AppError,
};

#[derive(Debug)]
pub struct Client {
    app: Arc<App>,
    net: ClientNetwork,
    window: ClientWindow,
}

impl Client {
    pub(crate) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting client...");
        //let _ = app.config().lock()?;
        Ok(Client {
            app: Arc::clone(&app),
            net: ClientNetwork::new(app)?,
            window: ClientWindow::new(app)?,
        })
    }

    fn run_frame(&mut self) {
        self.net.frame_start(&self.app);

        self.net.update(&self.app);

        self.net.frame_end(&self.app);
    }
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.app.exit_flag.store(true, Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.run_frame();
            }
            _ => (),
        }

        self.window.window_event(event_loop, window_id, event);
    }
}
