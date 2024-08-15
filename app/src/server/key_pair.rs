use std::{error::Error, fmt::Display};

use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

use crate::error::AppError;

#[derive(Debug)]
pub(crate) struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

#[derive(Debug, Default)]
pub(crate) struct KeyPairError {
    pub message: String,
}

impl Error for KeyPairError {}

impl Display for KeyPairError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
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
