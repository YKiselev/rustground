use std::{thread, time::Duration};

use log::info;
use rg_common::Arguments;

use crate::{app::App, app_logger, client::Client, error::AppError, server::server_init};

pub(crate) fn run_client_server(args: Arguments) -> Result<(), AppError> {
    let (handle, log_buf) = app_logger::init().expect("Unable to init app logger!");
    info!("Begin initialization...");

    let mut app = App::new(args, handle, Some(log_buf));
    //let mut state: Box<dyn AppState> = Box::new(InitialState::default());
    info!("Entering main loop...");
    let mut client = Client::new(&mut app);
    let (server, sv_handle) = server_init(&mut app).expect("Server initialization failed!");
    while !app.exit_flag() {
        client.frame_start();

        client.update(&mut app);

        client.frame_end();

        thread::sleep(Duration::from_millis(5));
    }
    sv_handle.join().expect("Unable to join server thread!");
    info!("Leaving main loop.");
    Ok(())
}
