use std::{net::AddrParseError, sync::PoisonError};

use log::SetLoggerError;
use rg_common::{commands::CmdError, VarRegistryError};
use rg_net::ProtocolError;
use snafu::Snafu;
use winit::error::EventLoopError;

#[derive(Debug, Snafu)]
pub enum AppError {
    #[snafu(display("{e}"))]
    ProtocolError { e: ProtocolError },
    #[snafu(display("Lock poisoned"))]
    PoisonError,
    #[snafu(display("RSA error: {cause:?}"))]
    RsaError { cause: rsa::Error },
    #[snafu(display("RSA PKSC1 error: {cause:?}"))]
    RsaPksc1Error { cause: rsa::pkcs1::Error },
    #[snafu(display("I/O error {kind}"))]
    IoError { kind: std::io::ErrorKind },
    #[snafu(display("Address parsing error"))]
    AddrParseError,
    #[snafu(display("Illegal state: {message}"))]
    IllegalState { message: String },
    #[snafu(display("Command error: {cause}"))]
    CmdError { cause: CmdError },
    #[snafu(display("Event loop error: {error:?}"))]
    EventLoopError { error: EventLoopError },
    #[snafu(display("Variable regsitry error: {error:?}"))]
    VarRegistryError { error: VarRegistryError },
}

impl From<ProtocolError> for AppError {
    fn from(value: ProtocolError) -> Self {
        Self::ProtocolError { e: value }
    }
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

impl From<rsa::Error> for AppError {
    fn from(value: rsa::Error) -> Self {
        Self::RsaError { cause: value }
    }
}

impl From<rsa::pkcs1::Error> for AppError {
    fn from(value: rsa::pkcs1::Error) -> Self {
        Self::RsaPksc1Error { cause: value }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError { kind: e.kind() }
    }
}

impl From<AddrParseError> for AppError {
    fn from(_: AddrParseError) -> Self {
        Self::AddrParseError
    }
}

impl From<SetLoggerError> for AppError {
    fn from(value: SetLoggerError) -> Self {
        Self::IllegalState {
            message: value.to_string(),
        }
    }
}

impl From<CmdError> for AppError {
    fn from(value: CmdError) -> Self {
        AppError::CmdError { cause: value }
    }
}

impl From<EventLoopError> for AppError {
    fn from(value: EventLoopError) -> Self {
        AppError::EventLoopError { error: value }
    }
}

impl From<VarRegistryError> for AppError {
    fn from(value: VarRegistryError) -> Self {
        AppError::VarRegistryError { error: value }
    }
}
