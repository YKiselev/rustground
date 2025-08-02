use std::{thread, time::Duration};

use log::{debug, info, warn};
use rg_common::{Arguments, Plugin};
use winit::event_loop::{ControlFlow, EventLoop};

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
    let event_loop = EventLoop::new()?;
    let mut client = Client::new(&app)?;
    event_loop.set_control_flow(ControlFlow::Poll);

    info!("Entering main loop...");
    event_loop.run_app(&mut client)?;
    // while !app.is_exit() {
    //     client.frame_start(&host.app);

    //     client.update(&host.app);

    //     client.frame_end(&host.app);

    //     thread::sleep(Duration::from_millis(5));
    // }
    server.lock()?.shutdown();
    debug!("Joining sv thread...");
    let _ = sv_handle
        .join()
        .inspect_err(|e| warn!("Failed to join server thread: {:?}", e));
    info!("Bye");
    Ok(())
}