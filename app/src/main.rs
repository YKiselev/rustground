use std::io::Read;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use log::{error, info, LevelFilter};
use serde::{Deserialize, Serialize};

use core::arguments::Arguments;
use core::files;
use core::services::Services;

use crate::client::Client;
use crate::config::Config;
use crate::net::{ConnectData, Message};
use crate::server::Server;

mod client;
mod server;
mod app_logger;
mod net;
mod config;

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut files = files::Files::new(&args);
    let mut cfg = files.open("config.toml").expect("Unable to load config!");
    let mut tmp = String::new();
    let read = cfg.read_to_string(&mut tmp)?;
    let cfg: Config = toml::from_str(&tmp)?;
    info!("Loaded config: {cfg:?}");

    // init services
    //let services = Services::new(&args);

    // server
    let mut server = Server::new(&cfg.server);

    // debug
    let server_addr = server.local_address().expect("Unable to get server address");

    // client
    let mut client = if !args.dedicated() {
        Some(Client::new(&args, server_addr))
    } else {
        None
    };

    // main loop
    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let mut i = 0;
    let mut time = Instant::now();
    let mut lag = 0u128;
    const NANOS_PER_UPDATE: u128 = 20_000_000;
    while !exit_flag.load(Ordering::Acquire) {
        let delta = time.elapsed().as_nanos();
        time = Instant::now();
        lag += delta;
        info!("lag={lag} ns., delta={delta} ns.");
        let mut m = 0;
        while lag >= NANOS_PER_UPDATE {
            server.update()?;
            lag -= NANOS_PER_UPDATE;
            m+=1;
        }
        info!("Updated {m} times to eliminate lag.");
        if let Some(ref mut client) = client.as_mut() {
            client.update();
        }
        thread::sleep(Duration::from_millis(10));
        //logger_buf.update();
        i += 1;
        if i > 20 {
            exit_flag.store(true, Ordering::Release);
        }
    }
    Ok(())
}
