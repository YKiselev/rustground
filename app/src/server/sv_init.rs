use crate::app::App;
use crate::error::AppError;
use crate::server::Server;
use log::{info, warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) fn server_init(
    app: &Arc<App>,
) -> Result<(Arc<Mutex<Server>>, JoinHandle<()>), AppError> {
    let server = Arc::new(Mutex::new(Server::new(app)));
    let sv_clone = server.clone();
    let app_clone = app.clone();
    let handle = thread::Builder::new()
        .name("server-thread".to_string())
        .spawn(move || {
            let mut time = Instant::now();
            let mut lag = 0;
            const MILLIS_PER_UPDATE: u128 = 10;
            info!("Entering server loop...");
            while !app_clone.exit_flag() {
                let delta = time.elapsed();
                time = Instant::now();
                lag += delta.as_millis();
                let mut m = 0;
                while lag >= MILLIS_PER_UPDATE {
                    if let Err(e) = sv_clone.lock().unwrap().update() {
                        warn!("Server update failed: {:?}", e);
                    }
                    lag -= MILLIS_PER_UPDATE;
                    m += 1;
                }
                if m == 0 {
                    thread::sleep(Duration::from_millis((MILLIS_PER_UPDATE - lag) as u64));
                }
            }
            info!("Server loop ended.");
        })?;
    Ok((server, handle))
}
