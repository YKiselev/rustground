
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

use crate::error::{to_app_error, AppError};

#[derive(Debug)]
pub(crate) struct PublicKey {
    key: RsaPublicKey,
}

impl PublicKey {
    pub(crate) fn from_pem(data: &str) -> Result<Self, AppError> {
        Ok(PublicKey {
            key: RsaPublicKey::from_public_key_pem(data).map_err(to_app_error)?,
        })
    }

    pub(crate) fn from_der(bytes: &[u8]) -> Result<Self, AppError> {
        Ok(PublicKey {
            key: RsaPublicKey::from_public_key_der(bytes).map_err(to_app_error)?,
        })
    }

    pub(crate) fn encode(&self, data: &[u8]) -> Result<Vec<u8>, AppError> {
        self.key
            .encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data)
            .map_err(|e| AppError::GenericError {
                message: e.to_string(),
            })
    }

    pub(crate) fn encode_str(&self, data: &str) -> Result<Vec<u8>, AppError> {
        self.encode(data.as_bytes())
    }
}
