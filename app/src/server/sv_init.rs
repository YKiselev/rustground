use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use log::{info, warn};
use crate::config::ServerConfig;
use crate::server::Server;

pub(crate) fn server_init(cfg: &ServerConfig) -> anyhow::Result<(Arc<RwLock<Server>>, JoinHandle<()>)> {
    let server = Arc::new(RwLock::new(Server::new(cfg)));
    let sv_clone = server.clone();
    let handle = thread::Builder::new()
        .name("server-thread".to_string())
        .spawn(move || {
            let mut time = Instant::now();
            let mut lag = 0;
            const MILLIS_PER_UPDATE: u128 = 10;
            info!("Entering server loop...");
            while !sv_clone.read().unwrap().is_exit() {
                let delta = time.elapsed();
                time = Instant::now();
                lag += delta.as_millis();
                let mut m = 0;
                while lag >= MILLIS_PER_UPDATE {
                    if let Err(e) = sv_clone.write().unwrap().update() {
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
        }).map_err(|e| anyhow::Error::from(e))?;
    Ok((server, handle))
}