use std::borrow::Cow;

use error::AppError;

use log::warn;
use rg_common::Arguments;

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
        application::run_client_server(args)
    }
}

fn get_arguments() -> Result<Arguments, AppError> {
    let mut args: Vec<String> = std::env::args().collect();
    if let Ok(app_args) = std::env::var("APP_ARGS") {
        let app_args = app_args.split_whitespace().map(String::from);
        args.extend(app_args);
    }

    let program_name = args.get(0).map(|s| s.as_str()).unwrap();
    let args_strs: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    let args = argh::FromArgs::from_args(&[program_name], &args_strs)
        .map_err(|e| AppError::IllegalState(Cow::from(e.output)))?;
    Ok(args)
}
