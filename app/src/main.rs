extern crate core;


use log::info;
use rsa::signature::digest::Digest;

use rg_common::arguments::Arguments;


mod client;
mod server;
mod app_logger;
mod net;
mod bit_code_test;
mod app;
mod state;

fn main() -> anyhow::Result<()> {
    let logger_buf = app_logger::init().expect("Unable to init app logger!");
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut app = app::App::new(&args);
    app.run()
}
