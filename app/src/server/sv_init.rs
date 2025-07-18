use crate::app::App;
use crate::error::AppError;
use crate::server::Server;
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) fn server_init(
    app: &Arc<App>,
) -> Result<(Arc<Mutex<Server>>, JoinHandle<()>), AppError> {
    let server = Arc::new(Mutex::new(Server::new(app)?));
    let sv_clone = server.clone();
    let app_clone = app.clone();
    let handle = thread::Builder::new()
        .name("server-thread".to_string())
        .spawn(move || {
            let mut time = Instant::now();
            let mut lag = 0u128;
            const MILLIS_PER_UPDATE: u128 = 10;
            info!("Entering server loop...");
            while !app_clone.is_exit() {
                let delta = time.elapsed();
                time = Instant::now();
                lag += delta.as_millis();
                while lag >= MILLIS_PER_UPDATE {
                    match sv_clone.lock() {
                        Ok(mut srv) => {
                            if let Err(e) = srv.update() {
                                warn!("Server update failed: {:?}", e);
                            }
                            lag -= MILLIS_PER_UPDATE;
                        }
                        Err(e) => {
                            error!("Failed to update server: {e:?}");
                        }
                    }
                }
                let sleep = MILLIS_PER_UPDATE.saturating_sub(lag);
                if sleep > 0 {
                    thread::sleep(Duration::from_millis(sleep as _));
                }
            }
            info!("Server loop ended.");
        })?;
    Ok((server, handle))
}
