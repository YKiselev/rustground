use std::{sync::Arc, thread, time::Duration};

use log::{debug, info, warn};
use rg_common::Arguments;

use crate::{
    app_logger, application::app_host::AppHost, client::Client, error::AppError,
    server::server_init,
};

pub(crate) fn run_client_server(args: Arguments) -> Result<(), AppError> {
    let (handle, log_buf) = app_logger::init(&args)?;
    info!("=== App started ===");

    let host = AppHost::new(args);
    let app = host.app.clone();
    let (server, sv_handle) = server_init(&app)?;
    let mut client = Client::new(&app)?;
    info!("Entering main loop...");
    while !app.is_exit() {
        client.frame_start();

        client.update(&host.app);

        client.frame_end();

        thread::sleep(Duration::from_millis(5));
    }
    server.lock()?.shutdown();
    debug!("Joining sv thread...");
    let _ = sv_handle
        .join()
        .inspect_err(|e| warn!("Failed to join server thread: {:?}", e));
    info!("Bye");
    Ok(())
}
