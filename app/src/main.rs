extern crate core;

use std::io::Read;
use std::ops::DerefMut;
use std::process::exit;
use std::result;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use log::{error, info, LevelFilter, warn};
use rsa::signature::digest::Digest;

use common::arguments::Arguments;
use common::files;

use crate::bit_code_test::test_bitcode;
use crate::client::Client;
use crate::config::{Config, ServerConfig};
use crate::net::Message;
use crate::server::{Server, server_init};

mod client;
mod server;
mod app_logger;
mod net;
mod config;
mod bit_code_test;
mod app;
mod app_state;
mod as_single;
mod as_multi;
mod as_dedicated;
mod as_init;

/*
fn run_client(args: &Arguments, cfg: &Config) -> anyhow::Result<()> {
    let (server, sv_handle) = server_init(&cfg.server)?;
    // debug
    let server_addr = server.read().unwrap().local_address()?;
    let mut client = Client::new(&args, server_addr);

    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let started_at = Instant::now();
    while !exit_flag.load(Ordering::Relaxed) {
        client.frame_start();

        client.update();

        thread::sleep(Duration::from_millis(5));
        //logger_buf.update();
        if started_at.elapsed() > Duration::from_secs(7) {
            exit_flag.store(true, Ordering::Release);
        }
        client.frame_end();
    }
    server.write().unwrap().shutdown();
    sv_handle.join().unwrap();

    Ok(())
}
*/
fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut app = app::App::new(&args);
    app.run()
    //let files = Arc::new(RwLock::new(files::AppFiles::new(&args)));
    //let cfg = Config::load("config.toml", files.write().unwrap().deref_mut());
    //info!("Loaded config: {cfg:?}");
}
