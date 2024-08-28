extern crate core;

use error::AppError;
use log::info;

use rg_common::Arguments;

mod app;
mod app_logger;
mod client;
mod error;
mod net;
mod server;
mod state;
mod application;

fn main() -> Result<(), AppError> {
    let args = Arguments::parse();
    if args.dedicated() {
        todo!("Not implemented!");
    } else {
        application::run_client_server(args)
    }
}
