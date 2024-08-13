use std::io;

use log::SetLoggerError;
use log4rs::config::runtime::ConfigErrors;


#[derive(Debug)]
pub struct AppError {
    pub message: String
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