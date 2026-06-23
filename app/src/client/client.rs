use std::sync::{Arc, RwLock, atomic::Ordering};

use log::{info, warn};
use rg_common::{App, save_config, wrap_var_bag};
use winit::{
    application::ApplicationHandler,
    event::{Event::UserEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    client::{cl_config::ClientConfig, cl_state::ClientState},
    error::AppError,
};

pub struct ClientEvent();

pub struct Client(Arc<RwLock<ClientConfig>>, Option<ClientState>);

impl Client {
    pub(crate) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting client...");
        let cfg = wrap_var_bag(ClientConfig::new());
        app.vars.add("client", &cfg)?;
        let state = ClientState::new(&app)?;
        Ok(Client(cfg, Some(state)))
    }
}

impl ApplicationHandler<ClientEvent> for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.1.as_mut() {
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
                info!("Window close requested");
                if let Some(mut state) = self.1.take() {
                    match state.app.vars.to_toml() {
                        Ok(toml) => {
                            save_config("config.toml", &state.app.files, toml);
                        }
                        Err(e) => {
                            warn!("Unable to export vars to toml: {:?}", e);
                        }
                    }
                    state.app.exit_flag.store(true, Ordering::Relaxed);
                    state.destroy();
                }
                event_loop.exit();
            }
            _ => (),
        }
        if let Some(state) = self.1.as_mut() {
            state.window_event(event_loop, window_id, event);
        }
    }
}

