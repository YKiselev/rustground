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

    let cfg = Config::load("config.toml", &mut files);
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
    let mut time = Instant::now();
    let mut lag = 0u128;
    const MILLIS_PER_UPDATE: u128 = 10;
    let started_at = Instant::now();
    while !exit_flag.load(Ordering::Acquire) {
        let delta = time.elapsed();
        time = Instant::now();
        lag += delta.as_millis();
        //info!("lag={lag} ns., delta={delta} ns.");
        let mut m = 0;
        while lag >= MILLIS_PER_UPDATE {
            server.update()?;
            lag -= MILLIS_PER_UPDATE;
            m += 1;
        }
        //info!("Updated {m} times to eliminate lag.");
        if let Some(ref mut client) = client.as_mut() {
            client.update();
        }
        thread::sleep(Duration::from_millis(10));
        //logger_buf.update();
        if started_at.elapsed() > Duration::from_secs(10) {
            exit_flag.store(true, Ordering::Release);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::Write;

    #[test]
    fn arrays() {
        let mut v1: Vec<u8> = Vec::with_capacity(32);
        //let r1 = &mut v1[..];
        let r2: &mut dyn Write = &mut v1;
        //let mut r2 = Cursor::new(v1);
        //r1[0] = 23;
        let buf = [1u8, 2, 3];
        //let r = r2.write(&buf).expect("Failed!");
        let r = r2.write(&buf).expect("Failed!");
        //println!("array: {v1:?}");
        let buf = [4u8, 5, 6];
        //let r = r2.write(&buf).expect("Failed!");
        let r = r2.write(&buf).expect("Failed!");
        println!("array: {:?}", v1);
        //let buf2 = r3.into_inner();

        let r3: &mut dyn Write = &mut v1;
        let buf = [7u8, 8, 9, 10];
        let r = r3.write(&buf).expect("Failed!");

        println!("array: {:?}", v1);
    }
}