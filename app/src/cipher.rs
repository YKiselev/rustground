use crate::error::{AppError, to_illegal_state};
use bytes::{BufMut, Bytes, BytesMut};
use chacha20poly1305::{
    AeadInOut, ChaCha20Poly1305, Key, KeyInit, Nonce,
    aead::{Aead, AeadInPlace, inout::InOutBuf},
};
use hkdf::Hkdf;
use rand::{TryRng, rngs::SysRng};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey};

pub struct Cipher {
    key: Key,
    cipher: ChaCha20Poly1305,
    buffer: BytesMut,
}

impl Cipher {
    pub fn new(secret: EphemeralSecret, public: &PublicKey) -> Result<Self, AppError> {
        let shared_secret = secret.diffie_hellman(&public);

        let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
        let mut key_bytes = [0u8; 32];
        hkdf.expand(b"rust ground protocol key context", &mut key_bytes)
            .map_err(|e| to_illegal_state(e.to_string()))?;

        let key = Key::try_from(key_bytes)?;
        let cipher = ChaCha20Poly1305::new(&key);
        let buffer = BytesMut::with_capacity(128 * 1024);

        Ok(Self {
            key,
            cipher,
            buffer,
        })
    }

    pub fn encode(&mut self, bytes: &Bytes) -> Result<Bytes, AppError> {
        let nonce = self.generate_nonce()?;
        let required = bytes.len() + 12 + 16;

        if self.buffer.capacity() < required {
            if !self.buffer.try_reclaim(required) {
                return Err(to_illegal_state("Cipher buffer overflow!"));
            }
        }

        self.buffer.put_slice(&nonce);
        self.buffer.resize(required - 16, 0);

        let out_slice = &mut self.buffer[12..12 + bytes.len()];
        let in_out_buffer =
            InOutBuf::new(bytes, out_slice).map_err(|_| to_illegal_state("Mismatched size!"))?;

        let tag = self
            .cipher
            .encrypt_inout_detached(&nonce, b"", in_out_buffer)
            .map_err(|_| to_illegal_state("Failed to encrypt!"))?;

        self.buffer.put_slice(&tag);

        let result = self.buffer.split();

        Ok(result.freeze())
    }

    pub fn encode_str<S>(&mut self, text: S) -> Result<Bytes, AppError>
    where
        S: AsRef<str>,
    {
        self.buffer.clear();
        self.buffer.extend_from_slice(text.as_ref().as_bytes());
        let buf = self.buffer.split();
        self.encode(&buf.freeze())
    }

    pub fn decode(&mut self, bytes: &Bytes) -> Result<BytesMut, AppError> {
        let nonce_bytes: [u8; 12] = bytes[0..12].try_into()?;
        let nonce = Nonce::try_from(nonce_bytes)?;

        let ciphertext_with_tag = &bytes[12..];
        let ciphertext_len = ciphertext_with_tag.len() - 16;
        let ciphertext = &ciphertext_with_tag[..ciphertext_len];

        let tag_bytes = &ciphertext_with_tag[ciphertext_len..];
        let tag = chacha20poly1305::Tag::try_from(tag_bytes)?;

        let output = &mut self.buffer;

        output.clear();

        if output.capacity() < ciphertext_len {
            if !output.try_reclaim(ciphertext_len) {
                return Err(to_illegal_state("Cipher buffer overflow!"));
            }
        }

        output.resize(ciphertext_len, 0);

        let in_out_buf = InOutBuf::new(ciphertext, &mut output[..])
            .map_err(|e| to_illegal_state(format!("{:?}", e)))?;

        self.cipher
            .decrypt_inout_detached(&nonce, b"", in_out_buf, &tag)
            .map_err(|_| to_illegal_state("Failed to decrypt!"))?;

        Ok(output.split())
    }

    fn generate_nonce(&self) -> Result<Nonce, AppError> {
        let mut nonce_bytes = [0u8; 12];

        let _ = SysRng
            .try_fill_bytes(&mut nonce_bytes)
            .map_err(|e| to_illegal_state(format!("{:?}", e)))?;

        let nonce = Nonce::try_from(nonce_bytes)?;
        Ok(nonce)
    }
}

#[cfg(test)]
mod test {

    use bytes::Bytes;
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
    use hkdf::Hkdf;
    use sha2::Sha256;
    use x25519_dalek::{EphemeralSecret, PublicKey};

    use super::*;

    #[test]
    fn gen_pair() {
        let alice_secret = EphemeralSecret::random();
        let alice_public = PublicKey::from(&alice_secret);

        let bob_secret = EphemeralSecret::random();
        let bob_public = PublicKey::from(&bob_secret);

        let alice_shared_secret = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_secret = bob_secret.diffie_hellman(&alice_public);

        assert_eq!(alice_shared_secret.as_bytes(), bob_shared_secret.as_bytes());

        let hkdf = Hkdf::<Sha256>::new(None, alice_shared_secret.as_bytes());
        let mut encryption_key_bytes = [0u8; 32];
        hkdf.expand(
            b"rust ground protocol key context",
            &mut encryption_key_bytes,
        )
        .unwrap();

        let key = Key::try_from(encryption_key_bytes).unwrap();
        let cipher = ChaCha20Poly1305::new(&key);

        // Nonce. Should be unique for each packet!
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..4].copy_from_slice(&105u32.to_be_bytes()); // Example: packet ID = 105
        let nonce = Nonce::try_from(nonce_bytes).unwrap();

        // Encoding
        let message = b"This is a totally secret message!";
        let ciphertext = cipher.encrypt(&nonce, message.as_ref()).unwrap();

        println!("Encoded bytes: {:?}\n", ciphertext);

        // Decoding
        let decrypted_bytes = cipher
            .decrypt(&nonce, ciphertext.as_slice())
            .expect("Failed!");

        let decrypted_message = String::from_utf8(decrypted_bytes).unwrap();
        println!("Decoded result: {}", decrypted_message);
    }

    #[test]
    fn test_cipher() {
        let alice_secret = EphemeralSecret::random();

        let bob_secret = EphemeralSecret::random();
        let bob_public = PublicKey::from(&bob_secret);

        let mut cipher = Cipher::new(alice_secret, &bob_public).unwrap();

        let s1 = b"First message";
        let s2 = b"Second message";
        let s3 = b"Message number three";

        let b1 = Bytes::from(&s1[..]);
        let b2 = Bytes::from(&s2[..]);
        let b3 = Bytes::from(&s3[..]);

        let e1 = cipher.encode(&b1).unwrap();
        let e2 = cipher.encode(&b2).unwrap();
        let e3 = cipher.encode(&b3).unwrap();

        let r1 = cipher.decode(&e1).unwrap();
        let r3 = cipher.decode(&e3).unwrap();
        let r2 = cipher.decode(&e2).unwrap();

        assert_eq!(s1, &r1[..]);
        assert_eq!(s2, &r2[..]);
        assert_eq!(s3, &r3[..]);
    }
}
