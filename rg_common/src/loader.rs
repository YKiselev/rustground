use std::{
    fmt::Display,
    io::{Error, Read},
};

use thiserror::Error;

pub trait Loader<A, R>: Fn(&mut R) -> Result<A, LoaderError>
where
    A: Send + Sync,
    R: Read,
{
}

impl<A, R, T> Loader<A, R> for T
where
    A: Send + Sync,
    R: Read,
    T: Fn(&mut R) -> Result<A, LoaderError>,
{
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Not found")]
    NotFound,
    #[error("Not a valid utf-8 data")]
    BadUtf8,
    #[error("{0}")]
    Custom(String),
}

impl From<std::io::Error> for LoaderError {
    fn from(value: std::io::Error) -> Self {
        LoaderError::Custom(value.to_string())
    }
}