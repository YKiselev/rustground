use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use log::{info, LevelFilter};

use core::arguments::Arguments;
use core::services::Services;

use crate::client::Client;
use crate::server::Server;

mod client;
mod server;
mod app_logger;
mod net;

fn main() {
    let logger_buf = app_logger::init().unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();

    // init services
    let services = Services::new(&args);

    // server
    let mut server = Server::new(&args);

    // debug
    let server_addr = server.local_address().expect("Unable to get server address");

    // client
    let mut client = if !args.dedicated() {
        Some(Client::new(&args, server_addr))
    } else {
        None
    };

    // serde test

    // main loop
    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let mut i = 0;
    while !exit_flag.load(Ordering::Acquire) {
        server.update();
        if let Some(ref mut client) = client.as_mut() {
            client.update();
        }
        std::thread::sleep(Duration::from_millis(10));
        logger_buf.update();
        i += 1;
        if i > 100 {
            exit_flag.store(true, Ordering::Release);
        }
    }
}
