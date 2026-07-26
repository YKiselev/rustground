use std::collections::HashMap;

use rg_macros::VarBag;
use serde::{Deserialize, Serialize};

#[derive(Debug, VarBag, Serialize, Deserialize, Default)]
pub(super) struct ClientConfig {
    pub name: String,
    pub bindings: HashMap<String, String>
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            name: "player".to_owned(),
            ..Default::default()
        }
    }
}
