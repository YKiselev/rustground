use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use rsa::pkcs8::DecodePublicKey;

#[derive(Debug)]
pub(crate) struct PublicKey {
    public_key: RsaPublicKey,
}

impl PublicKey {
    pub(crate) fn from_pem(data: &str) -> anyhow::Result<Self> {
        Ok(
            PublicKey {
                public_key: RsaPublicKey::from_public_key_pem(data)?
            }
        )
    }

    pub(crate) fn new(key: RsaPublicKey) -> Self {
        PublicKey {
            public_key: key
        }
    }

    pub(crate) fn encode(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.public_key.encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data).map_err(|e| anyhow::Error::from(e))
    }

    pub(crate) fn encode_str(&self, data: &str) -> anyhow::Result<Vec<u8>> {
        self.encode(data.as_bytes())
    }
}
