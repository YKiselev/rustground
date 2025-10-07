use std::{borrow::Cow, net::AddrParseError, sync::PoisonError};

use log::SetLoggerError;
use log4rs::config::runtime::ConfigErrors;
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
    #[error(transparent)]
    RsaError(#[from] rsa::Error),
    #[error(transparent)]
    RsaPksc1Error(#[from] rsa::pkcs1::Error),
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
    #[error(transparent)]
    LogError(#[from] ConfigErrors)
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

impl From<SetLoggerError> for AppError {
    fn from(value: SetLoggerError) -> Self {
        Self::IllegalState(Cow::Owned(value.to_string()))
    }
}
