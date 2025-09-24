use std::sync::{atomic::Ordering, Arc, RwLock};

use log::{info, warn};
use rg_common::{save_config, wrap_var_bag, App};
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    client::{cl_config::ClientConfig, cl_state::ClientState},
    error::AppError,
};

#[derive(Debug)]
pub struct Client(Arc<RwLock<ClientConfig>>, Option<ClientState>);

impl Client {
    pub(crate) fn new(app: &Arc<App>) -> Result<Self, AppError> {
        info!("Starting client...");
        let cfg = wrap_var_bag(ClientConfig::new());
        app.vars.add("client".to_owned(), &cfg)?;
        let state = ClientState::new(&app)?;
        Ok(Client(cfg, Some(state)))
    }
}

impl ApplicationHandler for Client {
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
                            if let Some(mut files) = state.app.files.lock().ok() {
                                save_config("config.toml", &mut files, toml);
                            } else {
                                warn!("Unable to lock files!");
                            }
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
