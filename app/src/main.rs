use std::io::Read;
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use log::{error, info, LevelFilter, warn};
use rsa::signature::digest::Digest;
use serde::{Deserialize, Serialize};
use serde::de::StdError;

use core::arguments::Arguments;
use core::files;
use core::services::Services;

use crate::client::Client;
use crate::config::{Config, ServerConfig};
use crate::net::{ConnectData, Message};
use crate::server::Server;

mod client;
mod server;
mod app_logger;
mod net;
mod config;

fn server_init(cfg: &ServerConfig) -> anyhow::Result<(Arc<RwLock<Server>>, JoinHandle<()>)> {
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

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut files = files::Files::new(&args);

    let cfg = Config::load("config.toml", &mut files);
    info!("Loaded config: {cfg:?}");

    // server
    let (server, sv_handle) = server_init(&cfg.server)?;

    // debug
    let server_addr = server.read().unwrap().local_address()?;//.expect("Unable to get server address");

    // client
    let mut client = if !args.dedicated() {
        Some(Client::new(&args, server_addr))
    } else {
        None
    };

    // main loop
    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let started_at = Instant::now();
    while !exit_flag.load(Ordering::Acquire) {
        if let Some(ref mut cl) = client {
            cl.frame_start();
        }

        if let Some(ref mut cl) = client.as_mut() {
            cl.update();
        }
        thread::sleep(Duration::from_millis(5));
        //logger_buf.update();
        if started_at.elapsed() > Duration::from_secs(7) {
            exit_flag.store(true, Ordering::Release);
        }
        if let Some(ref mut cl) = client {
            cl.frame_end();
        }
    }
    server.write().unwrap().shutdown();
    sv_handle.join().expect("Unable to join server thread!");
    Ok(())
}