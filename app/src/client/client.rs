use std::sync::{Arc, RwLock, atomic::Ordering};

use rg_common::{App, save_config, wrap_var_bag};
use tracing::{info, warn};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    application::async_runtime::ClientChannel,
    client::{cl_config::ClientConfig, cl_state::ClientState},
    error::AppError,
};

pub struct ClientEvent();

pub struct Client {
    config: Arc<RwLock<ClientConfig>>,
    channel: ClientChannel,
    state: Option<ClientState>,
}

impl Client {
    pub(crate) fn new(app: &Arc<App>, channel: ClientChannel) -> Result<Self, AppError> {
        info!("Starting client...");
        let cfg = wrap_var_bag(ClientConfig::new());
        app.vars.add("client", &cfg)?;
        let state = ClientState::new(&app, &cfg, channel.clone())?;
        Ok(Client {
            config: cfg,
            channel,
            state: Some(state),
        })
    }
}

impl ApplicationHandler<ClientEvent> for Client {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            state.resumed(event_loop);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ClientEvent) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
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
                if let Some(mut state) = self.state.take() {
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
        if let Some(state) = self.state.as_mut() {
            state.window_event(event_loop, window_id, event);
        }
    }
}
