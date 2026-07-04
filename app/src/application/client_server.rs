use log::{debug, info, warn};
use rg_common::Arguments;
use std::sync::Arc;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{
    app_logger,
    application::app_host::AppHost,
    client::{Client, ClientEvent},
    error::AppError,
    server,
};

pub(crate) fn run_client_server(args: Arguments) -> Result<(), AppError> {
    #[allow(unused_variables)]
    let (handle, log_buf) = app_logger::init(&args)?;

    info!("========= Starting =========");

    let host = AppHost::new(args);
    let app = Arc::clone(&host.app);
    app.load_config("config.toml");

    let (server, sv_handle) = server::init(&app)?;
    let event_loop = EventLoop::<ClientEvent>::with_user_event().build()?;
    //let proxy = event_loop.create_proxy();
    //proxy.send_event(ClientEvent::new());
    
    let mut client = Client::new(&app)?;
    event_loop.set_control_flow(ControlFlow::Poll);

    info!("Entering main loop...");
    event_loop.run_app(&mut client)?;

    server.lock()?.shutdown();
    debug!("Joining server thread...");
    let _ = sv_handle
        .join()
        .inspect_err(|e| warn!("Failed to join server thread: {:?}", e));
    info!("Bye");
    Ok(())
}
