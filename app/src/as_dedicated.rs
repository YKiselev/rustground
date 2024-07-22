use std::sync::{Arc, RwLock};

use anyhow::anyhow;

use crate::app::App;
use crate::app_state::AppState;
use crate::server::Server;

pub struct DedicatedServerState {
}

impl AppState for DedicatedServerState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        Err(anyhow!("Not implemented!"))
    }
}
