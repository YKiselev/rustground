
use std::io::ErrorKind;

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum AppError {
    #[snafu(display("Error: {message}"))]
    GenericError{ message: String }
}

pub(crate) fn to_app_error<E>(e: E) -> AppError
where
    E: ToString,
{
    AppError::GenericError {
        message: e.to_string(),
    }
}
