use crate::{error::AppError, server::key_pair::KeyPair};


#[derive(Debug)]
pub(super) struct ServerSecurity {
    pub keys: KeyPair,
    pub password: Option<String>,
}

impl ServerSecurity {
    pub(super) fn new(key_bits: usize, pwd: &Option<String>) -> Result<Self, AppError> {
        let keys = KeyPair::new(key_bits)?;
        Ok(Self {
            keys,
            password: pwd.to_owned(),
        })
    }

    pub(super) fn decode(&self, value: &[u8]) -> Result<Vec<u8>, AppError> {
        self.keys.decode(value)
    }

    pub(super) fn is_password_ok(&self, pwd: &[u8]) -> bool {
        if let Some(p) = self.password.as_ref() {
            p.as_bytes().eq(pwd)
        } else {
            pwd.is_empty()
        }
    }
}
