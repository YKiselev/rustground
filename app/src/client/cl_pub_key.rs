
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::error::AppError;

#[derive(Debug)]
pub(crate) struct PublicKey {
    key: RsaPublicKey,
}

impl PublicKey {
    pub(crate) fn from_der(bytes: &[u8]) -> Result<Self, AppError> {
        Ok(PublicKey {
            key: RsaPublicKey::from_pkcs1_der(bytes)?,
        })
    }

    pub(crate) fn encode(&self, data: &[u8]) -> Result<Vec<u8>, AppError> {
        Ok(self.key
            .encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data)?)
    }

    pub(crate) fn encode_str(&self, data: &str) -> Result<Vec<u8>, AppError> {
        self.encode(data.as_bytes())
    }
}
