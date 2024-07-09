use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::EncodePublicKey;

#[derive(Debug)]
pub(crate) struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    public_bytes: Vec<u8>,
}

impl KeyPair {
    pub(crate) fn new(bits: usize) -> anyhow::Result<Self> {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), bits)?;
        let public_key = RsaPublicKey::from(&private_key);
        // let public_pem = public_key.to_public_key_pem(LineEnding::LF)
        //     .map_err(|e| anyhow::Error::from(e))?;
        let public_bytes = rmp_serde::to_vec(&public_key).expect("AAAAAAAAAAa");
        //let new_pk: RsaPublicKey = rmp_serde::from_read(&s_key[..]).expect("BBBBBBB");
        Ok(
            KeyPair {
                private_key,
                public_key,
                public_bytes,
            }
        )
    }

    pub(crate) fn public_key_as_pem(&self) -> RsaPublicKey {
        self.public_key.clone()
    }

    pub(crate) fn encode(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.public_key.encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data).map_err(|e| anyhow::Error::from(e))
    }

    pub(crate) fn decode(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.private_key.decrypt(Pkcs1v15Encrypt, data).map_err(|e| anyhow::Error::from(e))
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