use std::{error::Error, fmt::Display, io};

use log::SetLoggerError;
use log4rs::config::runtime::ConfigErrors;


#[derive(Debug)]
pub struct AppError {
    pub message: String
}

impl std::error::Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}


impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        AppError {
            message: value.to_owned()
        }
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        AppError { 
            message: value.to_string() 
        }
    }
}

impl From<ConfigErrors> for AppError {
    fn from(value: ConfigErrors) -> Self {
        AppError{
            message: value.to_string()
        }
    }
}

impl From<SetLoggerError> for AppError {
    fn from(value: SetLoggerError) -> Self {
        AppError { 
            message:  value.to_string()
        }
    }
}
    