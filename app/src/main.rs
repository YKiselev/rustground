extern crate core;

use error::AppError;

use rg_common::Arguments;

mod app;
mod app_logger;
mod application;
mod client;
mod error;
mod net;
mod server;

fn main() -> Result<(), AppError> {
    let args = Arguments::parse();
    if args.dedicated() {
        todo!("Not implemented!");
    } else {
        application::run_client_server(args)
    }
}
