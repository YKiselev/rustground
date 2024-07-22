use anyhow::anyhow;

use crate::app::App;
use crate::app_state::AppState;
use crate::client::Client;

pub struct SinglePlayerState {
    client: Client,
}

impl SinglePlayerState {
    pub(crate) fn new(app: &mut App) -> Self {
        let mut client = Client::new(app);
        SinglePlayerState {
            client
        }
    }
}

impl AppState for SinglePlayerState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        Err(anyhow!("Not implemented!"))
    }
}
