use std::{
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use rg_common::{App, commands::CommandOwner, save_config, wrap_var_bag};
use tracing::{info, warn};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    app_logger::AppLoggerBuffer,
    application::{async_runtime::ClientChannel, trigger_exit},
    client::{
        cl_commands::{ClientCommand, init_client_commands},
        cl_config::ClientConfig,
        cl_state::ClientState,
    },
    error::AppError,
};

pub enum ClientEvent {
    Exiting,
}

pub struct Client {
    app: Arc<App>,
    config: Arc<RwLock<ClientConfig>>,
    state: ClientState,
    _commands: CommandOwner,
}

impl Client {
    pub(crate) fn new(
        app: Arc<App>,
        channel: ClientChannel,
        app_log_buffer: AppLoggerBuffer,
    ) -> Result<Self, AppError> {
        info!("Starting client...");

        let cfg = wrap_var_bag(ClientConfig::new());
        app.vars.add("client", &cfg)?;

        let state = ClientState::new(&app, &cfg, channel, app_log_buffer)?;

        let _commands = init_client_commands(Arc::clone(&app))?;

        let client = Self {
            app,
            config: cfg,
            state,
            _commands,
        };

        Ok(client)
    }

    fn on_cl_restart(&mut self) {
        // todo get current state
        //let Some(ClientState { app, config, console, .. }) = self.state.take();

        //let state = ClientState::new(&app, &cfg, self.channel.clone(), app_log_buffer)?;
        //self.state = Some(state);
        // restore state
    }
}

impl ApplicationHandler<ClientEvent> for Client {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state.resumed(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ClientEvent) {
        let _ = (event_loop, event);
        match event {
            ClientEvent::Exiting => {
                event_loop.exit();
            }
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.state.device_event(event, event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        //if let Some(mut state) = self.state.take() {
            match self.state.app.vars.to_toml() {
                Ok(toml) => {
                    save_config("config.toml", &self.state.app.files, toml);
                }
                Err(e) => {
                    warn!("Unable to export vars to toml: {:?}", e);
                }
            }
            self.state.destroy();
        //}
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
                trigger_exit();
            }
            _ => (),
        }

        let mut restart = false;

        // for command in self.command_rx.try_iter() {
        //     match command {
        //         ClientCommand::Restart => restart = true,
        //         _ => {
        //             if let Some(state) = self.state.as_mut() {
        //                 state.on_command(command);
        //             }
        //         }
        //     }
        // }

        if restart {
            self.on_cl_restart();
        }

        //if let Some(state) = self.state.as_mut() {
            self.state.window_event(event_loop, window_id, event);
        //}
    }
}
