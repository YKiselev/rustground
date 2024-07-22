use std::sync::{Arc, RwLock};

use crate::app::App;
use crate::app_state::AppState;
use crate::as_dedicated::DedicatedServerState;
use crate::as_multi::MultiPlayerState;
use crate::as_single::SinglePlayerState;
use crate::client::Client;
use crate::server::Server;

#[derive(Default)]
pub struct InitialState {
    server: Option<Arc<RwLock<Server>>>,
    client: Option<Client>,
}


impl AppState for InitialState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        let dedicated = false;
        if (dedicated) {
            return Ok(Some(Box::new(DedicatedServerState {})));
        }
        let multiplayer = false;
        if (multiplayer) {
            return Ok(Some(Box::new(MultiPlayerState {})));
        }
        Ok(Some(Box::new(SinglePlayerState::new(app))))
    }
}
