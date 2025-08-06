extern crate core;

use error::AppError;

use rg_common::Arguments;

mod app_logger;
mod application;
mod client;
mod error;
mod server;
mod fps;

fn main() -> Result<(), AppError> {
    let args: Arguments = argh::from_env();
    if args.dedicated() {
        todo!("Not implemented!");
    } else {
        application::run_client_server(args)
    }
}
