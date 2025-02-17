use std::{net::AddrParseError, sync::PoisonError};

use snafu::Snafu;

use super::key_pair::KeyPairError;

#[derive(Debug, Snafu)]
pub(crate) struct ServerError(InnerError);

#[derive(Debug, Snafu)]
enum InnerError {
    #[snafu(display("I/O error {kind}"))]
    IoError { kind: std::io::ErrorKind },
    #[snafu(display("Lock poisoned"))]
    PoisonError,
    #[snafu(display("Address parsing error"))]
    AddrParseError,
    #[snafu(display("Key pair error: {error}"))]
    KeyPairError { error: KeyPairError },
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError(InnerError::IoError { kind: e.kind() })
    }
}

impl<T> From<PoisonError<T>> for ServerError {
    fn from(_: PoisonError<T>) -> Self {
        Self(InnerError::PoisonError)
    }
}

impl From<AddrParseError> for ServerError {
    fn from(_: AddrParseError) -> Self {
        Self(InnerError::AddrParseError)
    }
}

impl From<KeyPairError> for ServerError {
    fn from(value: KeyPairError) -> Self {
        Self(InnerError::KeyPairError { error: value })
    }
}
