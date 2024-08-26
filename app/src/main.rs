extern crate core;

use error::AppError;
use log::info;

use rg_common::arguments::Arguments;

mod app;
mod app_logger;
mod client;
mod error;
mod net;
mod server;
mod state;

fn main() -> Result<(), AppError> {
    let args = Arguments::parse();
    let (handle, log_buf) = app_logger::init().expect("Unable to init app logger!");
    info!("Begin initialization...");

    let mut app = app::App::new(args, handle, log_buf);
    app.run()
}
