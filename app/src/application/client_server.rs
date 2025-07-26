use std::{sync::Arc, thread, time::Duration};

use log::{info, warn};
use rg_common::Arguments;

use crate::{app::App, app_logger, client::Client, error::AppError, server::server_init};

pub(crate) fn run_client_server(args: Arguments) -> Result<(), AppError> {
    let (handle, log_buf) = app_logger::init(&args)?;
    info!("=== App started ===");

    let app = Arc::new(App::new(args));
    let (server, sv_handle) = server_init(&app)?;
    let mut client = Client::new(&app)?;
    info!("Entering main loop...");
    while !app.is_exit() {
        client.frame_start();

        client.update(&app);

        client.frame_end();

        thread::sleep(Duration::from_millis(5));
    }
    server.lock()?.shutdown();
    let _ = sv_handle
        .join()
        .inspect_err(|e| warn!("Failed to join server thread: {:?}", e));
    info!("Leaving main loop.");
    Ok(())
}
