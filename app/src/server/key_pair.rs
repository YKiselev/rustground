use std::{error::Error, fmt::Display};

use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use snafu::Snafu;

use crate::error::AppError;

#[derive(Debug)]
pub(crate) struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

#[derive(Debug, Snafu)]
#[snafu(display("Key pair error: {message}"))]
pub(crate) struct KeyPairError {
    pub message: String,
}

impl KeyPairError {
    pub(crate) fn new<S: AsRef<str>>(message: S) -> Self {
        Self {
            message: message.as_ref().to_owned(),
        }
    }
}

impl From<rsa::Error> for KeyPairError {
    fn from(value: rsa::Error) -> Self {
        KeyPairError {
            message: value.to_string(),
        }
    }
}

impl KeyPair {
    pub(crate) fn new(bits: usize) -> Result<Self, AppError> {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), bits)
            .map_err(|e| AppError::from("Unable to generate key!"))?;
        let public_key = RsaPublicKey::from(&private_key);
        Ok(KeyPair {
            private_key,
            public_key,
        })
    }

    pub(crate) fn public_key(&self) -> &RsaPublicKey {
        &self.public_key
    }

    pub(crate) fn encode(&self, data: &[u8]) -> Result<Vec<u8>, KeyPairError> {
        Ok(self
            .public_key
            .encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data)?)
    }

    pub(crate) fn decode(&self, data: &[u8]) -> Result<Vec<u8>, KeyPairError> {
        Ok(self.private_key.decrypt(Pkcs1v15Encrypt, data)?)
    }
}

#[cfg(test)]
mod test {
    use crate::server::key_pair::KeyPair;

    #[test]
    fn gen_pair() {
        let keys = KeyPair::new(512).expect("Failed to generate key pair!");
        let data = b"hello world";

        let encoded = keys.encode(&data[..]).expect("Unable to encode!");
        assert_ne!(&data[..], &encoded[..]);

        let decoded = keys.decode(&encoded).expect("Unable to decode!");
        assert_eq!(&data[..], &decoded[..]);
    }
}
