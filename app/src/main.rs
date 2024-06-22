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

fn main() {
    let logger_buf = app_logger::init().unwrap();
    //log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    info!("Begin initialization...");

    let args = Arguments::parse();

    // init services
    let services = Services::new(&args);

    // server
    let server = Server::new(&args);

    // client
    let client = if !args.dedicated() {
        Some(Client::new(&args))
    } else {
        None
    };

    // main loop
    info!("Entering main loop...");
    let exit_flag = AtomicBool::new(false);
    let mut i = 0;
    while !exit_flag.load(Ordering::Acquire) {
        server.update();
        if let Some(client) = client.as_ref() {
            client.update();
        }
        std::thread::sleep(Duration::from_millis(10));
        logger_buf.update();
        i += 1;
        if i > 10 {
            exit_flag.store(true, Ordering::Release);
        }
    }
}
