use std::sync::{atomic::Ordering, Arc};

use log::info;
use rg_common::App;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    client::cl_state::ClientState,
    error::AppError,
};

#[derive(Debug)]
pub struct Client {
    app: Arc<App>,
    state: Option<ClientState>,
}

impl Client {
    pub(crate) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting client...");
        //let _ = app.config().lock()?;
        Ok(Client {
            app: Arc::clone(&app),
            state: Some(ClientState::new(&app)?),
        })
    }
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            state.resumed(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.app.exit_flag.store(true, Ordering::Relaxed);
                if let Some(mut state) = self.state.take() {
                    state.destroy();
                }
                event_loop.exit();
            }
            _ => (),
        }
        if let Some(state) = self.state.as_mut() {
            state.window_event(event_loop, window_id, event);
        }
    }
}
