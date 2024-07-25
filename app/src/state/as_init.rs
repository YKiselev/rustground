use crate::app::App;
use crate::state::app_state::AppState;
use crate::state::DedicatedServerState;
use crate::state::MultiPlayerState;
use crate::state::SinglePlayerState;

#[derive(Default)]
pub struct InitialState;


impl AppState for InitialState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        Ok(Some(
            if app.args().dedicated() {
                Box::new(DedicatedServerState {})
            } else {
                let multiplayer = false;
                if multiplayer {
                    Box::new(MultiPlayerState {})
                } else {
                    Box::new(SinglePlayerState::new(app))
                }
            }
        ))
    }
}
