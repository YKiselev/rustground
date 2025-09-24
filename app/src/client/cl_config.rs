use rg_macros::VarBag;
use serde::{Deserialize, Serialize};

#[derive(Debug, VarBag, Serialize, Deserialize)]
pub(super) struct ClientConfig {
    name: String,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            name: "player".to_owned()
        }
    }
}