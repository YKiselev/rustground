
use thiserror::Error;

use crate::files::SeekAndRead;

pub trait Loader<A>: Fn(&mut std::io::BufReader<SeekAndRead>) -> Result<A, LoaderError>
where
    A: Send + Sync,
{
}

impl<A, T> Loader<A> for T
where
    A: Send + Sync,
    T: Fn(&mut std::io::BufReader<SeekAndRead>) -> Result<A, LoaderError>,
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
