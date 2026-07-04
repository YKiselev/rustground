
use error::AppError;

use rg_common::Arguments;

mod app_logger;
mod application;               
mod client;
mod error;
mod server;

fn main() -> Result<(), AppError> {
    let args: Arguments = argh::from_env();
    let dedicated = args.dedicated.unwrap_or(false);
    if dedicated {
        todo!("Not implemented!");
    } else {
        application::run_client_server(args)
    }
}
