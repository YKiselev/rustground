use crate::application::async_runtime::ServerChannel;
use crate::error::AppError;
use crate::server::Server;
use tracing::{info, warn};
use rg_common::App;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) fn init(app: &Arc<App>, channel: ServerChannel) -> Result<JoinHandle<()>, AppError> {
    let mut server = Server::new(app, channel)?;
    server.init(app)?;
    let handle = start_server_thread(Arc::clone(app), server)?;
    Ok(handle)
}

fn start_server_thread(app: Arc<App>, mut server: Server) -> Result<JoinHandle<()>, AppError> {
    let handle = thread::Builder::new()
        .name("server-main".to_string())
        .spawn(move || {
            let mut time = Instant::now();
            let mut lag = 0u128;

            const MILLIS_PER_UPDATE: u128 = 10;

            info!("Entering server main loop...");

            while !app.is_exit() {
                let delta = time.elapsed();
                time = Instant::now();
                lag += delta.as_millis();
                while lag >= MILLIS_PER_UPDATE {
                    if let Err(e) = server.update() {
                        warn!("Server update failed: {:?}", e);
                    }
                    lag = lag.saturating_sub(MILLIS_PER_UPDATE);
                }

                let sleep = MILLIS_PER_UPDATE.saturating_sub(lag);
                thread::sleep(Duration::from_millis(sleep as _));
            }
            info!("Server loop ended.");
            server.shutdown();
            std::mem::drop(server);
        })?;
    Ok(handle)
}
