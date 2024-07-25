use anyhow::anyhow;

use crate::app::App;
use crate::state::app_state::AppState;

pub struct MultiPlayerState {}

impl AppState for MultiPlayerState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        Err(anyhow!("Not implemented!"))
    }
}
