use error::AppError;

use crate::application::{args::get_arguments, client_server::run_client_server};

mod app_logger;
mod application;
mod client;
mod error;
mod server;

fn main() -> Result<(), AppError> {
    let args = get_arguments()?;
    let dedicated = args.dedicated.unwrap_or(false);
    if dedicated {
        todo!("Not implemented!");
    } else {
        run_client_server(args)
    }
}
