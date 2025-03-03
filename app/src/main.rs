extern crate core;

use argh::FromArgs;
use error::AppError;

use rg_common::Arguments;

mod app;
mod app_logger;
mod application;
mod client;
mod error;
mod net;
mod server;
mod net2;

fn main() -> Result<(), AppError> {
    let args: Arguments = argh::from_env();
    if args.dedicated() {
        todo!("Not implemented!");
    } else {
        application::run_client_server(args)
    }
}
