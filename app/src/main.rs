extern crate core;


use error::AppError;
use log::info;

use rg_common::arguments::Arguments;


mod client;
mod server;
mod app_logger;
mod net;
mod bit_code_test;
mod app;
mod state;
mod error;

fn main() -> Result<(), AppError> {
    let logger_buf = app_logger::init().expect("Unable to init app logger!");
    info!("Begin initialization...");

    let args = Arguments::parse();
    let mut app = app::App::new(&args);
    app.run()
}
