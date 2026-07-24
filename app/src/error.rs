use std::{
    array::TryFromSliceError, borrow::Cow, convert::Infallible, fmt::Debug, net::AddrParseError, sync::PoisonError,
};

use rg_common::{VarRegistryError, commands::CmdError};
use rg_net::ProtocolError;
use thiserror::Error;
use winit::error::EventLoopError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    ProtocolError(#[from] ProtocolError),
    #[error("Lock poisoned")]
    PoisonError,
    #[error("I/O error {0}")]
    IoError(std::io::ErrorKind),
    #[error("Address parsing error")]
    AddrParseError,
    #[error("Illegal state: {0}")]
    IllegalState(Cow<'static, str>),
    #[error(transparent)]
    CmdError(#[from] CmdError),
    #[error("Event loop error: {0:?}")]
    EventLoopError(#[from] EventLoopError),
    #[error(transparent)]
    VarRegistryError(#[from] VarRegistryError),
    #[error("Log error: {0}")]
    LogError(String),
    #[error("Async runtime error: {0}")]
    AsyncError(String),
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error(transparent)]
    SliceError(#[from] TryFromSliceError),
    #[error(transparent)]
    Infallible(#[from] Infallible)
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
    }
}

impl From<AddrParseError> for AppError {
    fn from(_: AddrParseError) -> Self {
        Self::AddrParseError
    }
}

pub fn to_illegal_state<S>(msg: S) -> AppError
where
    S: Into<Cow<'static, str>>,
{
    AppError::IllegalState(msg.into())
}
