use crate::error::AppError;

pub(super) struct ServerSecurity {
    pub password: Option<String>,
}

impl ServerSecurity {
    pub(super) fn new(password: Option<String>) -> Result<Self, AppError> {
        Ok(Self { password })
    }

    pub(super) fn is_password_ok(&self, pwd: &[u8]) -> bool {
        if let Some(p) = self.password.as_ref() {
            p.as_bytes().eq(pwd)
        } else {
            true
        }
    }
}
