use std::thread;
use std::time::Duration;

use crate::app::App;
use crate::client::Client;
use crate::server::server_init;
use crate::state::app_state::AppState;

pub struct SinglePlayerState;

impl SinglePlayerState {
    pub(crate) fn new(app: &mut App) -> Self {
        SinglePlayerState {}
    }
}

impl AppState for SinglePlayerState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>> {
        let mut client = Client::new(app);
        let (server, sv_handle) = server_init(app).expect("Server initialization failed!");
        while !app.exit_flag() {
            client.frame_start();

            client.update(app);

            client.frame_end();

            thread::sleep(Duration::from_millis(5));
        }
        sv_handle.join().expect("Unable to join server thread!");
        Ok(None)
    }
}
