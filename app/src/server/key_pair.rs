
use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::Document,
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use snafu::Snafu;

#[derive(Debug)]
pub(crate) struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    pk_der: Document,
}

#[derive(Debug, Snafu)]
pub(crate) enum KeyPairError {
    #[snafu(display("RSA error: {error}"))]
    RsaError { error: rsa::Error },
    #[snafu(display("Error: {message}"))]
    Other { message: String },
}

impl From<rsa::Error> for KeyPairError {
    fn from(value: rsa::Error) -> Self {
        Self::RsaError { error: value }
    }
}

impl KeyPair {
    pub(crate) fn new(bits: usize) -> Result<Self, KeyPairError> {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), bits)?;
        let public_key = RsaPublicKey::from(&private_key);
        let pk_der = public_key.to_pkcs1_der().map_err(|e| KeyPairError::Other {
            message: e.to_string(),
        })?;
        Ok(KeyPair {
            private_key,
            public_key,
            pk_der,
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

    pub(crate) fn public_key_bytes(&self) -> &[u8] {
        self.pk_der.as_bytes()
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
