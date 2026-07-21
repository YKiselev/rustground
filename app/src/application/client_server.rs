use log::{debug, info, warn};
use rg_common::Arguments;
use std::sync::Arc;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{
    app_logger,
    application::{app_host::AppHost, async_runtime::init_client_server_async_runtime},
    client::{Client, ClientEvent},
    error::AppError,
    server,
};

pub fn run_client_server(args: Arguments) -> Result<(), AppError> {
    #[allow(unused_variables)]
    let (handle, log_buf) = app_logger::init(&args)?;

    info!("========= Starting =========");

    let host = AppHost::new(args);
    let app = Arc::clone(&host.app);
    app.load_config("config.toml");

    let (async_handle, server_channel, client_channel) = init_client_server_async_runtime()?;

    let sv_handle = server::init(&app, server_channel)?;
    let event_loop = EventLoop::<ClientEvent>::with_user_event().build()?;
    //let proxy = event_loop.create_proxy();
    //proxy.send_event(ClientEvent::new());

    let mut client = Client::new(&app, client_channel)?;
    event_loop.set_control_flow(ControlFlow::Poll);

    info!("Entering main loop...");
    event_loop.run_app(&mut client)?;

    debug!("Joining server thread...");
    let _ = sv_handle
        .join()
        .inspect_err(|e| warn!("Failed to join server thread: {:?}", e));

    std::mem::drop(client);

    let _ = async_handle
        .join()
        .inspect_err(|e| warn!("Failed to join async runtime thread: {:?}", e));

    info!("Bye");
    Ok(())
}
