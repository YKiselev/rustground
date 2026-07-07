use std::io::Read;

use serde::Deserialize;
use thiserror::Error;

use crate::files::SeekAndRead;

pub trait Loader<A, Ctx>
where
    A: Send + Sync,
{
    fn load(
        &self,
        reader: &mut std::io::BufReader<SeekAndRead>,
        ctx: Ctx,
    ) -> Result<A, LoaderError>;
}

impl<A, Ctx, T> Loader<A, Ctx> for T
where
    A: Send + Sync,
    T: Fn(&mut std::io::BufReader<SeekAndRead>, Ctx) -> Result<A, LoaderError>,
{
    fn load(
        &self,
        reader: &mut std::io::BufReader<SeekAndRead>,
        ctx: Ctx,
    ) -> Result<A, LoaderError> {
        (self)(reader, ctx)
    }
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Not found: {0:?}")]
    NotFound(String),
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

pub fn load_bytes(
    reader: &mut std::io::BufReader<SeekAndRead>,
    _: (),
) -> Result<Vec<u8>, LoaderError> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn load_deserializable<T>(
    reader: &mut std::io::BufReader<SeekAndRead>,
    ctx: (),
) -> Result<T, LoaderError>
where
    T: for<'a> Deserialize<'a>,
{
    let buf = load_bytes(reader, ctx)?;
    toml::from_slice(&buf).map_err(|e| LoaderError::Custom(e.to_string()))
}
