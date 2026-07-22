use std::sync::Arc;
use std::sync::RwLock;

use tracing::warn;
use rg_common::App;
use rg_common::wrap_var_bag;
use rg_macros::VarBag;
use serde::Deserialize;
use serde::Serialize;

use crate::application::async_runtime::ServerChannel;
use crate::error::AppError;
use crate::server;
use crate::server::sv_state::ServerState;

#[derive(Debug, Serialize, Deserialize, VarBag)]
pub struct ServerConfig {
    pub address: String,
    #[serde(skip_serializing)]
    pub bound_to: Option<String>,
    pub key_bits: usize,
    pub password: Option<String>,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self {
            address: "127.0.0.1:0".to_owned(),
            bound_to: None,
            key_bits: 512,
            password: None,
        }
    }
}

#[derive()]
pub(crate) struct Server {
    config: Arc<RwLock<ServerConfig>>,
    channel: ServerChannel,
    state: Option<ServerState>,
}

impl Server {
    pub fn new(app: &Arc<App>, channel: ServerChannel) -> Result<Self, AppError> {
        let cfg = wrap_var_bag(ServerConfig::new());
        let _ = app.vars.add("server", &cfg)?;
        Ok(Self {
            config: cfg,
            channel,
            state: None,
        })
    }

    pub fn init(&mut self, app: &Arc<App>) -> Result<(), AppError> {
        if self.state.is_none() {
            self.state = Some(ServerState::new(app, &self.config, self.channel.clone())?);
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if let Err(e) = self.channel.tx.send(server::Request::StopNetworkLoop) {
            warn!("Failed to send stop network loop request!");
        }

        if let Some(s) = self.state.take() {
            s.shutdow();
        }
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        if let Some(state) = self.state.as_mut() {
            state.update()
        } else {
            Ok(())
        }
    }
}
