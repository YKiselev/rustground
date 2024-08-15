use std::error::Error;
use std::fmt::Display;

use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::error::AppError;

#[derive(Debug)]
pub(crate) struct PublicKey {
    public_key: RsaPublicKey,
}

#[derive(Debug)]
pub(crate) struct PublicKeyError {
    pub message: String,
}

impl Error for PublicKeyError {}

impl Display for PublicKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<rsa::Error> for PublicKeyError {
    fn from(value: rsa::Error) -> Self {
        PublicKeyError {
            message: value.to_string(),
        }
    }
}

impl PublicKey {
    pub(crate) fn from_pem(data: &str) -> Result<Self, AppError> {
        Ok(PublicKey {
            public_key: RsaPublicKey::from_public_key_pem(data)
                .map_err(|e| AppError::from("Unable to reconstruct public key!"))?,
        })
    }

    pub(crate) fn new(key: RsaPublicKey) -> Self {
        PublicKey { public_key: key }
    }

    pub(crate) fn encode(&self, data: &[u8]) -> Result<Vec<u8>, PublicKeyError> {
        Ok(self
            .public_key
            .encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data)?)
    }

    pub(crate) fn encode_str(&self, data: &str) -> Result<Vec<u8>, PublicKeyError> {
        Ok(self.encode(data.as_bytes())?)
    }
}
