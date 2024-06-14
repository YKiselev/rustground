use core::services::Services;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use log::{info, LevelFilter};
use core::arguments::Arguments;
use crate::client::Client;
use crate::server::Server;

mod client;
mod server;
mod app_logger;

fn main() {
    let logger = app_logger::init();

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
    while !exit_flag.load(Ordering::Acquire) {
        server.update();
        if let Some(client) = client.as_ref() {
            client.update();
        }
        std::thread::sleep(Duration::from_millis(10));
        exit_flag.store(true, Ordering::Release);
    }
}
