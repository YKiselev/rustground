use serde::{Deserialize, Serialize};

pub const MAX_DATAGRAM_SIZE: usize = 65507;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "id")]
pub enum Message {
    Connect { name: String, password: String },
    Accepted,
}

