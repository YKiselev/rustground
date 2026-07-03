use rg_macros::VarBag;
use serde::{Deserialize, Serialize};

#[derive(Debug, VarBag, Serialize, Deserialize, Default)]
pub(super) struct ClientConfig {
    name: String,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            name: "player".to_owned(),
            ..Default::default()
        }
    }
}
