use std::sync::Arc;
use std::sync::RwLock;

use rg_common::wrap_var_bag;
use rg_common::App;
use rg_macros::VarBag;
use serde::Deserialize;
use serde::Serialize;

use crate::error::AppError;
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
            password: None
        }
    }
}

#[derive(Debug)]
pub(crate) struct Server(Arc<RwLock<ServerConfig>>, Option<ServerState>);

impl Server {
    pub fn new(app: &Arc<App>) -> Result<Self, AppError> {
        let cfg = wrap_var_bag(ServerConfig::new());
        let _ = app.vars.add("server".to_owned(), &cfg)?;
        Ok(Self(cfg, None))
    }

    pub fn init(&mut self, app: &Arc<App>) -> Result<(), AppError> {
        if self.1.is_none() {
            self.1 = Some(ServerState::new(app, &self.0)?);
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if let Some(s) = self.1.take() {
            s.shutdow();
        }
    }

    pub(crate) fn update(&mut self) -> Result<(), AppError> {
        self.1
            .as_mut()
            .map(|state| state.update())
            .unwrap_or(Ok(()))
    }
}
